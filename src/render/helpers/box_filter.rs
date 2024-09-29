use encase::ShaderType;

use crate::{
    filesystem,
    render::{self},
    Context,
};

pub struct BoxFilter {
    pipeline: render::ArcComputePipeline,
    bindgroup_layout: render::ArcBindGroupLayout,
    debug_input: render::DebugInput,
    params_buffer: render::UniformBuffer,

    copy_texture: render::FrameBuffer,
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
                // params
                render::BindGroupLayoutEntry::new().uniform().compute(),
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
            .format(wgpu::TextureFormat::Rgba8Unorm)
            .usage(wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING)
            .build(ctx);

        let params_buffer =
            render::UniformBufferBuilder::new().build(ctx, BoxFilterParams::min_size());

        Self {
            pipeline,
            bindgroup_layout,
            debug_input,
            copy_texture,
            params_buffer,
        }
    }

    /// Applies box filter to the specificed texture
    ///
    /// NOTE, it overrides the texture
    pub fn apply_filter(
        &mut self,
        ctx: &mut Context,
        texture: &render::FrameBuffer,
        params: &BoxFilterParams,
    ) {
        let width = texture.texture().width();
        let height = texture.texture().height();

        // Update buffers
        self.debug_input.update_buffer(ctx);
        self.params_buffer.write(ctx, &params);
        let mut encoder = render::EncoderBuilder::new().build(ctx);

        if texture.texture().size() != self.copy_texture.texture().size() {
            log::warn!("in and out texture of box blur must have same size");
            self.copy_texture.resize(ctx, width, height);
        }

        // Copy current texture to copy texture
        encoder.copy_texture_to_texture(
            wgpu::ImageCopyTextureBase {
                texture: &texture.texture(),
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
            texture.texture().size(),
        );

        // Run box filter compute shader
        let bindgroup = render::BindGroupBuilder::new(self.bindgroup_layout.clone())
            .entries(vec![
                // in
                render::BindGroupEntry::Texture(self.copy_texture.view()),
                // out
                render::BindGroupEntry::Texture(texture.view()),
                // Params
                render::BindGroupEntry::Buffer(self.params_buffer.buffer()),
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

#[derive(ShaderType)]
pub struct BoxFilterParams {
    kernel_size: i32,
}

impl BoxFilterParams {
    pub fn new(kernel_size: i32) -> Self {
        Self { kernel_size }
    }
}
