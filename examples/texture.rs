use std::path::Path;

use gbase::{
    filesystem,
    render::{self, VertexUV},
    Callbacks, Context, ContextBuilder, LogLevel,
};

#[pollster::main]
pub async fn main() {
    let (mut ctx, ev) = ContextBuilder::new()
        .log_level(LogLevel::Info)
        .build()
        .await;
    let app = App::new(&mut ctx).await;
    gbase::run(app, ctx, ev).await;
}

struct App {
    vertex_buffer: render::VertexBuffer<VertexUV>,
    texture: render::Texture,
    pipeline: render::RenderPipeline,
}

impl App {
    async fn new(ctx: &mut Context) -> Self {
        let device = render::device(ctx);
        let surface_config = render::surface_config(ctx);

        let vertex_buffer = render::VertexBuffer::new(device, QUAD_VERTICES);

        let texture_bytes = filesystem::load_bytes(ctx, Path::new("texture.jpeg"))
            .await
            .unwrap();
        let texture =
            render::TextureBuilder::new(render::TextureSource::FormattedBytes(texture_bytes))
                .build(ctx);

        let shader = render::ShaderBuilder::new("texture.wgsl".to_string())
            .buffers(vec![vertex_buffer.desc()])
            .targets(vec![Some(wgpu::ColorTargetState {
                format: surface_config.format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })])
            .bind_group_layouts(vec![&texture.bind_group_layout()])
            .build(ctx)
            .await;

        let pipeline = render::RenderPipelineBuilder::new(&shader).build(ctx);

        Self {
            vertex_buffer,
            pipeline,
            texture,
        }
    }
}

impl Callbacks for App {
    fn render(&mut self, ctx: &mut Context, screen_view: &wgpu::TextureView) -> bool {
        let mut encoder = render::create_encoder(ctx, None);
        let queue = render::queue(ctx);
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("render pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: screen_view,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLUE),
                    store: wgpu::StoreOp::Store,
                },
                resolve_target: None,
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        render_pass.set_pipeline(self.pipeline.pipeline());
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_bind_group(0, self.texture.bind_group(), &[]);
        render_pass.draw(0..self.vertex_buffer.len(), 0..1);

        drop(render_pass);
        queue.submit(Some(encoder.finish()));

        false
    }
}

#[rustfmt::skip]
const QUAD_VERTICES: &[VertexUV] = &[
    VertexUV { position: [-0.5, -0.5, 0.0], uv: [0.0, 1.0] }, // bottom left
    VertexUV { position: [ 0.5,  0.5, 0.0], uv: [1.0, 0.0] }, // top right
    VertexUV { position: [-0.5,  0.5, 0.0], uv: [0.0, 0.0] }, // top left

    VertexUV { position: [-0.5, -0.5, 0.0], uv: [0.0, 1.0] }, // bottom left
    VertexUV { position: [ 0.5, -0.5, 0.0], uv: [1.0, 1.0] }, // bottom right
    VertexUV { position: [ 0.5,  0.5, 0.0], uv: [1.0, 0.0] }, // top right
];
// let texture_rgba = image::load_from_memory(&texture_bytes).unwrap().to_rgba8();
// let texture = device.create_texture(&wgpu::TextureDescriptor {
//     label: Some("texture"),
//     size: wgpu::Extent3d {
//         width: texture_rgba.width(),
//         height: texture_rgba.height(),
//         depth_or_array_layers: 1,
//     },
//     mip_level_count: 1,
//     sample_count: 1,
//     dimension: wgpu::TextureDimension::D2,
//     format: wgpu::TextureFormat::Rgba8Unorm,
//     usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
//     view_formats: &[],
// });
// queue.write_texture(
//     wgpu::ImageCopyTexture {
//         texture: &texture,
//         mip_level: 0,
//         origin: wgpu::Origin3d::ZERO,
//         aspect: wgpu::TextureAspect::All,
//     },
//     &texture_rgba,
//     wgpu::ImageDataLayout {
//         offset: 0,
//         bytes_per_row: Some(4 * texture_rgba.width()),
//         rows_per_image: Some(texture_rgba.height()),
//     },
//     texture.size(),
// );
// let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
// let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
//     address_mode_u: wgpu::AddressMode::ClampToEdge,
//     address_mode_v: wgpu::AddressMode::ClampToEdge,
//     address_mode_w: wgpu::AddressMode::ClampToEdge,
//     mag_filter: wgpu::FilterMode::Nearest,
//     min_filter: wgpu::FilterMode::Nearest,
//     mipmap_filter: wgpu::FilterMode::Nearest,
//     ..Default::default()
// });
//
// let texture_bind_group_layout =
//     device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
//         label: Some("texture bind group layout"),
//         entries: &[
//             wgpu::BindGroupLayoutEntry {
//                 binding: 0,
//                 visibility: wgpu::ShaderStages::FRAGMENT,
//                 ty: wgpu::BindingType::Texture {
//                     sample_type: wgpu::TextureSampleType::Float { filterable: true },
//                     view_dimension: wgpu::TextureViewDimension::D2,
//                     multisampled: false,
//                 },
//                 count: None,
//             },
//             wgpu::BindGroupLayoutEntry {
//                 binding: 1,
//                 visibility: wgpu::ShaderStages::FRAGMENT,
//                 ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
//                 count: None,
//             },
//         ],
//     });
//
// let texture_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
//     label: Some("texture bind group"),
//     layout: &texture_bind_group_layout,
//     entries: &[
//         wgpu::BindGroupEntry {
//             binding: 0,
//             resource: wgpu::BindingResource::TextureView(&view),
//         },
//         wgpu::BindGroupEntry {
//             binding: 1,
//             resource: wgpu::BindingResource::Sampler(&sampler),
//         },
//     ],
// });
