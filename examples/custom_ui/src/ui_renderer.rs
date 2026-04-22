use crate::ui_layout::{Glyph, TextLayoutResult, TextSizeResult, UIElement, UILayoutTextMeasurer};
use core::f32;
use gbase::{
    asset::{AssetCache, AssetHandle, ConvertAssetResult, ShaderLoader},
    bytemuck,
    glam::{self, Mat4},
    render::{self, BindGroupBindable},
    tracing, wgpu,
};
use std::collections::HashMap;

pub struct UIRenderer {
    shader_handle: AssetHandle<render::ShaderBuilder>,
    bindgroup_layout: render::ArcBindGroupLayout,
    pipeline_layout: render::ArcPipelineLayout,

    instance_buffer: render::RawBuffer<UIElementInstace>,

    projection: render::UniformBuffer<glam::Mat4>,

    // TODO: remove pub
    pub font_atlas: render::ArcTexture,
    glyph_lookup: HashMap<char, AtlasGlyphInfo>,

    font: fontdue::Font,
}

impl UIRenderer {
    pub fn new(
        ctx: &mut gbase::Context,
        cache: &mut AssetCache,
        font: &[u8],
        max_elements: u64,
    ) -> Self {
        let font_atlas_raster_size = 256.0;
        //
        // create font atlas
        //

        let mut supported_chars = Vec::new();
        for char in 'a'..='z' {
            supported_chars.push(char);
        }
        for char in 'A'..='Z' {
            supported_chars.push(char);
        }
        for char in '0'..='9' {
            supported_chars.push(char);
        }
        for char in " ,.;/".chars() {
            supported_chars.push(char);
        }

        let font = fontdue::Font::from_bytes(font, fontdue::FontSettings::default())
            .expect("could not build font from bytes");
        let (glyph_lookup, font_atlas) =
            create_font_atlas(ctx, &font, &supported_chars, font_atlas_raster_size);

        //
        // gpu resources
        //

        let shader_handle = cache
            .load_builder("assets/shaders/ui.wgsl", ShaderLoader {})
            .watch(cache)
            .build(cache);

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

        Self {
            pipeline_layout,
            bindgroup_layout,
            shader_handle,
            instance_buffer,
            projection,

            font_atlas,
            glyph_lookup,
            font,
        }
    }

    pub fn render(
        &self,
        ctx: &mut gbase::Context,
        cache: &mut AssetCache,
        view: &wgpu::TextureView,
        view_format: wgpu::TextureFormat,
        ui_elements: &[UIElement],
    ) {
        let ConvertAssetResult::Success(shader) = self.shader_handle.convert(ctx, cache) else {
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
            let atlas_width = self.font_atlas.width();
            let atlas_height = self.font_atlas.height();
            if !element.text_info.text.is_empty() {
                for glyph in element.text_layout.glyphs.iter() {
                    // TODO: backup
                    let glyph_info = self
                        .glyph_lookup
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

        let atlas_view = render::TextureViewBuilder::new(self.font_atlas.clone()).build(ctx);
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
}

impl UILayoutTextMeasurer for UIRenderer {
    fn calculate_preferred_text_size(
        &mut self,
        text: &str,
        font_size: u32,
        wrap_on_newline: bool,
    ) -> TextSizeResult {
        let line_metrics = self
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
            let font_metrics = self.font.metrics(letter, font_size as f32);

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
                if let Some(kern) = self.font.horizontal_kern(prev, letter, font_size as f32) {
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

    fn layout_text(&mut self, text: &str, font_size: u32, max_width: f32) -> TextLayoutResult {
        let line_metrics = self
            .font
            .horizontal_line_metrics(font_size as f32)
            .expect("could not get line metrics");
        let line_height = line_metrics.new_line_size;

        let mut longest_line_width = 0.0f32;
        let mut x_offset = 0.0f32;
        let mut y_offset = 0.0f32; // push offset back
        let mut glyphs = Vec::new();

        let mut prev_char = None;
        for letter in text.chars() {
            let font_metrics = self.font.metrics(letter, font_size as f32);

            if let Some(prev) = prev_char {
                if let Some(kern) = self.font.horizontal_kern(prev, letter, font_size as f32) {
                    x_offset += kern;
                }
            }

            // wrapping
            if x_offset + font_metrics.width as f32 > max_width {
                y_offset += line_height;
                x_offset = 0.0;
                longest_line_width = longest_line_width.max(x_offset);
            }
            // TODO: scale?

            glyphs.push(Glyph {
                character: letter,
                x: x_offset,
                y: y_offset + line_height - font_metrics.height as f32 - font_metrics.ymin as f32
                    + line_metrics.descent,
                width: font_metrics.width as f32,
                height: font_metrics.height as f32,
            });

            x_offset += font_metrics.advance_width;
            prev_char = Some(letter);
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
            let width = sdf_output.width();
            let height = sdf_output.height();

            // sdfer -> u8 array
            let mut sdf_raster = vec![0u8; width * height];
            for y in 0..height {
                for x in 0..width {
                    sdf_raster[y * width + x] = sdf_output[(x, y)].to_bits();
                }
            }

            total_area += width * height;
            glyphs.push((char, width, height, Some(sdf_raster)));
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

    let font_atlas = render::TextureBuilder::new(render::TextureSource::Data(
        atlas_side_size as u32,
        atlas_side_size as u32,
        atlas_data,
    ))
    .with_format(wgpu::TextureFormat::R8Unorm)
    .build(ctx);

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
