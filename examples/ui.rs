use gbase::{
    render::{self, VertexColor},
    Callbacks, Context, ContextBuilder,
};
use glam::{vec2, vec3, Vec2, Vec3};

#[pollster::main]
pub async fn main() {
    let (ctx, ev) = ContextBuilder::new()
        .log_level(gbase::LogLevel::Info)
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
        let quads = 10;
        let gui_renderer = GUIRenderer::new(ctx, 4 * quads, 6 * quads).await;
        Self { gui_renderer }
    }
}

impl Callbacks for App {
    #[rustfmt::skip]
    fn update(&mut self, _ctx: &mut Context) -> bool {
        self.gui_renderer.draw_quad(vec2(0.0, 0.0), vec2(1.8, 1.8), vec3(1.0, 1.0, 1.0));
        self.gui_renderer.draw_quad(vec2(0.0, 0.0), vec2(0.2, 0.2), vec3(1.0, 0.0, 0.0));
        self.gui_renderer.draw_quad(vec2(0.5, 0.5), vec2(0.3, 0.1), vec3(0.0, 1.0, 0.0));
        self.gui_renderer.draw_quad(vec2(0.0, 0.0), vec2(0.2, 0.2), vec3(0.0, 0.0, 1.0));
        false
    }
    fn render(&mut self, ctx: &mut Context, screen_view: &wgpu::TextureView) -> bool {
        self.gui_renderer.render(ctx, screen_view);
        false
    }
}

struct GUIRenderer {
    batch: render::BatchBuffer<VertexColor>,
    pipeline: render::RenderPipeline,
}

impl GUIRenderer {
    async fn new(ctx: &Context, vertices_batch_size: u32, indices_batch_size: u32) -> Self {
        let surface_config = render::surface_config(ctx);
        let batch_buffer = render::BatchBufferBuilder::new()
            .vertices_size(vertices_batch_size)
            .indices_size(indices_batch_size)
            .build(ctx);
        let shader = render::ShaderBuilder::new("ui.wgsl")
            .buffers(vec![batch_buffer.vertices_desc()])
            .targets(vec![Some(wgpu::ColorTargetState {
                format: surface_config.format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })])
            .build(ctx)
            .await;
        let pipeline = render::RenderPipelineBuilder::new(&shader).build(ctx);
        Self {
            batch: batch_buffer,
            pipeline,
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
        self.batch.add_vertex(VertexColor { position: [x - half_x, y - half_y, 0.0], color, }); // bl
        self.batch.add_vertex(VertexColor { position: [x - half_x, y + half_y, 0.0], color, }); // tl
        self.batch.add_vertex(VertexColor { position: [x + half_x, y + half_y, 0.0], color, }); // tr
        self.batch.add_vertex(VertexColor { position: [x + half_x, y - half_y, 0.0], color, }); // br
        self.batch.add_index(offset); // bl
        self.batch.add_index(offset + 1); // tl
        self.batch.add_index(offset + 2); // tr
        self.batch.add_index(offset); // bl
        self.batch.add_index(offset + 2); // tr
        self.batch.add_index(offset + 3); // br
    }
}
