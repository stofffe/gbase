use crate::{AssetCache, AssetHandle};

use super::CameraUniform;
use encase::ShaderSize;
use gbase::{
    glam::Vec3,
    render::{self, ArcBindGroupLayout, ArcPipelineLayout, ArcShaderModule},
    wgpu, Context,
};

//
// Deferred renderer
//

pub struct DeferredRenderer {
    shader_handle: AssetHandle<render::ShaderBuilder>,

    pipeline_layout: ArcPipelineLayout,
    bindgroup_layout: ArcBindGroupLayout,

    vertex_buffer: render::VertexBuffer<render::VertexUV>,
    debug_input: crate::DebugInput,
}

impl DeferredRenderer {
    pub fn new(
        ctx: &mut Context,
        shader_cache: &mut AssetCache<render::ShaderBuilder, wgpu::ShaderModule>,
    ) -> Self {
        let vertex_buffer = render::VertexBufferBuilder::new(render::VertexBufferSource::Data(
            QUAD_VERTICES.to_vec(),
        ))
        .build(ctx);
        let shader_handle = shader_cache.allocate_reload(
            render::ShaderBuilder {
                label: None,
                source: include_str!("../assets/shaders/deferred.wgsl").into(),
            },
            "../../utils/gbase_utils/assets/shaders/deferred.wgsl".into(),
        );
        let debug_input = crate::DebugInput::new(ctx);
        let bindgroup_layout = render::BindGroupLayoutBuilder::new()
            .label("deferred")
            .entries(vec![
                // sampler
                render::BindGroupLayoutEntry::new()
                    .sampler_filtering()
                    .fragment(),
                // position
                render::BindGroupLayoutEntry::new()
                    .texture_float_filterable()
                    .fragment(),
                // albedo
                render::BindGroupLayoutEntry::new()
                    .texture_float_filterable()
                    .fragment(),
                // normal
                render::BindGroupLayoutEntry::new()
                    .texture_float_filterable()
                    .fragment(),
                // roughness
                render::BindGroupLayoutEntry::new()
                    .texture_float_filterable()
                    .fragment(),
                // camera
                render::BindGroupLayoutEntry::new().uniform().fragment(),
                // light
                render::BindGroupLayoutEntry::new().uniform().fragment(),
                // debug input
                render::BindGroupLayoutEntry::new().uniform().fragment(),
            ])
            .build(ctx);
        let pipeline_layout = render::PipelineLayoutBuilder::new()
            .bind_groups(vec![bindgroup_layout.clone()])
            .build(ctx);

        Self {
            shader_handle,
            pipeline_layout,
            bindgroup_layout,
            vertex_buffer,
            debug_input,
        }
    }

    pub fn render(
        &mut self,
        ctx: &mut Context,
        view: &wgpu::TextureView,
        view_format: wgpu::TextureFormat,
        buffers: &crate::DeferredBuffers,
        camera: &render::UniformBuffer<CameraUniform>,
        light: &render::UniformBuffer<Vec3>,
        shader_cache: &mut AssetCache<render::ShaderBuilder, wgpu::ShaderModule>,
    ) {
        self.debug_input.update_buffer(ctx);

        let sampler = render::SamplerBuilder::new().build(ctx);
        let bindgroup = render::BindGroupBuilder::new(self.bindgroup_layout.clone())
            .label("deferred bindgroup")
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
                render::BindGroupEntry::Buffer(self.debug_input.buffer()),
            ])
            .build(ctx);

        let shader = shader_cache.get_gpu(ctx, self.shader_handle.clone());
        let pipeline = render::RenderPipelineBuilder::new(shader, self.pipeline_layout.clone())
            .single_target(render::ColorTargetState::new().format(view_format))
            .buffers(vec![self.vertex_buffer.desc()])
            .build(ctx);

        render::RenderPassBuilder::new()
            .color_attachments(&[Some(render::RenderPassColorAttachment::new(view).load())])
            .build_run_submit(ctx, |mut render_pass| {
                render_pass.set_pipeline(&pipeline);
                render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                render_pass.set_bind_group(0, Some(bindgroup.as_ref()), &[]);
                render_pass.draw(0..self.vertex_buffer.len(), 0..1);
            });
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
