use gbase::{
    filesystem,
    render::{self, VertexTrait},
    time, Callbacks, Context, ContextBuilder,
};
use glam::{uvec2, vec2, vec3, UVec2, Vec2, Vec3};
use std::{collections::HashMap, path::Path};

#[pollster::main]
pub async fn main() {
    let (ctx, ev) = ContextBuilder::new()
        .log_level(gbase::LogLevel::Warn)
        .vsync(false)
        .build()
        .await;
    let app = App::new(&ctx).await;
    gbase::run(app, ctx, ev).await;
}

struct App {
    gui_renderer: GUIRenderer,
}

impl App {
    async fn new(ctx: &Context) -> Self {
        let quads = 100;
        let gui_renderer = GUIRenderer::new(ctx, 4 * quads, 6 * quads).await;

        Self { gui_renderer }
    }
}

impl Callbacks for App {
    #[rustfmt::skip]
    fn update(&mut self, ctx: &mut Context) -> bool {
        self.gui_renderer.draw_quad(vec2(0.0, 0.0), vec2(2.0, 2.0), vec3(1.0, 1.0, 1.0));

        let fps_text = time::fps(ctx).to_string();
        self.gui_renderer.draw_text(vec2(-1.0,0.8), vec2(1.0,1.0), 1.0, &fps_text);

        self.gui_renderer.draw_text(vec2(-1.0,0.0), vec2(1.0,1.0), 1.0, "hello this is some text that is going to wrap a few times lol lol");
        false
    }
    fn render(&mut self, ctx: &mut Context, screen_view: &wgpu::TextureView) -> bool {
        self.gui_renderer.render(ctx, screen_view);
        false
    }
}

struct TextureAtlas {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    sampler: wgpu::Sampler,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
}

impl TextureAtlas {
    fn write_texture(&mut self, ctx: &Context, origin: UVec2, dimensions: UVec2, bytes: &[u8]) {
        let queue = render::queue(ctx);

        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: origin.x,
                    y: origin.y,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            bytes,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(dimensions.x),
                rows_per_image: Some(dimensions.y),
            },
            wgpu::Extent3d {
                width: dimensions.x,
                height: dimensions.y,
                depth_or_array_layers: 1,
            },
        );
    }
    fn new(ctx: &Context, size: UVec2) -> Self {
        let device = render::device(ctx);

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: size.x,
                height: size.y,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: None,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        Self {
            texture,
            view,
            sampler,
            bind_group_layout,
            bind_group,
        }
    }
}

struct FontAtlas {
    texture_atlas: TextureAtlas,
    info: HashMap<char, LetterInfo>,
    line_height: f32,
}

impl FontAtlas {
    async fn new(ctx: &Context, font_path: &str, supported_chars: &str) -> Self {
        // texture
        let font_bytes = filesystem::load_bytes(ctx, Path::new(font_path))
            .await
            .unwrap();
        let font = fontdue::Font::from_bytes(font_bytes, fontdue::FontSettings::default()).unwrap();

        let chars = supported_chars
            .chars()
            .map(|letter| {
                let (metrics, bitmap) = font.rasterize(letter, 128.0);
                (metrics, bitmap, letter)
            })
            .collect::<Vec<_>>();
        // chars.sort_by(|a, b| a.0.height.partial_cmp(&b.0.height).unwrap());
        let texture_dim = uvec2(1024, 1024);
        let max_height = chars
            .iter()
            .map(|(metrics, _, _)| metrics.height)
            .max()
            .unwrap() as u32;
        let line_height = max_height as f32 / texture_dim.y as f32;

        let mut texture_atlas = TextureAtlas::new(ctx, texture_dim);
        let mut offset = UVec2::ZERO;

        let padding = uvec2(10, 10);

        let mut info = HashMap::<char, LetterInfo>::new();

        for (metrics, bitmap, letter) in chars {
            let dimensions = uvec2(metrics.width as u32, metrics.height as u32);

            if dimensions.x + padding.x > texture_dim.x - offset.x {
                offset.y += max_height + padding.x;
                offset.x = 0;
            }

            info.insert(
                letter,
                LetterInfo {
                    uv_offset: offset.as_vec2() / texture_dim.as_vec2(),
                    uv_dimensions: dimensions.as_vec2() / texture_dim.as_vec2(),
                    min: vec2(metrics.xmin as f32, metrics.ymin as f32) / texture_dim.as_vec2(),
                    advance: vec2(metrics.advance_width, metrics.advance_height)
                        / texture_dim.as_vec2(),
                },
            );

            // println!("{:?}", metrics);
            texture_atlas.write_texture(ctx, offset, dimensions, &bitmap);
            offset.x += dimensions.x + padding.x;
        }

        Self {
            texture_atlas,
            info,
            line_height,
        }
    }
}

