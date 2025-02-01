use gbase::{
    filesystem,
    render::{self, ArcBindGroup, ArcRenderPipeline, VertexBufferBuilder, VertexBufferSource},
    wgpu, Callbacks, Context,
};

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub async fn run() {
    gbase::run::<App>().await;
}

struct App {
    vertex_buffer: render::VertexBuffer<render::VertexUV>,
    texture_bindgroup: ArcBindGroup,
    pipeline: ArcRenderPipeline,
}

impl Callbacks for App {
    fn new(ctx: &mut Context) -> Self {
        let vertex_buffer =
            VertexBufferBuilder::new(VertexBufferSource::Data(QUAD_VERTICES.to_vec())).build(ctx);

        let texture = gbase_utils::texture_builder_from_image_bytes(
            &filesystem::load_b!("textures/texture.jpeg").unwrap(),
        )
        .unwrap()
        .build(ctx)
        .with_default_view(ctx);

        let sampler = render::SamplerBuilder::new().build(ctx);

        let shader_str = filesystem::load_s!("shaders/texture.wgsl").unwrap();
        let shader = render::ShaderBuilder::new(shader_str).build(ctx);

        let texture_bindgroup_layout = render::BindGroupLayoutBuilder::new()
            .entries(vec![
                // texture
                render::BindGroupLayoutEntry::new()
                    .fragment()
                    .texture_float_filterable(),
                // sampler
                render::BindGroupLayoutEntry::new()
                    .fragment()
                    .sampler_filtering(),
            ])
            .build(ctx);
        let texture_bindgroup = render::BindGroupBuilder::new(texture_bindgroup_layout.clone())
            .entries(vec![
                // texture
                render::BindGroupEntry::Texture(texture.view()),
                // sampler
                render::BindGroupEntry::Sampler(sampler),
            ])
            .build(ctx);

        let pipeline_layout = render::PipelineLayoutBuilder::new()
            .bind_groups(vec![texture_bindgroup_layout.clone()])
            .build_uncached(ctx);
        let pipeline = render::RenderPipelineBuilder::new(shader, pipeline_layout)
            .single_target(render::ColorTargetState::from_current_screen(ctx))
            .buffers(vec![vertex_buffer.desc()])
            .build(ctx);

        Self {
            vertex_buffer,
            pipeline,
            texture_bindgroup,
        }
    }
    fn render(&mut self, ctx: &mut Context, screen_view: &wgpu::TextureView) -> bool {
        let mut encoder = render::EncoderBuilder::new().build(ctx);
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

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_bind_group(0, Some(self.texture_bindgroup.as_ref()), &[]);
        render_pass.draw(0..self.vertex_buffer.len(), 0..1);

        drop(render_pass);
        queue.submit(Some(encoder.finish()));

        false
    }
}

#[rustfmt::skip]
const QUAD_VERTICES: &[render::VertexUV] = &[
    render::VertexUV { position: [-0.5, -0.5, 0.0], uv: [0.0, 1.0] }, // bottom left
    render::VertexUV { position: [ 0.5,  0.5, 0.0], uv: [1.0, 0.0] }, // top right
    render::VertexUV { position: [-0.5,  0.5, 0.0], uv: [0.0, 0.0] }, // top left

    render::VertexUV { position: [-0.5, -0.5, 0.0], uv: [0.0, 1.0] }, // bottom left
    render::VertexUV { position: [ 0.5, -0.5, 0.0], uv: [1.0, 1.0] }, // bottom right
    render::VertexUV { position: [ 0.5,  0.5, 0.0], uv: [1.0, 0.0] }, // top right
];
