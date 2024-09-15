use crate::{
    filesystem,
    render::{self},
    Context,
};

use super::debug_input;

pub struct BoxFilter {
    pipeline: render::ArcComputePipeline,
    bindgroup_layout: render::ArcBindGroupLayout,
    debug_input: render::DebugInput,
}

impl BoxFilter {
    pub async fn new(ctx: &mut Context) -> Self {
        let shader_str = filesystem::load_string(ctx, "box_filter.wgsl")
            .await
            .unwrap();
        let shader = render::ShaderBuilder::new(shader_str).build(ctx);

        let debug_input = render::DebugInput::new(ctx);

        let bindgroup_layout = render::BindGroupLayoutBuilder::new()
            .entries(vec![
                // in
                render::BindGroupLayoutEntry::new()
                    .ty(wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    })
                    .compute(),
                // out
                render::BindGroupLayoutEntry::new()
                    .ty(wgpu::BindingType::StorageTexture {
                        access: wgpu::StorageTextureAccess::WriteOnly,
                        format: wgpu::TextureFormat::Rgba8Unorm,
                        view_dimension: wgpu::TextureViewDimension::D2,
                    })
                    .compute(),
                // debug
                render::BindGroupLayoutEntry::new().uniform().compute(),
            ])
            .build(ctx);

        let pipeline_layout = render::PipelineLayoutBuilder::new()
            .bind_groups(vec![bindgroup_layout.clone()])
            .build(ctx);

        let pipeline = render::ComputePipelineBuilder::new(shader, pipeline_layout).build(ctx);

        Self {
            pipeline,
            bindgroup_layout,
            debug_input,
        }
    }

    pub fn apply_filter(
        &mut self,
        ctx: &mut Context,
        in_texture: render::ArcTextureView,
        out_texture: render::ArcTextureView,
        width: u32,
        height: u32,
        // in_texture: &render::FrameBuffer,
        // out_texture: &render::FrameBuffer,
    ) {
        let mut encoder = render::EncoderBuilder::new().build(ctx);
        // if in_texture.texture().width() != out_texture.texture().width()
        //     || in_texture.texture().height() != out_texture.texture().height()
        // {
        //     log::error!("in and out texture of box blur must have same size");
        // }

        self.debug_input.update_buffer(ctx);
        let bindgroup = render::BindGroupBuilder::new(self.bindgroup_layout.clone())
            .entries(vec![
                // in
                render::BindGroupEntry::Texture(in_texture),
                // out
                render::BindGroupEntry::Texture(out_texture),
                // debug
                render::BindGroupEntry::Buffer(self.debug_input.buffer()),
            ])
            .build(ctx);

        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: None,
            timestamp_writes: None,
        });

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &bindgroup, &[]);
        pass.dispatch_workgroups(width, height, 1);

        drop(pass);

        let queue = render::queue(ctx);
        queue.submit(Some(encoder.finish()));
    }
}
