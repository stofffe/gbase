use encase::ShaderType;

use gbase::{
    render::{self, FrameBufferBuilder},
    tracing, wgpu, Context,
};

pub struct GaussianFilter {
    horizontal_pipeline: render::ArcComputePipeline,
    vertical_pipeline: render::ArcComputePipeline,

    bindgroup_layout: render::ArcBindGroupLayout,
    params_buffer: render::UniformBuffer<GaussianFilterParams>,

    copy_texture: render::FrameBuffer,
}

impl GaussianFilter {
    pub fn new(ctx: &mut Context) -> Self {
        let shader = render::ShaderBuilder::new(
            include_str!("../../assets/shaders/gaussian_filter.wgsl").to_string(),
        )
        .build(ctx);

        let bindgroup_layout = render::BindGroupLayoutBuilder::new()
            .entries(vec![
                // in
                render::BindGroupLayoutEntry::new()
                    .texture_float_filterable()
                    .compute(),
                // out
                render::BindGroupLayoutEntry::new()
                    .storage_texture_2d_write(wgpu::TextureFormat::Rgba8Unorm)
                    .compute(),
                // params
                render::BindGroupLayoutEntry::new().uniform().compute(),
            ])
            .build(ctx);

        let pipeline_layout = render::PipelineLayoutBuilder::new()
            .bind_groups(vec![bindgroup_layout.clone()])
            .build(ctx);

        let horizontal_pipeline =
            render::ComputePipelineBuilder::new(shader.clone(), pipeline_layout.clone())
                .entry_point("horizontal")
                .build(ctx);
        let vertical_pipeline = render::ComputePipelineBuilder::new(shader, pipeline_layout)
            .entry_point("vertical")
            .build(ctx);

        let copy_texture = FrameBufferBuilder::new()
            .screen_size(ctx)
            .usage(wgpu::TextureUsages::STORAGE_BINDING | wgpu::TextureUsages::TEXTURE_BINDING)
            .format(wgpu::TextureFormat::Rgba8Unorm)
            .build(ctx);

        let params_buffer = render::UniformBufferBuilder::new().build(ctx);

        Self {
            horizontal_pipeline,
            vertical_pipeline,
            bindgroup_layout,
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
        framebuffer: &render::FrameBuffer,
        params: &GaussianFilterParams,
    ) {
        assert!(
            self.copy_texture.format() == framebuffer.format(),
            "frambuffer sent to gaussian filter must be RGBA8UNORM"
        );

        let width = framebuffer.texture().width();
        let height = framebuffer.texture().height();

        // Update buffers
        self.params_buffer.write(ctx, params);
        let mut encoder = render::EncoderBuilder::new().build(ctx);

        // Recreate copy buffer if necessary
        if framebuffer.texture().size() != self.copy_texture.texture().size() {
            tracing::warn!("in and out texture of gaussian blur must have same size and format");
            self.copy_texture
                .resize(ctx, gbase::winit::dpi::PhysicalSize::new(width, height));
        }

        // Create bindgroups
        let horizontal_bindgroup = render::BindGroupBuilder::new(self.bindgroup_layout.clone())
            .entries(vec![
                render::BindGroupEntry::Texture(framebuffer.view()), // in
                render::BindGroupEntry::Texture(self.copy_texture.view()), // out
                render::BindGroupEntry::Buffer(self.params_buffer.buffer()), // params
            ])
            .build(ctx);
        let vertical_bindgroup = render::BindGroupBuilder::new(self.bindgroup_layout.clone())
            .entries(vec![
                render::BindGroupEntry::Texture(self.copy_texture.view()), // in
                render::BindGroupEntry::Texture(framebuffer.view()),       // out
                render::BindGroupEntry::Buffer(self.params_buffer.buffer()), // params
            ])
            .build(ctx);

        // Run shader
        render::ComputePassBuilder::new().build_run(&mut encoder, |mut pass| {
            // horizontal
            pass.set_pipeline(&self.horizontal_pipeline);
            pass.set_bind_group(0, Some(horizontal_bindgroup.as_ref()), &[]);
            pass.dispatch_workgroups(width, height, 1);

            // vertical
            pass.set_pipeline(&self.vertical_pipeline);
            pass.set_bind_group(0, Some(vertical_bindgroup.as_ref()), &[]);
            pass.dispatch_workgroups(width, height, 1);
        });

        let queue = render::queue(ctx);
        queue.submit(Some(encoder.finish()));
    }
}

#[derive(ShaderType)]
pub struct GaussianFilterParams {
    kernel_size: i32,
    sigma: f32,
}

impl GaussianFilterParams {
    pub fn new(kernel_size: i32, sigma: f32) -> Self {
        Self { kernel_size, sigma }
    }
}
