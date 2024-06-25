use gbase::{
    filesystem,
    render::{self, ArcBindGroup, ArcRenderPipeline},
    Callbacks, Context, ContextBuilder,
};
use std::path::Path;

#[pollster::main]
pub async fn main() {
    let (mut ctx, ev) = ContextBuilder::new()
        .log_level(gbase::LogLevel::Info)
        .build()
        .await;
    let app = App::new(&mut ctx).await;
    gbase::run(app, ctx, ev);
}

struct App {
    vertex_buffer: render::VertexBuffer<render::VertexUV>,
    texture_bindgroup: ArcBindGroup,
    pipeline: ArcRenderPipeline,
}

impl App {
    async fn new(ctx: &mut Context) -> Self {
        let vertex_buffer = render::VertexBufferBuilder::new(QUAD_VERTICES.to_vec()).build(ctx);

        let texture_bytes = filesystem::load_bytes(ctx, Path::new("texture.jpeg"))
            .await
            .unwrap();
        let texture =
            render::TextureBuilder::new(render::TextureSource::Bytes(texture_bytes)).build(ctx);
        let sampler = render::SamplerBuilder::new().build(ctx);

        let shader_str = filesystem::load_string(ctx, "texture.wgsl").await.unwrap();
        let shader = render::ShaderBuilder::new().source(shader_str).build(ctx);

        let texture_bindgroup_layout = render::BindGroupLayoutBuilder::new()
            .entries(vec![
                // texture
                render::BindGroupLayoutEntry::new()
                    .fragment()
                    .texture_float(true),
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
            .targets(vec![render::RenderPipelineBuilder::default_target(ctx)])
            .buffers(vec![vertex_buffer.desc()])
            .build(ctx);

        Self {
            vertex_buffer,
            pipeline,
            texture_bindgroup,
        }
    }
}

impl Callbacks for App {
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
        render_pass.set_bind_group(0, &self.texture_bindgroup, &[]);
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
