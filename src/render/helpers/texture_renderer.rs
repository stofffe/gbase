use crate::{
    filesystem,
    render::{self, ArcTextureView, VertexUV},
    Context,
};

pub struct TextureRenderer {
    pipeline: render::ArcRenderPipeline,
    bindgroup_layout: render::ArcBindGroupLayout,
    vertices: render::VertexBuffer<VertexUV>,
    indices: render::IndexBuffer,
    sampler: render::ArcSampler,
}

impl TextureRenderer {
    pub async fn new(ctx: &mut Context, output_texture_format: wgpu::TextureFormat) -> Self {
        let shader_str = filesystem::load_string(ctx, "texture.wgsl").await.unwrap();
        let shader = render::ShaderBuilder::new(shader_str).build(ctx);

        let sampler = render::SamplerBuilder::new().build(ctx);

        let vertices = render::VertexBufferBuilder::new(CENTERED_QUAD_VERTICES).build(ctx);
        let indices = render::IndexBufferBuilder::new(CENTERED_QUAD_INDICES).build(ctx);
        let bindgroup_layout = render::BindGroupLayoutBuilder::new()
            .entries(vec![
                // texture
                render::BindGroupLayoutEntry::new()
                    .texture_float_filterable()
                    .fragment(),
                // sampler
                render::BindGroupLayoutEntry::new()
                    .sampler_filtering()
                    .fragment(),
            ])
            .build(ctx);
        let pipeline_layout = render::PipelineLayoutBuilder::new()
            .bind_groups(vec![bindgroup_layout.clone()])
            .build(ctx);
        let pipeline = render::RenderPipelineBuilder::new(shader, pipeline_layout)
            .targets(vec![Some(wgpu::ColorTargetState {
                // format: wgpu::TextureFormat::Bgra8Unorm,
                // format: wgpu::TextureFormat::Bgra8UnormSrgb,
                format: output_texture_format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })])
            .buffers(vec![vertices.desc()])
            .build(ctx);

        Self {
            vertices,
            indices,
            bindgroup_layout,
            pipeline,
            sampler,
        }
    }

    pub fn render(
        &self,
        ctx: &mut Context,
        in_texture: ArcTextureView,
        out_texture: &wgpu::TextureView,
    ) {
        let bindgroup = render::BindGroupBuilder::new(self.bindgroup_layout.clone())
            .entries(vec![
                // texture
                render::BindGroupEntry::Texture(in_texture),
                // sampler
                render::BindGroupEntry::Sampler(self.sampler.clone()),
            ])
            .build(ctx);

        let queue = render::queue(ctx);
        let mut encoder = render::EncoderBuilder::new().build(ctx);

        let color_attachment = [Some(wgpu::RenderPassColorAttachment {
            view: out_texture,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                store: wgpu::StoreOp::Store,
            },
            resolve_target: None,
        })];
        let mut render_pass = render::RenderPassBuilder::new()
            .color_attachments(&color_attachment)
            .build(&mut encoder);

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_vertex_buffer(0, self.vertices.slice(..));
        render_pass.set_index_buffer(self.indices.slice(..), self.indices.format());
        render_pass.set_bind_group(0, &bindgroup, &[]);
        render_pass.draw_indexed(0..self.indices.len(), 0, 0..1);
        drop(render_pass);

        queue.submit(Some(encoder.finish()));
    }
}

#[rustfmt::skip]
const CENTERED_QUAD_VERTICES: &[render::VertexUV] = &[
    render::VertexUV { position: [-1.0, -1.0, 0.0], uv: [0.0, 1.0] }, // bottom left
    render::VertexUV { position: [ 1.0, -1.0, 0.0], uv: [1.0, 1.0] }, // bottom right
    render::VertexUV { position: [ 1.0,  1.0, 0.0], uv: [1.0, 0.0] }, // top right

    render::VertexUV { position: [-1.0, -1.0, 0.0], uv: [0.0, 1.0] }, // bottom left
    render::VertexUV { position: [ 1.0,  1.0, 0.0], uv: [1.0, 0.0] }, // top right
    render::VertexUV { position: [-1.0,  1.0, 0.0], uv: [0.0, 0.0] }, // top left

];

#[rustfmt::skip]
const CENTERED_QUAD_INDICES: &[u32] = &[
    0, 1, 2,
    3, 4, 5
];

// pub fn set_texture(&mut self, ctx: &mut Context, texture: ArcTextureView) {
//     self.bindgroup = render::BindGroupBuilder::new(self.bindgroup_layout.clone())
//         .entries(vec![
//             // texture
//             render::BindGroupEntry::Texture(texture),
//             // sampler
//             render::BindGroupEntry::Sampler(self.sampler.clone()),
//         ])
//         .build(ctx);
// }
