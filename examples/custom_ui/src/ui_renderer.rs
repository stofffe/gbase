use crate::ui_layout::{Glyph, TextLayoutResult, TextSizeResult, UIElement};
use core::f32;
use gbase::{
    asset::{
        AssetCache, AssetConverter, AssetHandle, AssetLoader, ConvertAssetResult,
        ConvertAssetStatus, DerivedAsset, EmptyError, GetAssetResult, ShaderGpuConverter,
        ShaderLoader,
    },
    bytemuck, filesystem,
    glam::{self, Mat4},
    render::{self, BindGroupBindable},
    tracing, wgpu, Context,
};
use std::{collections::HashMap, path::PathBuf};

pub struct UIRenderer {
    shader_handle: AssetHandle<render::Shader>,
    bindgroup_layout: render::ArcBindGroupLayout,
    pipeline_layout: render::ArcPipelineLayout,

    instance_buffer: render::RawBuffer<UIElementInstace>,

    projection: render::UniformBuffer<glam::Mat4>,

    font: AssetHandle<Font>,
    font_atlas_raster_size: f32,
    font_atlas_supported_chars: Vec<char>,
}

impl UIRenderer {
    pub fn new(
        ctx: &mut gbase::Context,
        cache: &mut AssetCache,
        font_path: impl Into<PathBuf>,
        font_atlas_raster_size: f32,
        max_elements: u64,
    ) -> Self {
        let font = cache
            .load_builder(
                font_path,
                FontLoader {
                    settings: fontdue::FontSettings::default(),
                },
            )
            // TODO: dont actually want this but needed for manual reloading for now
            // .watch(true)
            .build(ctx, cache);

        //
        // gpu resources
        //

        let shader_handle = cache
            .load_builder("assets/shaders/ui.wgsl", ShaderLoader {})
            .watch(true)
            .build(ctx, cache);

        let bindgroup_layout = render::BindGroupLayoutBuilder::new()
            .entries(vec![
                // projection
                render::BindGroupLayoutEntry::new().uniform().vertex(),
                // atlas texture
                render::BindGroupLayoutEntry::new()
                    .texture_float_filterable()
                    .vertex()
                    .fragment(),
                // atlas sampler
                render::BindGroupLayoutEntry::new()
                    .sampler_filtering()
                    .fragment(),
            ])
            .build(ctx);

        let pipeline_layout = render::PipelineLayoutBuilder::new()
            .bind_groups(vec![bindgroup_layout.clone()])
            .build(ctx);
        let instance_buffer = render::RawBufferBuilder::new(max_elements).build(ctx);

        let projection = render::UniformBufferBuilder::new()
            .usage(wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST)
            .build(ctx);

        let mut font_atlas_supported_chars = Vec::new();
        for char in 'a'..='z' {
            font_atlas_supported_chars.push(char);
        }
        for char in 'A'..='Z' {
            font_atlas_supported_chars.push(char);
        }
        for char in '0'..='9' {
            font_atlas_supported_chars.push(char);
        }
        for char in " \t\n,.;/".chars() {
            font_atlas_supported_chars.push(char);
        }

        Self {
            pipeline_layout,
            bindgroup_layout,
            shader_handle,
            instance_buffer,
            projection,

            font,
            font_atlas_raster_size,
            font_atlas_supported_chars,
        }
    }

