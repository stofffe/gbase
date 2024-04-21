use gbase::{
    filesystem,
    render::{self, Vertex},
    Callbacks, Context, ContextBuilder,
};

#[pollster::main]
pub async fn main() {
    let (mut ctx, ev) = ContextBuilder::new().build().await;
    let app = App::new(&mut ctx).await;
    gbase::run(app, ctx, ev);
}

struct App {
    vertex_buffer: render::VertexBuffer<render::Vertex>,
    pipeline: wgpu::RenderPipeline,
}

impl App {
    async fn new(ctx: &mut Context) -> Self {
        let vertex_buffer = render::VertexBufferBuilder::new()
            .usage(wgpu::BufferUsages::VERTEX)
            .build_init(ctx, TRIANGLE_VERTICES);

        let shader_str = filesystem::load_string(ctx, "triangle.wgsl").await.unwrap();
        let shader = render::ShaderBuilder::new(&shader_str).build(ctx);
        let pipeline = render::RenderPipelineBuilder::new(&shader)
            .buffers(&[vertex_buffer.desc()])
            .targets(&[render::RenderPipelineBuilder::default_target(ctx)])
            .build(ctx);

        Self {
            vertex_buffer,
            pipeline,
        }
    }
}

impl Callbacks for App {
    fn render(&mut self, ctx: &mut Context, screen_view: &wgpu::TextureView) -> bool {
        let queue = render::queue(ctx);
        let mut encoder = render::create_encoder(ctx, None);
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
        render_pass.draw(0..self.vertex_buffer.len(), 0..1);

        drop(render_pass);
        queue.submit(Some(encoder.finish()));

        false
    }
}

#[rustfmt::skip]
const TRIANGLE_VERTICES: &[Vertex] = &[
    Vertex { position: [-0.5, -0.5, 0.0]  },
    Vertex { position: [0.5, -0.5, 0.0]   },
    Vertex { position: [0.0, 0.5, 0.0] },
];
