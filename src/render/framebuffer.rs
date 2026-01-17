use super::ArcHandle;
use crate::{
    render::{self, next_id},
    Context,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameBufferBuilder {
    label: Option<String>,
    usage: wgpu::TextureUsages,
    format: wgpu::TextureFormat,
    size: wgpu::Extent3d,
    mip_level_count: u32,
}

impl FrameBufferBuilder {
    pub fn new() -> Self {
        Self {
            label: None,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            format: wgpu::TextureFormat::Rgba8Unorm, // TODO default to BRGA instead?
            size: wgpu::Extent3d {
                width: 0,
                height: 0,
                depth_or_array_layers: 0,
            },
            mip_level_count: 1,
        }
    }
    pub fn build(self, ctx: &mut Context) -> FrameBuffer {
        let device = render::device(ctx);
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: self.label.as_deref(),
            size: self.size,
            format: self.format,
            usage: self.usage,
            mip_level_count: self.mip_level_count,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: self.label.as_deref(),
            aspect: wgpu::TextureAspect::All,
            format: None,
            dimension: None,
            mip_level_count: None,
            array_layer_count: None,
            base_mip_level: 0,
            base_array_layer: 0,
            usage: None,
        });
        FrameBuffer {
            label: self.label,
            texture: render::ArcTexture::new(ctx, texture),
            view: render::ArcTextureView::new(ctx, view),
        }
    }

    pub fn label(mut self, label: &str) -> Self {
        self.label = Some(label.to_string());
        self
    }
    pub fn format(mut self, format: wgpu::TextureFormat) -> Self {
        self.format = format;
        self
    }
    pub fn usage(mut self, usage: wgpu::TextureUsages) -> Self {
        self.usage = usage;
        self
    }
    pub fn mip_level_count(mut self, mip_level_count: u32) -> Self {
        self.mip_level_count = mip_level_count;
        self
    }
    pub fn size(mut self, width: u32, height: u32) -> Self {
        self.size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        self
    }
    pub fn screen_size(self, ctx: &Context) -> Self {
        let surface_conf = render::surface_config(ctx);
        self.size(surface_conf.width, surface_conf.height)
    }
}

// TODO: sampler also?
#[derive(Debug)]
pub struct FrameBuffer {
    label: Option<String>,
    texture: render::ArcTexture,
    view: render::ArcTextureView,
}

impl FrameBuffer {
    pub fn texture(&self) -> render::ArcTexture {
        self.texture.clone()
    }
    pub fn view(&self) -> render::ArcTextureView {
        self.view.clone()
    }
    pub fn texture_ref(&self) -> &wgpu::Texture {
        &self.texture
    }
    pub fn view_ref(&self) -> &wgpu::TextureView {
        &self.view
    }
    pub fn target_blend(&self, blend: wgpu::BlendState) -> render::ColorTargetState {
        render::ColorTargetState::new()
            .format(self.format())
            .blend(blend)
    }
    pub fn target(&self) -> render::ColorTargetState {
        render::ColorTargetState::new().format(self.format())
    }
    pub fn attachment(&self) -> wgpu::RenderPassColorAttachment<'_> {
        wgpu::RenderPassColorAttachment {
            view: self.view_ref(),
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: wgpu::StoreOp::Store,
            },
        }
    }
    pub fn resize(&mut self, ctx: &mut Context, new_size: winit::dpi::PhysicalSize<u32>) {
        if self.size() == new_size {
            return;
        }
        self.texture.destroy(); // TODO: needed?

        let device = render::device(ctx);
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: self.label.as_deref(), // TODO:
            size: wgpu::Extent3d {
                width: new_size.width,
                height: new_size.height,
                depth_or_array_layers: self.texture().depth_or_array_layers(),
            },
            format: self.texture().format(),
            usage: self.texture().usage(),
            mip_level_count: self.texture().mip_level_count(),
            sample_count: self.texture().sample_count(),
            dimension: self.texture().dimension(),
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: self.label.as_deref(),
            aspect: wgpu::TextureAspect::All,
            format: None,
            dimension: None,
            mip_level_count: None,
            array_layer_count: None,
            base_mip_level: 0,
            base_array_layer: 0,
            usage: None,
        });
        *self = FrameBuffer {
            label: self.label.clone(),
            texture: ArcHandle::new(ctx, texture),
            view: ArcHandle::new(ctx, view),
        }
    }
    pub fn format(&self) -> wgpu::TextureFormat {
        self.texture().format()
    }
    pub fn width(&self) -> u32 {
        self.texture.width()
    }
    pub fn height(&self) -> u32 {
        self.texture.height()
    }
    pub fn size(&self) -> winit::dpi::PhysicalSize<u32> {
        winit::dpi::PhysicalSize::new(self.width(), self.height())
    }

    pub fn clear(&self, ctx: &mut Context, color: wgpu::Color) {
        let mut encoder = render::EncoderBuilder::new().build(ctx);
        render::RenderPassBuilder::new()
            .color_attachments(&[Some(
                render::RenderPassColorAttachment::new(self.view_ref()).clear(color),
            )])
            .build(ctx, &mut encoder);
        render::queue(ctx).submit(Some(encoder.finish()));
    }
}