impl FontAtlas {
    pub fn texture(&self) -> &wgpu::Texture {
        &self.texture_atlas.texture
    }
    pub fn view(&self) -> &wgpu::TextureView {
        &self.texture_atlas.view
    }
    pub fn sampler(&self) -> &wgpu::Sampler {
        &self.texture_atlas.sampler
    }
    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.texture_atlas.bind_group_layout
    }
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.texture_atlas.bind_group
    }
}

#[derive(Debug, Clone)]
struct LetterInfo {
    uv_offset: Vec2,
    uv_dimensions: Vec2,
    min: Vec2,
    advance: Vec2,
}

struct GUIRenderer {
    batch: render::BatchBuffer<VertexUI>,
    pipeline: render::RenderPipeline,
    font_atlas: FontAtlas,
}

impl GUIRenderer {
    async fn new(ctx: &Context, vertices_batch_size: u32, indices_batch_size: u32) -> Self {
        let surface_config = render::surface_config(ctx);
        let batch = render::BatchBufferBuilder::new()
            .vertices_size(vertices_batch_size)
            .indices_size(indices_batch_size)
            .build(ctx);

        let font_atlas = FontAtlas::new(
            ctx,
            "font.ttf",
            "abcdefghijklmnopqrstuvxyzwABCDEFGHIJKLMNOPQRSTUVXYZW0123456789.,_ ",
        )
        .await;
        // println!("A info {:?}", letter_info.get(&'a'));

        let shader = render::ShaderBuilder::new("ui.wgsl")
            .buffers(vec![batch.vertices_desc()])
            .targets(vec![Some(wgpu::ColorTargetState {
                format: surface_config.format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })])
            .bind_group_layouts(vec![font_atlas.bind_group_layout()])
            .build(ctx)
            .await;
        let pipeline = render::RenderPipelineBuilder::new(&shader).build(ctx);
        Self {
            batch,
            pipeline,
            font_atlas,
        }
    }

    fn render(&mut self, ctx: &Context, screen_view: &wgpu::TextureView) {
        // Update buffers with current frames data
        self.batch.upload_buffers(ctx);

        // Render batch
        let queue = render::queue(ctx);
        let mut encoder = render::create_encoder(ctx, None);
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: screen_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        render_pass.set_pipeline(self.pipeline.pipeline());
        render_pass.set_vertex_buffer(0, self.batch.vertices_slice(..));
        render_pass.set_index_buffer(self.batch.indices_slice(..), self.batch.indices_format());
        render_pass.set_bind_group(0, self.font_atlas.bind_group(), &[]);
        render_pass.draw_indexed(0..self.batch.indices_len(), 0, 0..1);

        drop(render_pass);
        queue.submit(Some(encoder.finish()));

        // Clear for next frame
        self.batch.clear();
    }

    #[rustfmt::skip]
    fn draw_quad(&mut self, pos: Vec2, size: Vec2, color: Vec3) {
        let offset = self.batch.vertices_len();
        let color = color.to_array();
        let x = pos.x;
        let y = pos.y;
        let half_x = size.x * 0.5;
        let half_y = size.y * 0.5;
        self.batch.add_vertex(VertexUI { position: [x - half_x, y - half_y, 0.0], color, uv: [0.0, 0.0], ty: VERTEX_TYPE_SHAPE }); // bl
        self.batch.add_vertex(VertexUI { position: [x - half_x, y + half_y, 0.0], color, uv: [0.0, 0.0], ty: VERTEX_TYPE_SHAPE }); // tl
        self.batch.add_vertex(VertexUI { position: [x + half_x, y + half_y, 0.0], color, uv: [0.0, 0.0], ty: VERTEX_TYPE_SHAPE }); // tr
        self.batch.add_vertex(VertexUI { position: [x + half_x, y - half_y, 0.0], color, uv: [0.0, 0.0], ty: VERTEX_TYPE_SHAPE }); // br
        self.batch.add_index(offset); // bl
        self.batch.add_index(offset + 1); // tl
        self.batch.add_index(offset + 2); // tr
        self.batch.add_index(offset); // bl
        self.batch.add_index(offset + 2); // tr
        self.batch.add_index(offset + 3); // br
    }

    // currently size.y does nothing
    fn draw_text(&mut self, pos: Vec2, size: Vec2, scale: f32, text: &str) {
        let mut offset = vec2(0.0, 0.0);
        for letter in text.chars() {
            let info = self.font_atlas.info.get(&letter).unwrap().clone();

            if info.uv_dimensions.x > size.x - offset.x {
                offset.x = 0.0;
                offset.y -= self.font_atlas.line_height;
            }

            let local_offset = info.min;
            self.draw_letter(pos + (offset + local_offset) * scale, scale, letter);
            offset.x += info.advance.x;
        }
    }

