//
// Deferred renderer
//

use crate::{filesystem, render, Context};

pub struct DeferredRenderer {
    pipeline: wgpu::RenderPipeline,
    bindgroup: wgpu::BindGroup,

    vertex_buffer: render::VertexBuffer<render::VertexUV>,
    debug_input: render::DebugInput,
}

impl DeferredRenderer {
    pub async fn new(
        ctx: &Context,
        buffers: &render::DeferredBuffers,
        camera: &render::UniformBuffer,
        light: &render::UniformBuffer,
    ) -> Self {
        let shader_str = filesystem::load_string(ctx, "deferred.wgsl").await.unwrap();
        let vertex_buffer = render::VertexBufferBuilder::new(QUAD_VERTICES).build(ctx);
        let shader = render::ShaderBuilder::new().build(ctx, &shader_str);
        let debug_input = render::DebugInput::new(ctx);
        let (bindgroup_layout, bindgroup) =
            Self::bindgroups(ctx, buffers, camera, light, &debug_input);
        let pipeline = render::RenderPipelineBuilder::new(&shader)
            .bind_groups(&[&bindgroup_layout])
            .targets(&[render::RenderPipelineBuilder::default_target(ctx)])
            .buffers(&[vertex_buffer.desc()])
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

        let color_attachments = [Some(wgpu::RenderPassColorAttachment {
            view: screen_view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                store: wgpu::StoreOp::Store,
            },
        })];
        let mut render_pass = render::RenderPassBuilder::new()
            .color_attachments(&color_attachments)
            .build(&mut encoder);

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_bind_group(0, &self.bindgroup, &[]);
        render_pass.draw(0..self.vertex_buffer.len(), 0..1);

        drop(render_pass);

        queue.submit(Some(encoder.finish()));
    }
    fn bindgroups(
        ctx: &Context,
        buffers: &render::DeferredBuffers,
        camera: &render::UniformBuffer,
        light: &render::UniformBuffer,
        debug_input: &render::DebugInput,
    ) -> (wgpu::BindGroupLayout, wgpu::BindGroup) {
        let sampler = render::SamplerBuilder::new().build(ctx);
        render::BindGroupCombinedBuilder::new()
            .entries(&[
                //sampler
                render::BindGroupCombinedEntry::new(sampler.resource())
                    .visibility(wgpu::ShaderStages::FRAGMENT)
                    .ty(sampler.binding_nonfiltering()),
                // position
                render::BindGroupCombinedEntry::new(buffers.position.resource())
                    .visibility(wgpu::ShaderStages::FRAGMENT)
                    .ty(buffers.position.binding_nonfilter()),
                // albedo
                render::BindGroupCombinedEntry::new(buffers.albedo.resource())
                    .visibility(wgpu::ShaderStages::FRAGMENT)
                    .ty(buffers.albedo.binding_nonfilter()),
                // normal
                render::BindGroupCombinedEntry::new(buffers.normal.resource())
                    .visibility(wgpu::ShaderStages::FRAGMENT)
                    .ty(buffers.normal.binding_nonfilter()),
                // roughness
                render::BindGroupCombinedEntry::new(buffers.roughness.resource())
                    .visibility(wgpu::ShaderStages::FRAGMENT)
                    .ty(buffers.roughness.binding_nonfilter()),
                // camera
                render::BindGroupCombinedEntry::new(camera.buf().as_entire_binding())
                    .visibility(wgpu::ShaderStages::FRAGMENT)
                    .uniform(),
                // light
                render::BindGroupCombinedEntry::new(light.buf().as_entire_binding())
                    .visibility(wgpu::ShaderStages::FRAGMENT)
                    .uniform(),
                // debug input
                render::BindGroupCombinedEntry::new(debug_input.buffer().as_entire_binding())
                    .visibility(wgpu::ShaderStages::FRAGMENT)
                    .uniform(),
            ])
            .build(ctx)
    }
    pub fn resize(
        &mut self,
        ctx: &Context,
        buffers: &render::DeferredBuffers,
        camera: &render::UniformBuffer,
        light: &render::UniformBuffer,
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
