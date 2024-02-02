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
            .usages(wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST)
            .source(render::BufferSource::Values(TRIANGLE_VERTICES))
            .build(ctx);

        let shader = render::ShaderBuilder::new("triangle.wgsl".to_string())
            .buffers(vec![vertex_buffer.desc()])
            .targets(vec![Some(wgpu::ColorTargetState {
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
    fn render(&mut self, ctx: &mut Context, screen_view: &wgpu::TextureView) -> bool {
        let queue = render::queue(ctx);

        queue.write_buffer(
            self.vertex_buffer.buffer(),
            0,
            bytemuck::cast_slice(TRIANGLE_VERTICES),
        );

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

        render_pass.set_pipeline(self.pipeline.pipeline());
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.draw(0..self.vertex_buffer.len(), 0..1);

        drop(render_pass);
        queue.submit(Some(encoder.finish()));

        queue.write_buffer(
            self.vertex_buffer.buffer(),
            0,
            bytemuck::cast_slice(TRIANGLE_VERTICES_2),
        );

        let mut encoder = render::create_encoder(ctx, None);
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("render pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: screen_view,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
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
        queue.submit(Some(encoder.finish()));

        false
    }
}

#[rustfmt::skip]
const TRIANGLE_VERTICES: &[Vertex] = &[
    // Vertex { position: [-0.5, -0.5, 0.0] }, // bottom left
    // Vertex { position: [ 0.5,  0.5, 0.0] }, // top right
    // Vertex { position: [-0.5,  0.5, 0.0] }, // top left
    //
    // Vertex { position: [-0.5, -0.5, 0.0] }, // bottom left
    // Vertex { position: [ 0.5, -0.5, 0.0] }, // bottom right
    // Vertex { position: [ 0.5,  0.5, 0.0] }, // top right
    Vertex { position: [-0.5, -0.5, 0.0]  },
    Vertex { position: [0.5, -0.5, 0.0]   },
    Vertex { position: [0.0, 0.5, 0.0] },
];

#[rustfmt::skip]
const TRIANGLE_VERTICES_2: &[Vertex] = &[
    Vertex { position: [-1.0, -1.0, 0.0]  },
    Vertex { position: [-0.8, -1.0, 0.0]   },
    Vertex { position: [-0.9, -0.8, 0.0] },
];