    pub fn render(
        &mut self,
        ctx: &mut gbase::Context,
        cache: &mut AssetCache,
        view: &wgpu::TextureView,
        view_format: wgpu::TextureFormat,
        ui_elements: &[UIElement],
    ) {
        let ConvertAssetResult::Success(shader) =
            self.shader_handle.convert(ctx, cache, ShaderGpuConverter)
        else {
            return;
        };
        let ConvertAssetResult::Success(font_atlas) = self.font.convert(
            ctx,
            cache,
            FontAtlasConverter {
                supported_chars: &self.font_atlas_supported_chars,
                font_raster_size: self.font_atlas_raster_size,
            },
        ) else {
            return;
        };

        //
        // convert
        //
        let mut instances = Vec::new();
        for elem in 0..ui_elements.len() {
            let element = &ui_elements[elem];

            // container
            instances.push(UIElementInstace {
                element_type: ELEMENT_TYPE_CONTAINER,

                position: [element.x, element.y],
                size: [element.preferred_width, element.preferred_height],
                color: element.background_color.to_array(),
                font_atlas_offset: [0.0, 0.0],
                font_atlas_size: [0.0, 0.0],
            });

            // glyphs
            let atlas_width = font_atlas.texture.width();
            let atlas_height = font_atlas.texture.height();
            if !element.text_info.text.is_empty() {
                for glyph in element.text_layout.glyphs.iter() {
                    // TODO: backup
                    let glyph_info = font_atlas
                        .lookup
                        .get(&glyph.character)
                        .expect("could not find glyph");

                    let x = element.x + glyph.x;
                    let y = element.y + glyph.y;
                    instances.push(UIElementInstace {
                        element_type: ELEMENT_TYPE_GLYPH,

                        position: [x, y],
                        size: [glyph.width, glyph.height],
                        color: element.text_info.text_color.to_array(),
                        font_atlas_offset: [
                            glyph_info.atlas_offset_x as f32 / atlas_width as f32,
                            glyph_info.atlas_offset_y as f32 / atlas_height as f32,
                        ],
                        font_atlas_size: [
                            glyph_info.atlas_width as f32 / atlas_width as f32,
                            glyph_info.atlas_height as f32 / atlas_height as f32,
                        ],
                    });
                }
            }
        }

        self.instance_buffer.write(ctx, &instances);
        let screen_size = render::surface_size(ctx);
        self.projection.write(
            ctx,
            &Mat4::orthographic_rh(
                0.0,
                screen_size.width as f32,
                screen_size.height as f32,
                0.0,
                0.0,
                1.0,
            ),
        );

        let atlas_view = render::TextureViewBuilder::new(font_atlas.texture.clone()).build(ctx);
        let atlas_sampler = render::SamplerBuilder::new().build(ctx);
        let bindgroup = render::BindGroupBuilder::new(self.bindgroup_layout.clone())
            .entries(vec![
                // camera projection
                self.projection.bindgroup_entry(),
                // atlas texture
                render::BindGroupEntry::Texture(atlas_view),
                // atlas sampler
                render::BindGroupEntry::Sampler(atlas_sampler),
            ])
            .build(ctx);

        let pipeline = render::RenderPipelineBuilder::new(shader, self.pipeline_layout.clone())
            .buffers(vec![UIElementInstace::desc()])
            .topology(wgpu::PrimitiveTopology::TriangleStrip)
            .single_target(
                render::ColorTargetState::new()
                    .format(view_format)
                    .blend(wgpu::BlendState::ALPHA_BLENDING),
            )
            .build(ctx);

        render::RenderPassBuilder::new()
            .color_attachments(&[Some(render::RenderPassColorAttachment::new(view))])
            .build_run_submit(ctx, |mut pass| {
                pass.set_pipeline(&pipeline);
                pass.set_vertex_buffer(0, self.instance_buffer.slice(..));
                pass.set_bind_group(0, Some(bindgroup.as_ref()), &[]);

                pass.draw(0..4, 0..instances.len() as u32);
            });
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn hot_reload(
        &mut self,
        ctx: &mut gbase::Context,
        cache: &mut AssetCache,
        font_path: impl Into<PathBuf>,
    ) {
        // clear handle to not use old data
        cache.clear_handle(self.font.clone());
        self.font = cache
            .load_builder(
                font_path,
                FontLoader {
                    settings: fontdue::FontSettings::default(),
                },
            )
            .build(ctx, cache);
    }
}

impl UIRenderer {
    pub fn calculate_preferred_text_size(
        &mut self,
        _ctx: &mut Context,
        cache: &mut AssetCache,
        text: &str,
        font_size: u32,
        wrap_on_newline: bool,
    ) -> TextSizeResult {
        let GetAssetResult::Success(font) = self.font.get(cache) else {
            return TextSizeResult {
                preferred_width: 0.0,
                preferred_height: 0.0,
                min_width: 0.0,
                min_height: 0.0,
            };
        };

        let line_metrics = font
            .font
            .horizontal_line_metrics(font_size as f32)
            .expect("could not get line metrics");
        let line_height = line_metrics.new_line_size;

        let mut x_offset = 0.0f32;
        let mut y_offset = 0.0f32;
        let mut longest_line = 0.0f32;
        let mut shortest_word = f32::MAX;

        // width
        let mut current_word_width = 0.0;
        let mut prev_char = None;
        for letter in text.chars() {
            let font_metrics = font.font.metrics(letter, font_size as f32);

            // TODO: might be a bit too long due to using advance and not width
            if wrap_on_newline && letter == '\n' {
                longest_line = longest_line.max(x_offset);
                x_offset = 0.0;
                y_offset += line_height;
                continue;
            }

            if letter.is_whitespace() {
                shortest_word = shortest_word.min(current_word_width);
                current_word_width = 0.0;
            }

            if let Some(prev) = prev_char {
                if let Some(kern) = font.font.horizontal_kern(prev, letter, font_size as f32) {
                    x_offset += kern;
                }
            }

            current_word_width += font_metrics.advance_width;
            x_offset += font_metrics.advance_width;

            prev_char = Some(letter);
        }

        let preferred_width = longest_line.max(x_offset);
        let preferred_height = y_offset + line_height;
        let min_width = shortest_word;
        // TODO: needed?
        let min_height = preferred_height;

        TextSizeResult {
            preferred_width,
            preferred_height,
            min_width,
            min_height,
        }
    }

