use gbase::{filesystem, render, Callbacks, Context, ContextBuilder};
use std::path::Path;

#[pollster::main]
pub async fn main() {
    let (mut ctx, ev) = ContextBuilder::new().build().await;
    let app = App::new(&mut ctx).await;
    gbase::run(app, ctx, ev);
}

struct App {
    vertex_buffer: render::VertexBuffer<render::VertexUV>,
    texture_bindgroup: wgpu::BindGroup,
    pipeline: wgpu::RenderPipeline,
}

impl App {
    async fn new(ctx: &mut Context) -> Self {
        let vertex_buffer = render::VertexBufferBuilder::new(QUAD_VERTICES).build(ctx);

        let texture_bytes = filesystem::load_bytes(ctx, Path::new("texture.jpeg"))
            .await
            .unwrap();
        let texture = render::TextureBuilder::new().build_init(ctx, &texture_bytes);
        let sampler = render::SamplerBuilder::new().build(ctx);

        let shader_str = filesystem::load_string(ctx, "texture.wgsl").await.unwrap();
        let shader = render::ShaderBuilder::new().build(ctx, &shader_str);

        let (texture_bindgroup_layout, texture_bindgroup) = render::BindGroupCombinedBuilder::new()
            .entries(&[
                // texture
                render::BindGroupCombinedEntry::new(texture.resource())
                    .visibility(wgpu::ShaderStages::FRAGMENT)
                    .ty(texture.binding_type()),
                // sampler
                render::BindGroupCombinedEntry::new(sampler.resource())
                    .visibility(wgpu::ShaderStages::FRAGMENT)
                    .ty(sampler.binding_filtering()),
            ])
            .build(ctx);

        let pipeline = render::RenderPipelineBuilder::new(&shader)
            .targets(&[render::RenderPipelineBuilder::default_target(ctx)])
            .buffers(&[vertex_buffer.desc()])
            .bind_groups(&[&texture_bindgroup_layout])
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
