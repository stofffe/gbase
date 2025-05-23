use gbase::{
    render::{self, ArcTextureView, VertexUV},
    wgpu, Context,
};

pub struct TextureRenderer {
    shader: render::ArcShaderModule,
    pipeline_layout: render::ArcPipelineLayout,
    bindgroup_layout: render::ArcBindGroupLayout,
    vertices: render::VertexBuffer<VertexUV>,
    indices: render::IndexBuffer,
    sampler: render::ArcSampler,
}

impl TextureRenderer {
    pub fn new(ctx: &mut Context) -> Self {
        let shader =
            render::ShaderBuilder::new(include_str!("../assets/shaders/texture.wgsl")).build(ctx);

        let sampler = render::SamplerBuilder::new().build(ctx);

        let vertices = render::VertexBufferBuilder::new(render::VertexBufferSource::Data(
            CENTERED_QUAD_VERTICES.to_vec(),
        ))
        .build(ctx);
        let indices = render::IndexBufferBuilder::new(render::IndexBufferSource::Data(
            CENTERED_QUAD_INDICES.to_vec(),
        ))
        .build(ctx);
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

        Self {
            vertices,
            indices,
            shader,
            bindgroup_layout,
            pipeline_layout,
            sampler,
        }
    }

    pub fn render(
        &self,
        ctx: &mut Context,
        in_texture: ArcTextureView,
        out_texture: &wgpu::TextureView,
        out_texture_format: wgpu::TextureFormat,
    ) {
        let bindgroup = render::BindGroupBuilder::new(self.bindgroup_layout.clone())
            .entries(vec![
                // texture
                render::BindGroupEntry::Texture(in_texture),
                // sampler
                render::BindGroupEntry::Sampler(self.sampler.clone()),
            ])
            .build(ctx);

        let pipeline =
            render::RenderPipelineBuilder::new(self.shader.clone(), self.pipeline_layout.clone())
                .single_target(render::ColorTargetState::new().format(out_texture_format))
                .buffers(vec![self.vertices.desc()])
                .build(ctx);

        render::RenderPassBuilder::new()
            .label("texture renderer")
            .color_attachments(&[Some(
                render::RenderPassColorAttachment::new(out_texture).clear(wgpu::Color::BLACK),
            )])
            .build_run_submit(ctx, |mut render_pass| {
                render_pass.set_pipeline(&pipeline);
                render_pass.set_vertex_buffer(0, self.vertices.slice(..));
                render_pass.set_index_buffer(self.indices.slice(..), self.indices.format());
                render_pass.set_bind_group(0, Some(bindgroup.as_ref()), &[]);
                render_pass.draw_indexed(0..self.indices.len(), 0, 0..1);
            });
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
