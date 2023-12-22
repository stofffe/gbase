use gbase::{
    filesystem,
    render::{self, Vertex},
    Callbacks, Context, ContextBuilder, LogLevel,
};
use std::path::Path;
use wgpu::util::DeviceExt;

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
    vertex_buffer: wgpu::Buffer,
    pipeline: wgpu::RenderPipeline,
}

impl App {
    async fn new(ctx: &mut Context) -> Self {
        let device = render::device(ctx);
        let surface_config = render::surface_config(ctx);

        // Shader
        let shader_bytes = filesystem::load_bytes(ctx, Path::new("triangle.wgsl"))
            .await
            .unwrap();
        let shader_str = String::from_utf8(shader_bytes).unwrap();
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(shader_str.into()),
        });

        // Vertex buffer
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("vertex buffer"),
            contents: bytemuck::cast_slice(TRIANGLE_VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        // Pipeline
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("render pipeline layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("render pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        Self {
            vertex_buffer,
            pipeline,
        }
    }
}

impl Callbacks for App {
    fn render(
        &mut self,
        _ctx: &mut Context,
        encoder: &mut wgpu::CommandEncoder,
        screen_view: &wgpu::TextureView,
    ) -> bool {
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

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.draw(0..TRIANGLE_VERTICES.len() as u32, 0..1);

        drop(render_pass);

        false
    }
}

#[rustfmt::skip]
const TRIANGLE_VERTICES: &[Vertex] = &[
    Vertex { position: [-0.5, -0.5, 0.0]  },
    Vertex { position: [0.5, -0.5, 0.0]   },
    Vertex { position: [0.0, 0.5, 0.0] },
];
