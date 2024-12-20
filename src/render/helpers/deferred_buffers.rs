use crate::{render, Context};

pub struct DeferredBuffers {
    pub position: render::FrameBuffer,
    pub albedo: render::FrameBuffer,
    pub normal: render::FrameBuffer,
    pub roughness: render::FrameBuffer,
    pub depth: render::FrameBuffer,
}

impl DeferredBuffers {
    const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

    // TODO should all of these be 32float?
    pub fn new(ctx: &Context) -> Self {
        let position_buffer = render::FrameBufferBuilder::new()
            .screen_size(ctx)
            .format(wgpu::TextureFormat::Rgba16Float)
            .build(ctx);
        let albedo_buffer = render::FrameBufferBuilder::new()
            .screen_size(ctx)
            .format(wgpu::TextureFormat::Rgba8Unorm)
            .build(ctx);
        let normal_buffer = render::FrameBufferBuilder::new()
            .screen_size(ctx)
            .format(wgpu::TextureFormat::Rgba16Float)
            .build(ctx);
        let roughness_buffer = render::FrameBufferBuilder::new()
            .screen_size(ctx)
            .format(wgpu::TextureFormat::Rgba8Unorm)
            .build(ctx);
        let depth_buffer = render::FrameBufferBuilder::new()
            .screen_size(ctx)
            .format(Self::DEPTH_FORMAT)
            .build(ctx);
        Self {
            position: position_buffer,
            albedo: albedo_buffer,
            normal: normal_buffer,
            roughness: roughness_buffer,
            depth: depth_buffer,
        }
    }

    /// Depth stencil attachment (clear) for depth buffer
    pub fn depth_stencil_attachment_clear(&self) -> wgpu::RenderPassDepthStencilAttachment<'_> {
        wgpu::RenderPassDepthStencilAttachment {
            view: self.depth.view_ref(),
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Clear(1.0),
                store: wgpu::StoreOp::Store,
            }),
            stencil_ops: None,
        }
    }

    /// Depth stencil attachment (load) for depth buffer
    pub fn depth_stencil_attachment_load(&self) -> wgpu::RenderPassDepthStencilAttachment<'_> {
        wgpu::RenderPassDepthStencilAttachment {
            view: self.depth.view_ref(),
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: wgpu::StoreOp::Store,
            }),
            stencil_ops: None,
        }
    }

    /// Depth stencil state for depth buffer
    pub fn depth_stencil_state(&self) -> wgpu::DepthStencilState {
        wgpu::DepthStencilState {
            format: Self::DEPTH_FORMAT,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            bias: wgpu::DepthBiasState::default(),
            stencil: wgpu::StencilState::default(),
        }
    }

    /// Target including
    /// * Position
    /// * Albedo
    /// * Normal
    /// * Roughness
    pub fn targets(&self) -> [Option<wgpu::ColorTargetState>; 4] {
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
    pub fn clear(&self, ctx: &Context) {
        let queue = render::queue(ctx);
        let mut encoder = render::EncoderBuilder::new().build(ctx);
        let attachments = &self.color_attachments_clear();
        let pass = render::RenderPassBuilder::new()
            .color_attachments(attachments)
            .depth_stencil_attachment(self.depth_stencil_attachment_clear())
            .build(&mut encoder);
        drop(pass);
        queue.submit(Some(encoder.finish()));
    }

    // TODO add loadop option
    /// Color attachments for (load)
    /// * Position
    /// * Albedo
    /// * Normal
    /// * Roughness
    pub fn color_attachments(&self) -> [Option<wgpu::RenderPassColorAttachment<'_>>; 4] {
        [
            Some(wgpu::RenderPassColorAttachment {
                view: self.position.view_ref(),
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
                resolve_target: None,
            }),
            Some(wgpu::RenderPassColorAttachment {
                view: self.albedo.view_ref(),
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
                resolve_target: None,
            }),
            Some(wgpu::RenderPassColorAttachment {
                view: self.normal.view_ref(),
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
                resolve_target: None,
            }),
            Some(wgpu::RenderPassColorAttachment {
                view: self.roughness.view_ref(),
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
                resolve_target: None,
            }),
        ]
    }

    /// Color attachments for (clear)
    /// * Position
    /// * Albedo
    /// * Normal
    /// * Roughness
    pub fn color_attachments_clear(&self) -> [Option<wgpu::RenderPassColorAttachment<'_>>; 4] {
        const CLEAR_COLOR: wgpu::Color = wgpu::Color::BLACK;
        [
            Some(wgpu::RenderPassColorAttachment {
                view: self.position.view_ref(),
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(CLEAR_COLOR),
                    store: wgpu::StoreOp::Store,
                },
                resolve_target: None,
            }),
            Some(wgpu::RenderPassColorAttachment {
                view: self.albedo.view_ref(),
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(CLEAR_COLOR),
                    store: wgpu::StoreOp::Store,
                },
                resolve_target: None,
            }),
            Some(wgpu::RenderPassColorAttachment {
                view: self.normal.view_ref(),
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(CLEAR_COLOR),
                    store: wgpu::StoreOp::Store,
                },
                resolve_target: None,
            }),
            Some(wgpu::RenderPassColorAttachment {
                view: self.roughness.view_ref(),
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(CLEAR_COLOR),
                    store: wgpu::StoreOp::Store,
                },
                resolve_target: None,
            }),
        ]
    }

    /// Resizes
    /// * Position
    /// * Albedo
    /// * Normal
    /// * Roughness
    /// * Depth
    pub fn resize(&mut self, ctx: &Context, width: u32, height: u32) {
        self.position.resize(ctx, width, height);
        self.albedo.resize(ctx, width, height);
        self.normal.resize(ctx, width, height);
        self.roughness.resize(ctx, width, height);
        self.depth.resize(ctx, width, height);
    }

    /// Resizes using current screen dimensions
    /// * Position
    /// * Albedo
    /// * Normal
    /// * Roughness
    /// * Depth
    pub fn resize_screen(&mut self, ctx: &Context) {
        self.position.resize_screen(ctx);
        self.albedo.resize_screen(ctx);
        self.normal.resize_screen(ctx);
        self.roughness.resize_screen(ctx);
        self.depth.resize_screen(ctx);
    }
}
