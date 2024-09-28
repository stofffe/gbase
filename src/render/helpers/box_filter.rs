use crate::{
    filesystem,
    render::{self},
    Context,
};

pub struct BoxFilter {
    pipeline: render::ArcComputePipeline,
    bindgroup_layout: render::ArcBindGroupLayout,
    debug_input: render::DebugInput,

    pub copy_texture: render::FrameBuffer,
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

        let copy_texture = render::FrameBufferBuilder::new()
            .screen_size(ctx)
            .format(wgpu::TextureFormat::Rgba8UnormSrgb)
            .usage(wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING)
            .build(ctx);

        Self {
            pipeline,
            bindgroup_layout,
            debug_input,
            copy_texture,
        }
    }

    pub fn apply_filter(&mut self, ctx: &mut Context, in_texture: &render::FrameBuffer) {
        // Update buffers
        self.debug_input.update_buffer(ctx);
        let mut encoder = render::EncoderBuilder::new().build(ctx);

        if in_texture.texture().size() != self.copy_texture.texture().size() {
            log::error!("in and out texture of box blur must have same size");
        }

        // Copy current texture to copy texture
        let width = in_texture.texture().width();
        let height = in_texture.texture().height();
        self.copy_texture.resize(ctx, width, height);
        encoder.copy_texture_to_texture(
            wgpu::ImageCopyTextureBase {
                texture: &in_texture.texture(),
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::ImageCopyTextureBase {
                texture: &self.copy_texture.texture(),
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            in_texture.texture().size(),
        );

        let bindgroup = render::BindGroupBuilder::new(self.bindgroup_layout.clone())
            .entries(vec![
                // in
                render::BindGroupEntry::Texture(self.copy_texture.view()),
                // out
                render::BindGroupEntry::Texture(in_texture.view()),
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
