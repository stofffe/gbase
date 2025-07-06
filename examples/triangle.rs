use gbase::{
    filesystem,
    render::{self, ArcRenderPipeline},
    wgpu, Callbacks, Context,
};

#[cfg(not(target_arch = "wasm32"))]
fn main() {
    gbase::run_sync::<App>();
}

pub struct App {
    mesh: render::GpuMesh,
    pipeline: ArcRenderPipeline,
}

impl Callbacks for App {
    fn init_ctx() -> gbase::ContextBuilder {
        gbase::ContextBuilder::new().vsync(true)
    }

    fn new(ctx: &mut Context, _cache: &mut gbase::asset::AssetCache) -> Self {
        let mut vertex_buffer = render::Mesh::new(wgpu::PrimitiveTopology::TriangleStrip);
        vertex_buffer.set_attribute(
            render::VertexAttributeId::Position,
            render::VertexAttributeValues::Float32x3(vec![
                [-0.5, -0.5, 0.0],
                [0.5, -0.5, 0.0],
                [0.0, 0.5, 0.0],
            ]),
        );
        let mesh = vertex_buffer.to_gpu_mesh(ctx);

        let shader_str = filesystem::load_s!("shaders/triangle.wgsl").unwrap();
        let shader = render::ShaderBuilder::new(shader_str).build(ctx);
        let pipeline_layout = render::PipelineLayoutBuilder::new().build(ctx);
        let pipeline = render::RenderPipelineBuilder::new(shader.clone(), pipeline_layout.clone())
            .buffers(vertex_buffer.buffer_layout())
            .single_target(render::ColorTargetState::from_current_screen(ctx))
            .build(ctx);

        Self { mesh, pipeline }
    }
    fn render(
        &mut self,
        ctx: &mut Context,
        _cache: &mut gbase::asset::AssetCache,
        screen_view: &wgpu::TextureView,
    ) -> bool {
        let mut encoder = render::EncoderBuilder::new().build_new(ctx);
        render::RenderPassBuilder::new()
            .color_attachments(&[Some(
                render::RenderPassColorAttachment::new(screen_view).clear(wgpu::Color::BLUE),
            )])
            .build_run(&mut encoder, |mut render_pass| {
                render_pass.set_pipeline(&self.pipeline);

                self.mesh.bind_to_render_pass(&mut render_pass);
                render_pass.draw(0..self.mesh.vertex_count, 0..1);
            });
        encoder.submit(ctx);

        false
    }
}
