use crate::ui_layout::{Glyph, TextLayoutResult, TextSizeResult, UIElement, UILayoutTextMeasurer};
use core::f32;
use gbase::{
    asset::{AssetCache, AssetHandle, ConvertAssetResult, ShaderLoader},
    bytemuck,
    glam::{self, Mat4},
    render::{self, BindGroupBindable},
    wgpu,
};
use std::collections::HashMap;

pub struct UIRenderer {
    shader_handle: AssetHandle<render::ShaderBuilder>,
    bindgroup_layout: render::ArcBindGroupLayout,
    pipeline_layout: render::ArcPipelineLayout,

    instance_buffer: render::RawBuffer<UIElementInstace>,

    projection: render::UniformBuffer<glam::Mat4>,

    // TODO: remove pub
    font_atlas_raster_size: f32,
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
        let font_atlas_raster_size = 128.0;
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

            font_atlas_raster_size,
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
            instances.push(UIElementInstace {
                position: [element.x, element.y],
                size: [element.preferred_width, element.preferred_height],
                color: element.background_color.to_array(),
                font_atlas_offset: [0.0, 0.0],
                font_atlas_size: [0.0, 0.0],
            });

            let W = self.font_atlas.width() as f32;
            let H = self.font_atlas.height() as f32;
            // glyphs
            if !element.text_info.text.is_empty() {
                for glyph in element.text_layout.glyphs.iter() {
                    let glyph_info = self
                        .glyph_lookup
                        .get(&glyph.character)
                        .expect("could not find glyph");

                    // TODO:
                    if glyph_info.metrics.width == 0 || glyph_info.metrics.height == 0 {
                        continue;
                    }

                    let x = element.x + glyph.x;
                    let y = element.y + glyph.y;
                    instances.push(UIElementInstace {
                        position: [x, y],
                        size: [glyph.width, glyph.height],
                        color: element.text_info.text_color.to_array(),
                        font_atlas_offset: [
                            glyph_info.atlas_offset_x as f32 / W,
                            glyph_info.atlas_offset_y as f32 / H,
                        ],
                        font_atlas_size: [
                            glyph_info.metrics.width as f32 / W,
                            glyph_info.metrics.height as f32 / H,
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
            .single_target(render::ColorTargetState::new().format(view_format))
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
            .horizontal_line_metrics(self.font_atlas_raster_size)
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
            // TODO: should have fallback here
            let &glyph_info = &self
                .glyph_lookup
                .get(&letter)
                .expect("trying to get unsupported letter");

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

            current_word_width += glyph_info.metrics.advance_width;
            x_offset += glyph_info.metrics.advance_width;

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
            .horizontal_line_metrics(self.font_atlas_raster_size)
            .expect("could not get line metrics");
        let line_height = line_metrics.new_line_size;

        let mut longest_line_width = 0.0f32;
        let mut x_offset = 0.0f32;
        let mut y_offset = 0.0f32; // push offset back
        let mut glyphs = Vec::new();

        let mut prev_char = None;
        for letter in text.chars() {
            // TODO: should have fallback here
            let &glyph_info = &self
                .glyph_lookup
                .get(&letter)
                .expect("trying to get unsupported letter");

            if let Some(prev) = prev_char {
                if let Some(kern) = self.font.horizontal_kern(prev, letter, font_size as f32) {
                    x_offset += kern;
                }
            }

            // wrapping
            if x_offset + glyph_info.metrics.width as f32 > max_width {
                y_offset += line_height;
                x_offset = 0.0;
                longest_line_width = longest_line_width.max(x_offset);
            }

            glyphs.push(Glyph {
                character: letter,
                x: x_offset,
                y: y_offset + line_height
                    - glyph_info.metrics.height as f32
                    - glyph_info.metrics.ymin
                    + line_metrics.descent,
                width: glyph_info.metrics.width as f32,
                height: glyph_info.metrics.height as f32,
            });

            x_offset += glyph_info.metrics.advance_width;
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

#[derive(Debug, Clone)]
struct AtlasGlyphMetrics {
    xmin: f32,
    ymin: f32,
    width: usize,
    height: usize,
    advance_width: f32,
    advance_height: f32,
}

struct AtlasGlyphInfo {
    letter: char,

    atlas_offset_x: usize,
    atlas_offset_y: usize,

    metrics: AtlasGlyphMetrics,
}

fn create_font_atlas(
    ctx: &mut gbase::Context,
    font: &fontdue::Font,
    supported_chars: &[char],
    font_raster_size: f32,
) -> (HashMap<char, AtlasGlyphInfo>, render::ArcTexture) {
    let mut glyphs = Vec::new();
    let mut total_area = 0;

    // create sdf glyphs
    for &char in supported_chars {
        let (font_metrics, font_raster) = font.rasterize(char, font_raster_size);
        let is_invisble_char = font_metrics.width == 0 || font_metrics.height == 0;

        let sdf_params = sdfer::esdt::Params {
            pad: 8,
            radius: 6.0,
            cutoff: 0.5,
            solidify: true,
            preprocess: false,
        };
        let sdf_metrics = AtlasGlyphMetrics {
            width: font_metrics.width + 2 * sdf_params.pad,
            height: font_metrics.height + 2 * sdf_params.pad,

            xmin: font_metrics.xmin as f32,
            ymin: font_metrics.ymin as f32,
            advance_width: font_metrics.advance_width,
            advance_height: font_metrics.advance_height,
        };

        if is_invisble_char {
            glyphs.push((char, sdf_metrics, None));
        } else {
            // u8 array -> sdfer
            let mut sdf_input =
                sdfer::Image2d::<sdfer::Unorm8>::new(sdf_metrics.width, sdf_metrics.height);
            for y in 0..font_metrics.height {
                for x in 0..font_metrics.width {
                    let v = font_raster[y * font_metrics.width + x];
                    sdf_input[((x + sdf_params.pad), (y + sdf_params.pad))] =
                        sdfer::Unorm8::from_bits(v);
                }
            }

            let (sdf_output, _) = sdfer::esdt::glyph_to_sdf(&mut sdf_input, sdf_params, None);

            // sdfer -> u8 array
            let mut sdf_raster = vec![0u8; sdf_metrics.width * sdf_metrics.height];
            for y in 0..sdf_metrics.height {
                for x in 0..sdf_metrics.width {
                    sdf_raster[y * sdf_metrics.width + x] = sdf_output[(x, y)].to_bits();
                }
            }

            total_area += sdf_output.width() * sdf_output.height();
            glyphs.push((char, sdf_metrics, Some(sdf_raster)));
        }
    }

    // sort by glyph height
    glyphs.sort_by_key(|(_, sdf_metrics, _)| sdf_metrics.height);

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

        for (char, sdf_metrics, _) in glyphs.iter() {
            let raster_width = sdf_metrics.width;
            let raster_height = sdf_metrics.height;

            // check for wrapping
            if raster_width + atlas_offset_x > atlas_side_size {
                // check for space left
                if raster_height + atlas_offset_y > atlas_side_size {
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
                    letter: *char,

                    atlas_offset_x,
                    atlas_offset_y,

                    metrics: sdf_metrics.clone(),
                },
            );

            atlas_offset_x += raster_width;
            row_y_max = row_y_max.max(raster_height);
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
    for (char, _, raster) in glyphs.iter() {
        // skip invisible chars such as space, tab...
        let Some(raster) = raster else {
            continue;
        };

        let glyph_info = atlas_glyph_lookup.get(char).expect("could not find glyph");

        for glyph_x in 0..glyph_info.metrics.width {
            for glyph_y in 0..glyph_info.metrics.height {
                let index = glyph_x + glyph_y * glyph_info.metrics.width;
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

#[repr(C)]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct UIElementInstace {
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
                wgpu::VertexFormat::Float32x2, // pos
                wgpu::VertexFormat::Float32x2, // scale
                wgpu::VertexFormat::Float32x4, // color
                wgpu::VertexFormat::Float32x2, // atlas offset
                wgpu::VertexFormat::Float32x2, // atlas size
            ],
        )
    }
}