    fn draw_letter(&mut self, pos: Vec2, scale: f32, letter: char) {
        let letter_info = self.font_atlas.info.get(&letter).unwrap().clone(); // clone
        let dim = letter_info.uv_dimensions;

        let scaled_dim = dim * scale;
        // self.draw_quad(pos, scaled_dim, vec3(0.0, 1.0, 0.0));

        let vertex_offset = self.batch.vertices_len();
        let text_color = vec3(0.0, 0.0, 0.0).to_array();
        let offset = letter_info.uv_offset;
        //  bl
        self.batch.add_vertex(VertexUI {
            position: [pos.x, pos.y, 0.0],
            color: text_color,
            uv: [offset.x, offset.y + dim.y],
            ty: VERTEX_TYPE_TEXT,
        });
        // tl
        self.batch.add_vertex(VertexUI {
            position: [pos.x, pos.y + scaled_dim.y, 0.0],
            color: text_color,
            uv: [offset.x, offset.y],
            ty: VERTEX_TYPE_TEXT,
        });
        // tr
        self.batch.add_vertex(VertexUI {
            position: [pos.x + scaled_dim.x, pos.y + scaled_dim.y, 0.0],
            color: text_color,
            uv: [offset.x + dim.x, offset.y],
            ty: VERTEX_TYPE_TEXT,
        });
        // br
        self.batch.add_vertex(VertexUI {
            position: [pos.x + scaled_dim.x, pos.y, 0.0],
            color: text_color,
            uv: [offset.x + dim.x, offset.y + dim.y],
            ty: VERTEX_TYPE_TEXT,
        });

        self.batch.add_index(vertex_offset); // bl
        self.batch.add_index(vertex_offset + 1); // tl
        self.batch.add_index(vertex_offset + 2); // tr
        self.batch.add_index(vertex_offset); // bl
        self.batch.add_index(vertex_offset + 2); // tr
        self.batch.add_index(vertex_offset + 3); // br
    }
}

const VERTEX_TYPE_SHAPE: u32 = 0;
const VERTEX_TYPE_TEXT: u32 = 1;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct VertexUI {
    pub position: [f32; 3],
    pub ty: u32, // 0 shape, 1 text
    pub color: [f32; 3],
    pub uv: [f32; 2],
}

impl VertexUI {
    const ATTRIBUTES: &'static [wgpu::VertexAttribute] = &wgpu::vertex_attr_array![
        0=>Float32x3,
        1=>Uint32,
        2=>Float32x3,
        3=>Float32x2,
    ];
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: Self::ATTRIBUTES,
        }
    }
}

impl VertexTrait for VertexUI {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        Self::desc()
    }
}

// println!("{:#?}", a);

// let origin = uvec2(32, 32);
// let dim = uvec2(48, 80);
// let bytes = vec![255u8; (dim.x * dim.y) as usize];
// letter_texture.write_texture(ctx, origin, dim, &bytes);
// let letter_texture = render::TextureBuilder::new(render::TextureSource::RawBytes {
//     width: metrics.width as u32,
//     height: metrics.height as u32,
//     format: render::TextureFormat::R8Unorm,
//     bytes: bitmap,
// })
// .build(ctx);

// let (write_w, write_h) = (48, 80);
// let (origin_x, origin_y) = (32, 32);
// let bytes = vec![255u8; (write_w * write_h) as usize];
// queue.write_texture(
//     wgpu::ImageCopyTexture {
//         texture: &texture,
//         mip_level: 0,
//         origin: wgpu::Origin3d {
//             x: origin_x,
//             y: origin_y,
//             z: 0,
//         },
//         aspect: wgpu::TextureAspect::All,
//     },
//     &bytes,
//     wgpu::ImageDataLayout {
//         offset: 0,
//         bytes_per_row: Some(write_w),
//         rows_per_image: Some(write_h),
//     },
//     wgpu::Extent3d {
//         width: write_w,
//         height: write_h,
//         depth_or_array_layers: 1,
//     },
// );
// //  bl
// self.batch.add_vertex(VertexUI {
//     position: [x, y - half_y, 0.0],
//     color,
//     uv: [offset.x, offset.y + dim.y],
//     ty: VERTEX_TYPE_TEXT,
// });
// // tl
// self.batch.add_vertex(VertexUI {
//     position: [x - half_x, y + half_y, 0.0],
//     color,
//     uv: [offset.x, offset.y],
//     ty: VERTEX_TYPE_TEXT,
// });
// // tr
// self.batch.add_vertex(VertexUI {
//     position: [x + half_x, y + half_y, 0.0],
//     color,
//     uv: [offset.x + dim.x, offset.y],
//     ty: VERTEX_TYPE_TEXT,
// });
// // br
// self.batch.add_vertex(VertexUI {
//     position: [x + half_x, y - half_y, 0.0],
//     color,
//     uv: [offset.x + dim.x, offset.y + dim.y],
//     ty: VERTEX_TYPE_TEXT,
// });