//
// Depth
//

pub struct DepthBufferBuilder {
    framebuffer_builder: FrameBufferBuilder,
    depth_compare: wgpu::CompareFunction,
    depth_write_enabled: bool,
}

impl DepthBufferBuilder {
    pub fn new() -> Self {
        let framebuffer_builder =
            FrameBufferBuilder::new().format(wgpu::TextureFormat::Depth32Float);
        Self {
            framebuffer_builder,
            depth_compare: wgpu::CompareFunction::Less,
            depth_write_enabled: true,
        }
    }
    pub fn build(self, ctx: &mut Context) -> DepthBuffer {
        let framebuffer = self.framebuffer_builder.clone().build(ctx);
        DepthBuffer {
            framebuffer,
            depth_compare: self.depth_compare,
            depth_write_enabled: self.depth_write_enabled,
        }
    }

    pub fn label(mut self, label: &str) -> Self {
        self.framebuffer_builder.label = Some(label.to_string());
        self
    }
    pub fn usage(mut self, usage: wgpu::TextureUsages) -> Self {
        self.framebuffer_builder.usage = usage;
        self
    }
    pub fn depth_write_enabled(mut self, value: bool) -> Self {
        self.depth_write_enabled = value;
        self
    }
    pub fn depth_compare(mut self, value: wgpu::CompareFunction) -> Self {
        self.depth_compare = value;
        self
    }
    pub fn size(mut self, width: u32, height: u32) -> Self {
        self.framebuffer_builder.size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        self
    }
    pub fn screen_size(self, ctx: &Context) -> Self {
        let surface_conf = render::surface_config(ctx);
        self.size(surface_conf.width, surface_conf.height)
    }
    pub fn format(mut self, format: wgpu::TextureFormat) -> Self {
        self.framebuffer_builder.format = format;
        self
    }
}

pub struct DepthBuffer {
    framebuffer: FrameBuffer,
    depth_compare: wgpu::CompareFunction,
    depth_write_enabled: bool,
}

impl DepthBuffer {
    pub fn depth_stencil_state(&self) -> wgpu::DepthStencilState {
        wgpu::DepthStencilState {
            format: self.framebuffer.format(),
            depth_write_enabled: self.depth_write_enabled,
            depth_compare: self.depth_compare,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }
    }
    pub fn depth_render_attachment_load(&self) -> wgpu::RenderPassDepthStencilAttachment<'_> {
        wgpu::RenderPassDepthStencilAttachment {
            view: self.framebuffer.view_ref(),
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: wgpu::StoreOp::Store,
            }),
            stencil_ops: None,
        }
    }
    pub fn depth_render_attachment_clear(&self) -> wgpu::RenderPassDepthStencilAttachment<'_> {
        wgpu::RenderPassDepthStencilAttachment {
            view: self.framebuffer.view_ref(),
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Clear(1.0),
                store: wgpu::StoreOp::Store,
            }),
            stencil_ops: None,
        }
    }
    pub fn framebuffer(&self) -> &FrameBuffer {
        &self.framebuffer
    }
    pub fn resize(&mut self, ctx: &mut Context, new_size: winit::dpi::PhysicalSize<u32>) {
        self.framebuffer.resize(ctx, new_size);
    }
    pub fn clear(&mut self, ctx: &mut Context) {
        let mut encoder = render::EncoderBuilder::new().build(ctx);
        render::RenderPassBuilder::new()
            .depth_stencil_attachment(self.depth_render_attachment_clear())
            .build(ctx, &mut encoder);
        render::queue(ctx).submit(Some(encoder.finish()));
    }
}
