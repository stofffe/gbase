use gbase::bytemuck;
use gbase::encase;
use gbase::glam;
use gbase::log;
use gbase::wgpu;
use gbase::winit;
use gbase::{
    filesystem,
    render::{self, ArcRenderPipeline, Vertex},
    Callbacks, Context,
};

#[wasm_bindgen::prelude::wasm_bindgen]
pub async fn main() {
    gbase::run::<App>().await;
}

struct App {
    vertex_buffer: render::VertexBuffer<render::Vertex>,
    pipeline: ArcRenderPipeline,
}

impl Callbacks for App {
    fn init_ctx() -> gbase::ContextBuilder {
        gbase::ContextBuilder::new().vsync(false)
    }

    fn new(ctx: &mut Context) -> Self {
        let vertex_buffer = render::VertexBufferBuilder::new(render::VertexBufferSource::Data(
            TRIANGLE_VERTICES.to_vec(),
        ))
        .usage(wgpu::BufferUsages::VERTEX)
        .build(ctx);

        let shader_str = filesystem::load_s!("shaders/triangle.wgsl").unwrap();
        let shader = render::ShaderBuilder::new(shader_str).build(ctx);
        let pipeline_layout = render::PipelineLayoutBuilder::new().build(ctx);
        let pipeline = render::RenderPipelineBuilder::new(shader.clone(), pipeline_layout.clone())
            .buffers(vec![vertex_buffer.desc()])
            .targets(vec![render::RenderPipelineBuilder::default_target(ctx)])
            .build(ctx);

        Self {
            vertex_buffer,
            pipeline,
        }
    }
    fn render(&mut self, ctx: &mut Context, screen_view: &wgpu::TextureView) -> bool {
        render::RenderPassBuilder::new()
            .color_attachments(&[Some(wgpu::RenderPassColorAttachment {
                view: screen_view,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLUE),
                    store: wgpu::StoreOp::Store,
                },
                resolve_target: None,
            })])
            .build_run_new_encoder(ctx, |mut render_pass| {
                render_pass.set_pipeline(&self.pipeline);
                render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                render_pass.draw(0..self.vertex_buffer.len(), 0..1);
            });

        false
    }
}

#[rustfmt::skip]
const TRIANGLE_VERTICES: &[Vertex] = &[
    Vertex { position: [-0.5, -0.5, 0.0]  },
    Vertex { position: [0.5, -0.5, 0.0]   },
    Vertex { position: [0.0, 0.5, 0.0] },
];
