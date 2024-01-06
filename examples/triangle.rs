use gbase::{
    render::{self, Vertex},
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
    vertex_buffer: render::VertexBuffer<Vertex>,
    pipeline: render::RenderPipeline,
}

impl App {
    async fn new(ctx: &mut Context) -> Self {
        let surface_config = render::surface_config(ctx);

        let vertex_buffer = render::VertexBufferBuilder::new()
            .label("triangle vertex buffer")
            .vertices(TRIANGLE_VERTICES)
            .build(ctx);

        let shader = render::ShaderBuilder::new("triangle.wgsl".to_string())
            .buffers(&[vertex_buffer.desc()])
            .targets(&[Some(wgpu::ColorTargetState {
                format: surface_config.format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })])
            .build(ctx)
            .await;

        let render_pipeline = render::RenderPipelineBuilder::new(&shader).build(ctx);

        Self {
            vertex_buffer,
            pipeline: render_pipeline,
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

        render_pass.set_pipeline(self.pipeline.pipeline());
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.draw(0..self.vertex_buffer.len(), 0..1);

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