    pub fn layout_text(
        &mut self,
        ctx: &mut Context,
        cache: &mut AssetCache,
        text: &str,
        font_size: u32,
        max_width: f32,
    ) -> TextLayoutResult {
        let GetAssetResult::Success(font) = self.font.get(cache) else {
            return TextLayoutResult {
                width: 0.0,
                height: 0.0,
                glyphs: Vec::new(),
            };
        };
        // TODO: bad?
        let font = font.clone();

        let ConvertAssetResult::Success(font_atlas) = self.font.convert(
            ctx,
            cache,
            FontAtlasConverter {
                supported_chars: &self.font_atlas_supported_chars,
                font_raster_size: self.font_atlas_raster_size,
            },
        ) else {
            return TextLayoutResult {
                width: 0.0,
                height: 0.0,
                glyphs: Vec::new(),
            };
        };

        let line_metrics = font
            .font
            .horizontal_line_metrics(font_size as f32)
            .expect("could not get line metrics");
        let line_height = line_metrics.new_line_size;

        let text = text.chars().collect::<Vec<_>>();
        let mut glyphs = Vec::new();

        let mut x_offset = 0.0f32;
        let mut y_offset = 0.0f32;
        let mut longest_line_width = 0.0f32;

        let mut current_word_start = 0;
        let mut current_word_len = 0;
        let mut current_word_width = 0.0;

        let mut prev_char = None;
        for text_index in 0..text.len() {
            let letter = text[text_index];
            let font_metrics = font.font.metrics(letter, font_size as f32);

            // add letter if not whitespace
            let is_last_element = text_index == text.len() - 1;
            let is_whitespace = letter == ' ' || letter == '\t' || letter == '\n';

            if !is_whitespace {
                if let Some(prev) = prev_char {
                    if let Some(kern) = font.font.horizontal_kern(prev, letter, font_size as f32) {
                        current_word_width += kern;
                    }
                }
                current_word_width += font_metrics.advance_width;
                current_word_len += 1;
                prev_char = Some(letter);
            }

            if is_whitespace || is_last_element {
                if current_word_len > 0 {
                    // wrap check
                    if x_offset + current_word_width > max_width {
                        y_offset += line_height;
                        longest_line_width = longest_line_width.max(x_offset);
                        x_offset = 0.0;
                    }

                    let mut prev_char = None;
                    for text_index in current_word_start..current_word_start + current_word_len {
                        let text_char = text[text_index];
                        let glyph_metrics = font.font.metrics(text_char, font_size as f32);
                        let glyph_info = font_atlas.lookup.get(&text_char).unwrap();
                        let scale = font_size as f32 / self.font_atlas_raster_size;
                        glyphs.push(Glyph {
                            character: text_char,
                            x: x_offset,
                            y: y_offset + line_height
                                - glyph_metrics.height as f32
                                - glyph_metrics.ymin as f32
                                + line_metrics.descent,
                            width: glyph_info.atlas_width as f32 * scale,
                            height: glyph_info.atlas_height as f32 * scale,
                        });

                        if let Some(prev) = prev_char {
                            if let Some(kern) =
                                font.font.horizontal_kern(prev, letter, font_size as f32)
                            {
                                x_offset += kern;
                            }
                        }
                        prev_char = Some(text[text_index]);
                        x_offset += glyph_metrics.advance_width;
                    }
                }

                // TODO: how to handle tab

                if letter == ' ' {
                    let glyph_info = font_atlas.lookup.get(&letter).unwrap();
                    let scale = font_size as f32 / self.font_atlas_raster_size;
                    glyphs.push(Glyph {
                        character: letter,
                        x: x_offset,
                        y: y_offset + line_height
                            - font_metrics.height as f32
                            - font_metrics.ymin as f32
                            + line_metrics.descent,
                        width: glyph_info.atlas_width as f32 * scale,
                        height: glyph_info.atlas_height as f32 * scale,
                    });
                    x_offset += font_metrics.advance_width;
                }
                // TODO: should maybe be a setting
                if letter == '\n' {
                    y_offset += line_height;
                    longest_line_width = longest_line_width.max(x_offset);
                    x_offset = 0.0;
                }

                // reset state
                current_word_start = text_index + 1;
                current_word_len = 0;
                current_word_width = 0.0;
            }
        }

        let width = longest_line_width.max(x_offset);
        let height = y_offset + line_height;

        TextLayoutResult {
            width,
            height,
            glyphs,
        }
    }
}

#[derive(Clone)]
struct AtlasGlyphInfo {
    atlas_offset_x: usize,
    atlas_offset_y: usize,
    atlas_width: usize,
    atlas_height: usize,
}

fn create_font_atlas(
    ctx: &mut gbase::Context,
    font: &fontdue::Font,
    supported_chars: &[char],
    font_raster_size: f32,
) -> (HashMap<char, AtlasGlyphInfo>, render::ArcTexture) {
    let mut glyphs = Vec::new();
    let mut total_area = 0;

    let sdf_params = sdfer::esdt::Params {
        pad: 8,
        radius: 8.0,
        cutoff: 0.5,
        solidify: true,
        preprocess: false,
    };

    // create sdf glyphs
    for &char in supported_chars {
        let (font_metrics, font_raster) = font.rasterize(char, font_raster_size);
        let is_invisble_char = font_metrics.width == 0 || font_metrics.height == 0;
        if is_invisble_char {
            glyphs.push((char, font_metrics.width, font_metrics.height, None));
        } else {
            // u8 array -> sdfer
            let mut sdf_input =
                sdfer::Image2d::<sdfer::Unorm8>::new(font_metrics.width, font_metrics.height);
            for y in 0..font_metrics.height {
                for x in 0..font_metrics.width {
                    let v = font_raster[y * font_metrics.width + x];
                    sdf_input[(x, y)] = sdfer::Unorm8::from_bits(v);
                }
            }

            // generate glyph sdf
            let (sdf_output, _) = sdfer::esdt::glyph_to_sdf(&mut sdf_input, sdf_params, None);
            let sdf_width = sdf_output.width();
            let sdf_height = sdf_output.height();

            // sdfer -> u8 array
            let mut sdf_raster = vec![0u8; sdf_width * sdf_height];
            for y in 0..sdf_height {
                for x in 0..sdf_width {
                    sdf_raster[y * sdf_width + x] = sdf_output[(x, y)].to_bits();
                }
            }

            total_area += sdf_width * sdf_height;
            glyphs.push((char, sdf_width, sdf_height, Some(sdf_raster)));
        }
    }

    // sort by glyph height
    glyphs.sort_by_key(|(_, _, height, _)| *height);

    // packing
    // split up packing and atlas creation since packing might resize

    // calculate atlas size
    let mut atlas_glyph_lookup = HashMap::new();
    let mut atlas_side_size = ((total_area as f32).sqrt() as usize).next_power_of_two(); // TODO: cleanup
    let mut atlas_offset_x;
    let mut atlas_offset_y;
    let mut row_y_max;

    loop {
        let mut packing_success = true;
        atlas_offset_x = 0;
        atlas_offset_y = 0;
        row_y_max = 0;

        for (char, width, height, _) in glyphs.iter() {
            // check for wrapping
            if width + atlas_offset_x > atlas_side_size {
                // check for space left
                if height + atlas_offset_y > atlas_side_size {
                    packing_success = false;
                    break;
                }

                atlas_offset_y += row_y_max;
                row_y_max = 0;
                atlas_offset_x = 0;
            }

            atlas_glyph_lookup.insert(
                *char,
                AtlasGlyphInfo {
                    atlas_offset_x,
                    atlas_offset_y,

                    atlas_width: *width,
                    atlas_height: *height,
                },
            );

            atlas_offset_x += width;
            row_y_max = row_y_max.max(*height);
        }

        if packing_success {
            break;
        } else {
            atlas_side_size *= 2;
            atlas_glyph_lookup.clear();
        }
    }

    // atlas creation using packing info
    let atlas_size = atlas_side_size * atlas_side_size;
    let mut atlas_data = vec![0u8; atlas_size];
    for &(char, width, height, ref raster) in glyphs.iter() {
        // skip invisible chars such as space, tab...
        let Some(raster) = raster else {
            continue;
        };

        let glyph_info = atlas_glyph_lookup.get(&char).expect("could not find glyph");

        for glyph_x in 0..width {
            for glyph_y in 0..height {
                let index = glyph_x + glyph_y * width;
                let value = raster[index];

                let atlas_x = glyph_info.atlas_offset_x + glyph_x;
                let atlas_y = glyph_info.atlas_offset_y + glyph_y;
                let index = atlas_x + atlas_y * atlas_side_size;
                atlas_data[index] = value;
            }
        }
    }

    let font_atlas = render::TextureBuilder::new()
        .with_format(wgpu::TextureFormat::R8Unorm)
        .build(
            ctx,
            render::TextureSource::Data(atlas_side_size as u32, atlas_side_size as u32, atlas_data),
        );

    (atlas_glyph_lookup, font_atlas)
}

const ELEMENT_TYPE_CONTAINER: u32 = 0;
const ELEMENT_TYPE_GLYPH: u32 = 1;

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct UIElementInstace {
    // flag containing type of element
    // container: 0
    // glyph: 1
    pub element_type: u32,

    // uv coordinate system, (0,0) top left and y+ is down
    pub position: [f32; 2],
    pub size: [f32; 2],
    pub color: [f32; 4],

    // fonts
    pub font_atlas_offset: [f32; 2],
    pub font_atlas_size: [f32; 2],
}

impl UIElementInstace {
    pub fn desc() -> render::VertexBufferLayout {
        render::VertexBufferLayout::from_vertex_formats(
            wgpu::VertexStepMode::Instance,
            vec![
                wgpu::VertexFormat::Uint32,    // element type
                wgpu::VertexFormat::Float32x2, // pos
                wgpu::VertexFormat::Float32x2, // scale
                wgpu::VertexFormat::Float32x4, // color
                wgpu::VertexFormat::Float32x2, // atlas offset
                wgpu::VertexFormat::Float32x2, // atlas size
            ],
        )
    }
}

#[derive(Clone)]
pub struct Font {
    font: fontdue::Font,
}

impl gbase::asset::Asset for Font {}

#[derive(Clone)]
pub struct FontLoader {
    settings: fontdue::FontSettings,
}

impl AssetLoader for FontLoader {
    type Asset = Font;
    type Error = filesystem::LoadFileError;

