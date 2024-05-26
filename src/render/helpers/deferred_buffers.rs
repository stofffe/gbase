use crate::{render, Context};

pub struct DeferredBuffers {
    pub position: render::ResizableFrameBuffer,
    pub albedo: render::ResizableFrameBuffer,
    pub normal: render::ResizableFrameBuffer,
    pub roughness: render::ResizableFrameBuffer,
    pub depth: render::ResizableFrameBuffer,
}

impl DeferredBuffers {
    pub fn new(ctx: &Context) -> Self {
        let position_buffer = render::FrameBufferBuilder::new()
            .screen_size(ctx)
            .format(wgpu::TextureFormat::Rgba32Float)
            .build_resizable(ctx);
        let albedo_buffer = render::FrameBufferBuilder::new()
            .screen_size(ctx)
            .format(wgpu::TextureFormat::Rgba32Float)
            .build_resizable(ctx);
        let normal_buffer = render::FrameBufferBuilder::new()
            .screen_size(ctx)
            .format(wgpu::TextureFormat::Rgba32Float)
            .build_resizable(ctx);
        let roughness_buffer = render::FrameBufferBuilder::new()
            .screen_size(ctx)
            .format(wgpu::TextureFormat::Rgba32Float)
            .build_resizable(ctx);
        let depth_buffer = render::FrameBufferBuilder::new()
            .screen_size(ctx)
            .format(wgpu::TextureFormat::Depth32Float)
            .build_resizable(ctx);
        Self {
            position: position_buffer,
            albedo: albedo_buffer,
            normal: normal_buffer,
            roughness: roughness_buffer,
            depth: depth_buffer,
        }
    }

    pub fn depth_stencil_attachment_clear(&self) -> wgpu::RenderPassDepthStencilAttachment<'_> {
        wgpu::RenderPassDepthStencilAttachment {
            view: self.depth.view(),
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Clear(1.0),
                store: wgpu::StoreOp::Store,
            }),
            stencil_ops: None,
        }
    }

    pub fn depth_stencil_attachment_load(&self) -> wgpu::RenderPassDepthStencilAttachment<'_> {
        wgpu::RenderPassDepthStencilAttachment {
            view: self.depth.view(),
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: wgpu::StoreOp::Store,
            }),
            stencil_ops: None,
        }
    }

    pub fn depth_stencil_state(&self) -> wgpu::DepthStencilState {
        wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            bias: wgpu::DepthBiasState::default(),
            stencil: wgpu::StencilState::default(),
        }
    }

    pub fn targets(&self) -> [Option<wgpu::ColorTargetState>; 4] {
        [
            Some(self.position.target()),
            Some(self.albedo.target()),
            Some(self.normal.target()),
            Some(self.roughness.target()),
        ]
    }

    // TODO add loadop option
    pub fn color_attachments(&self) -> [Option<wgpu::RenderPassColorAttachment<'_>>; 4] {
        [
            Some(wgpu::RenderPassColorAttachment {
                view: self.position.view(),
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
                resolve_target: None,
            }),
            Some(wgpu::RenderPassColorAttachment {
                view: self.albedo.view(),
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
                resolve_target: None,
            }),
            Some(wgpu::RenderPassColorAttachment {
                view: self.normal.view(),
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
                resolve_target: None,
            }),
            Some(wgpu::RenderPassColorAttachment {
                view: self.roughness.view(),
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
                resolve_target: None,
            }),
        ]
    }

    pub fn resize(&mut self, ctx: &Context) {
        self.position.resize(ctx);
        self.albedo.resize(ctx);
        self.normal.resize(ctx);
        self.roughness.resize(ctx);
        self.depth.resize(ctx);
    }
}
