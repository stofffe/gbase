use gbase::{render, wgpu, winit, Context};

pub struct DeferredBuffers {
    pub position: render::FrameBuffer,
    pub albedo: render::FrameBuffer,
    pub normal: render::FrameBuffer,
    pub roughness: render::FrameBuffer,
    pub depth: render::DepthBuffer,
}

impl DeferredBuffers {
    // TODO: go over formats
    pub fn new(ctx: &Context) -> Self {
        let position = render::FrameBufferBuilder::new()
            .screen_size(ctx)
            .format(wgpu::TextureFormat::Rgba16Float)
            .build(ctx);
        let albedo = render::FrameBufferBuilder::new()
            .screen_size(ctx)
            .format(wgpu::TextureFormat::Rgba8Unorm)
            .build(ctx);
        let normal = render::FrameBufferBuilder::new()
            .screen_size(ctx)
            .format(wgpu::TextureFormat::Rgba16Float)
            .build(ctx);
        let roughness = render::FrameBufferBuilder::new()
            .screen_size(ctx)
            .format(wgpu::TextureFormat::Rgba8Unorm)
            .build(ctx);
        let depth = render::DepthBufferBuilder::new()
            .screen_size(ctx)
            .build(ctx);
        Self {
            position,
            albedo,
            normal,
            roughness,
            depth,
        }
    }

    // /// Depth stencil attachment (clear) for depth buffer
    // pub fn depth_stencil_attachment_clear(&self) -> wgpu::RenderPassDepthStencilAttachment<'_> {
    //     wgpu::RenderPassDepthStencilAttachment {
    //         view: self.depth.view_ref(),
    //         depth_ops: Some(wgpu::Operations {
    //             load: wgpu::LoadOp::Clear(1.0),
    //             store: wgpu::StoreOp::Store,
    //         }),
    //         stencil_ops: None,
    //     }
    // }
    //
    // /// Depth stencil attachment (load) for depth buffer
    // pub fn depth_stencil_attachment_load(&self) -> wgpu::RenderPassDepthStencilAttachment<'_> {
    //     wgpu::RenderPassDepthStencilAttachment {
    //         view: self.depth.view_ref(),
    //         depth_ops: Some(wgpu::Operations {
    //             load: wgpu::LoadOp::Load,
    //             store: wgpu::StoreOp::Store,
    //         }),
    //         stencil_ops: None,
    //     }
    // }
    //
    // /// Depth stencil state for depth buffer
    // pub fn depth_stencil_state(&self) -> wgpu::DepthStencilState {
    //     wgpu::DepthStencilState {
    //         format: Self::DEPTH_FORMAT,
    //         depth_write_enabled: true,
    //         depth_compare: wgpu::CompareFunction::Less,
    //         bias: wgpu::DepthBiasState::default(),
    //         stencil: wgpu::StencilState::default(),
    //     }
    // }

    /// Target including
    /// * Position
    /// * Albedo
    /// * Normal
    /// * Roughness
    pub fn targets(&self) -> [Option<render::ColorTargetState>; 4] {
        [
            Some(self.position.target()),
            Some(self.albedo.target()),
            Some(self.normal.target()),
            Some(self.roughness.target()),
        ]
    }

    /// Clear buffers
    ///
    /// Usually called at start of frame
    pub fn clear(&self, ctx: &mut Context) {
        let mut encoder = render::EncoderBuilder::new().build(ctx);
        render::RenderPassBuilder::new()
            .color_attachments(&self.color_attachments_clear())
            .depth_stencil_attachment(self.depth.depth_render_attachment_clear())
            .build(ctx, &mut encoder);
        render::queue(ctx).submit(Some(encoder.finish()));
    }

    // TODO add loadop option
    /// Color attachments for (load)
    /// * Position
    /// * Albedo
    /// * Normal
    /// * Roughness
    pub fn color_attachments(&self) -> [Option<render::RenderPassColorAttachment<'_>>; 4] {
        [
            Some(render::RenderPassColorAttachment::new(
                self.position.view_ref(),
            )),
            Some(render::RenderPassColorAttachment::new(
                self.albedo.view_ref(),
            )),
            Some(render::RenderPassColorAttachment::new(
                self.normal.view_ref(),
            )),
            Some(render::RenderPassColorAttachment::new(
                self.roughness.view_ref(),
            )),
        ]
    }

    /// Color attachments for (clear)
    /// * Position
    /// * Albedo
    /// * Normal
    /// * Roughness
    pub fn color_attachments_clear(&self) -> [Option<render::RenderPassColorAttachment<'_>>; 4] {
        const COLOR: wgpu::Color = wgpu::Color::BLACK;
        [
            Some(render::RenderPassColorAttachment::new(self.position.view_ref()).clear(COLOR)),
            Some(render::RenderPassColorAttachment::new(self.albedo.view_ref()).clear(COLOR)),
            Some(render::RenderPassColorAttachment::new(self.normal.view_ref()).clear(COLOR)),
            Some(render::RenderPassColorAttachment::new(self.roughness.view_ref()).clear(COLOR)),
        ]
    }

    /// Resizes
    /// * Position
    /// * Albedo
    /// * Normal
    /// * Roughness
    /// * Depth
    pub fn resize(&mut self, ctx: &Context, new_size: winit::dpi::PhysicalSize<u32>) {
        self.position.resize(ctx, new_size);
        self.albedo.resize(ctx, new_size);
        self.normal.resize(ctx, new_size);
        self.roughness.resize(ctx, new_size);
        self.depth.resize(ctx, new_size);
    }
}
