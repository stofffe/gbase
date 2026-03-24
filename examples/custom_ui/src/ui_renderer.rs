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

    pub font_atlas: render::ArcTexture,
}

impl UIRenderer {
    pub fn new(ctx: &mut gbase::Context, cache: &mut AssetCache, max_elements: u64) -> Self {
        let shader_handle = cache
            .load_builder("assets/shaders/ui.wgsl", ShaderLoader {})
            .watch(cache)
            .build(cache);

        let bindgroup_layout = render::BindGroupLayoutBuilder::new()
            .entries(vec![
                // projection
                render::BindGroupLayoutEntry::new().uniform().vertex(),
            ])
            .build(ctx);

        let pipeline_layout = render::PipelineLayoutBuilder::new()
            .bind_groups(vec![bindgroup_layout.clone()])
            .build(ctx);
        let instance_buffer = render::RawBufferBuilder::new(max_elements).build(ctx);

        let projection = render::UniformBufferBuilder::new()
            .usage(wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST)
            .build(ctx);

        // let mut image = sdfer::Image2d::<sdfer::Unorm8>::new(64, 64);
        //
        // (image, _) = esdt::glyph_to_sdf(&mut image, esdt::Params::default(), None);
        //
        // let mut data = Vec::new();
        // for y in 0..64 {
        //     for x in 0..64 {
        //         data.push(image[(x, y)].to_bits());
        //     }
        // }

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
        let font = fontdue::Font::from_bytes(
            include_bytes!("../assets/fonts/font.ttf").as_ref(),
            fontdue::FontSettings::default(),
        )
        .unwrap();

        let mut glyphs = Vec::new();
        let mut total_area = 0;
        for char in supported_chars {
            let (metrics, raster) = font.rasterize(char, 128.0);
            total_area += metrics.width * metrics.height;
            glyphs.push((char, metrics, raster));
        }

        // sort by glyph height
        glyphs.sort_by_key(|(_, metrics, _)| metrics.height);

        // calculate atlas size
        let mut glyph_info_lookup = HashMap::new();
        let mut atlas_side_size = ((total_area as f32).sqrt() as u32).next_power_of_two(); // TODO: cleanup
        let mut x_offset = 0u32;
        let mut y_offset = 0u32;
        let mut row_y_max = 0u32;

        // packing
        loop {
            let mut packing_success = true;

            for (char, metrics, _) in glyphs.iter() {
                let char_width = metrics.width as u32;
                let char_height = metrics.height as u32;

                // check for wrapping
                if char_width + x_offset > atlas_side_size {
                    // check for space left
                    if char_height + y_offset > atlas_side_size {
                        packing_success = false;
                        break;
                    }

                    y_offset += row_y_max;
                    row_y_max = 0;
                    x_offset = 0;
                }

                glyph_info_lookup.insert(
                    char,
                    GlyphInfo {
                        letter: *char,
                        x: x_offset,
                        y: y_offset,
                        width: char_width,
                        height: char_height,
                    },
                );

                x_offset += char_width;
                row_y_max = row_y_max.max(char_height);
            }

            if packing_success {
                break;
            } else {
                atlas_side_size *= 2;
            }
        }

        // atlas creation
        let atlas_size = atlas_side_size * atlas_side_size;
        let mut atlas_data = vec![0u8; atlas_size as usize];
        for (char, _, raster) in glyphs.iter() {
            let glyph_info = glyph_info_lookup.get(&char).expect("could not find glyph");
            for glyph_x in 0..glyph_info.width {
                for glyph_y in 0..glyph_info.height {
                    let index = glyph_x + glyph_y * glyph_info.width;
                    let value = raster[index as usize];

                    let atlas_x = glyph_info.x + glyph_x;
                    let atlas_y = glyph_info.y + glyph_y;
                    let index = atlas_x + atlas_y * atlas_side_size;
                    atlas_data[index as usize] = value;
                }
            }
        }

        dbg!(atlas_side_size);
        dbg!(atlas_size);
        dbg!(atlas_data.len());

        let image = render::TextureBuilder::new(render::TextureSource::Data(
            atlas_side_size,
            atlas_side_size,
            atlas_data,
        ))
        .with_format(wgpu::TextureFormat::R8Unorm)
        .build(ctx);

        Self {
            pipeline_layout,
            bindgroup_layout,
            shader_handle,
            instance_buffer,
            projection,
            font_atlas: image,
        }
    }

    pub fn render(
        &self,
        ctx: &mut gbase::Context,
        cache: &mut AssetCache,
        view: &wgpu::TextureView,
        view_format: wgpu::TextureFormat,
        ui_elements: Vec<UIElementInstace>,
    ) {
        let ConvertAssetResult::Success(shader) = self.shader_handle.convert(ctx, cache) else {
            return;
        };

        self.instance_buffer.write(ctx, &ui_elements);
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

        let bindgroup = render::BindGroupBuilder::new(self.bindgroup_layout.clone())
            .entries(vec![
                // camera projection
                self.projection.bindgroup_entry(),
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

                pass.draw(0..4, 0..ui_elements.len() as u32);
            });
    }
}

struct GlyphInfo {
    letter: char,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct UIElementInstace {
    pub position: [f32; 2], // uv coordinate system, (0,0) top left and y+ is down
    pub size: [f32; 2],
    pub color: [f32; 4],
}

impl UIElementInstace {
    pub fn desc() -> render::VertexBufferLayout {
        render::VertexBufferLayout::from_vertex_formats(
            wgpu::VertexStepMode::Instance,
            vec![
                wgpu::VertexFormat::Float32x2, // pos
                wgpu::VertexFormat::Float32x2, // scale
                wgpu::VertexFormat::Float32x4, // color
            ],
        )
    }
}
