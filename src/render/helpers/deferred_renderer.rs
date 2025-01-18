use glam::Vec3;

use crate::{
    filesystem,
    render::{
        self, ArcBindGroup, ArcBindGroupLayout, ArcRenderPipeline, RenderPassColorAttachment,
    },
    Context,
};

use super::CameraUniform;

//
// Deferred renderer
//

pub struct DeferredRenderer {
    pipeline: ArcRenderPipeline,
    bindgroup: ArcBindGroup,

    vertex_buffer: render::VertexBuffer<render::VertexUV>,
    debug_input: render::DebugInput,
}

impl DeferredRenderer {
    pub fn new(
        ctx: &mut Context,
        output_format: wgpu::TextureFormat,
        buffers: &render::DeferredBuffers,
        camera: &render::UniformBuffer<CameraUniform>,
        light: &render::UniformBuffer<Vec3>,
    ) -> Self {
        let vertex_buffer = render::VertexBufferBuilder::new(render::VertexBufferSource::Data(
            QUAD_VERTICES.to_vec(),
        ))
        .build(ctx);
        let shader_str = filesystem::load_s!("shaders/deferred.wgsl").unwrap();
        let shader = render::ShaderBuilder::new(shader_str).build(ctx);
        let debug_input = render::DebugInput::new(ctx);
        let (bindgroup_layout, bindgroup) =
            Self::bindgroups(ctx, buffers, camera, light, &debug_input);
        let pipeline_layout = render::PipelineLayoutBuilder::new()
            .bind_groups(vec![bindgroup_layout])
            .build(ctx);
        let pipeline = render::RenderPipelineBuilder::new(shader, pipeline_layout)
            .single_target(render::ColorTargetState::new().format(output_format))
            .buffers(vec![vertex_buffer.desc()])
            .build(ctx);
        Self {
            pipeline,
            bindgroup,
            vertex_buffer,
            debug_input,
        }
    }

    pub fn render(&mut self, ctx: &Context, screen_view: &wgpu::TextureView) {
        self.debug_input.update_buffer(ctx);

        let queue = render::queue(ctx);
        let mut encoder = render::EncoderBuilder::new().build(ctx);

        render::RenderPassBuilder::new()
            .color_attachments(&[Some(
                render::RenderPassColorAttachment::new(screen_view).clear(wgpu::Color::BLACK),
            )])
            .build_run(&mut encoder, |mut render_pass| {
                render_pass.set_pipeline(&self.pipeline);
                render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                render_pass.set_bind_group(0, Some(self.bindgroup.as_ref()), &[]);
                render_pass.draw(0..self.vertex_buffer.len(), 0..1);
            });

        queue.submit(Some(encoder.finish()));
    }
    fn bindgroups(
        ctx: &mut Context,
        buffers: &render::DeferredBuffers,
        camera: &render::UniformBuffer<CameraUniform>,
        light: &render::UniformBuffer<Vec3>,
        debug_input: &render::DebugInput,
    ) -> (ArcBindGroupLayout, ArcBindGroup) {
        let sampler = render::SamplerBuilder::new().build(ctx);
        let bindgroup_layout = render::BindGroupLayoutBuilder::new()
            .entries(vec![
                // sampler
                render::BindGroupLayoutEntry::new()
                    .sampler_nonfiltering()
                    .fragment(),
                // position
                render::BindGroupLayoutEntry::new()
                    .texture_float_nonfilterable()
                    .fragment(),
                // albedo
                render::BindGroupLayoutEntry::new()
                    .texture_float_nonfilterable()
                    .fragment(),
                // normal
                render::BindGroupLayoutEntry::new()
                    .texture_float_nonfilterable()
                    .fragment(),
                // roughness
                render::BindGroupLayoutEntry::new()
                    .texture_float_nonfilterable()
                    .fragment(),
                // camera
                render::BindGroupLayoutEntry::new().uniform().fragment(),
                // light
                render::BindGroupLayoutEntry::new().uniform().fragment(),
                // debug input
                render::BindGroupLayoutEntry::new().uniform().fragment(),
            ])
            .build(ctx);
        let bindgroup = render::BindGroupBuilder::new(bindgroup_layout.clone())
            .entries(vec![
                // sampler
                render::BindGroupEntry::Sampler(sampler),
                // position
                render::BindGroupEntry::Texture(buffers.position.view()),
                // albedo
                render::BindGroupEntry::Texture(buffers.albedo.view()),
                // normal
                render::BindGroupEntry::Texture(buffers.normal.view()),
                // roughness
                render::BindGroupEntry::Texture(buffers.roughness.view()),
                // camera
                render::BindGroupEntry::Buffer(camera.buffer()),
                // light
                render::BindGroupEntry::Buffer(light.buffer()),
                // debug input
                render::BindGroupEntry::Buffer(debug_input.buffer()),
            ])
            .build(ctx);

        (bindgroup_layout, bindgroup)
    }
    pub fn rebuild_bindgroup(
        &mut self,
        ctx: &mut Context,
        buffers: &render::DeferredBuffers,
        camera: &render::UniformBuffer<CameraUniform>,
        light: &render::UniformBuffer<Vec3>,
    ) {
        let (_, bindgroup) = Self::bindgroups(ctx, buffers, camera, light, &self.debug_input);
        self.bindgroup = bindgroup;
    }
}

#[rustfmt::skip]
const QUAD_VERTICES: &[render::VertexUV] = &[
    render::VertexUV { position: [-1.0, -1.0, 0.0], uv: [0.0, 1.0] }, // bottom left
    render::VertexUV { position: [ 1.0,  1.0, 0.0], uv: [1.0, 0.0] }, // top right
    render::VertexUV { position: [-1.0,  1.0, 0.0], uv: [0.0, 0.0] }, // top left

    render::VertexUV { position: [-1.0, -1.0, 0.0], uv: [0.0, 1.0] }, // bottom left
    render::VertexUV { position: [ 1.0, -1.0, 0.0], uv: [1.0, 1.0] }, // bottom right
    render::VertexUV { position: [ 1.0,  1.0, 0.0], uv: [1.0, 0.0] }, // top right
];