    async fn load(
        &self,
        load_ctx: gbase::asset::LoadContext,
        path: &std::path::Path,
    ) -> Result<Self::Asset, Self::Error> {
        let bytes = load_ctx.load_bytes(path).await?;
        let font = fontdue::Font::from_bytes(bytes, self.settings)
            .expect("could not create font from bytes");

        Ok(Font { font })
    }
}

#[derive(Clone)]
pub struct FontAtlas {
    lookup: HashMap<char, AtlasGlyphInfo>,
    texture: render::ArcTexture,
}

impl DerivedAsset for FontAtlas {}
pub struct FontAtlasConverter<'a> {
    supported_chars: &'a [char],
    font_raster_size: f32,
}
impl<'a> AssetConverter for FontAtlasConverter<'a> {
    type SourceAsset = Font;
    type TargetAsset = FontAtlas;
    type Error = EmptyError;

    fn convert(
        &self,
        ctx: &mut gbase::Context,
        cache: &mut AssetCache,
        source: AssetHandle<Self::SourceAsset>,
    ) -> gbase::asset::ConvertAssetStatus<Self::TargetAsset> {
        let source = match source.get(cache) {
            GetAssetResult::Loading => return ConvertAssetStatus::SourceLoading,
            GetAssetResult::Failed => return ConvertAssetStatus::Failed,
            GetAssetResult::Success(source) => source,
        };
        let (lookup, texture) = create_font_atlas(
            ctx,
            &source.font,
            self.supported_chars,
            self.font_raster_size,
        );

        ConvertAssetStatus::Success(FontAtlas { lookup, texture })
    }
}
