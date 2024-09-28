use crate::{render, Context};

#[derive(Clone)]
pub struct FrameBufferBuilder {
    label: Option<String>,
    usage: wgpu::TextureUsages,
    format: wgpu::TextureFormat,
    size: wgpu::Extent3d,
    view_formats: Vec<wgpu::TextureFormat>,
}

impl FrameBufferBuilder {
    pub fn new() -> Self {
        Self {
            label: None,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC,
            format: wgpu::TextureFormat::Bgra8UnormSrgb, // TODO default to BRGA instead?
            size: wgpu::Extent3d {
                width: 0,
                height: 0,
                depth_or_array_layers: 0,
            },
            view_formats: Vec::new(),
        }
    }
    pub fn build(&self, ctx: &Context) -> FrameBuffer {
        let device = render::device(ctx);
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: self.label.as_deref(),
            size: self.size,
            format: self.format,
            usage: self.usage,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            view_formats: &self.view_formats,
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
        });
        FrameBuffer {
            texture: render::ArcTexture::new(texture),
            view: render::ArcTextureView::new(view),
            format: self.format,
            builder: self.clone(),
        }
    }

    pub fn label(&mut self, label: &str) -> &mut Self {
        self.label = Some(label.to_string());
        self
    }
    pub fn format(&mut self, format: wgpu::TextureFormat) -> &mut Self {
        self.format = format;
        self
    }
    pub fn usage(&mut self, usage: wgpu::TextureUsages) -> &mut Self {
        self.usage = usage;
        self
    }
    pub fn view(&mut self, usage: wgpu::TextureUsages) -> &mut Self {
        self.usage = usage;
        self
    }
    pub fn view_formats(&mut self, view_formats: Vec<wgpu::TextureFormat>) -> &mut Self {
        self.view_formats = view_formats;
        self
    }
    pub fn size(&mut self, width: u32, height: u32) -> &mut Self {
        self.size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        self
    }
    pub fn screen_size(&mut self, ctx: &Context) -> &mut Self {
        let surface_conf = render::surface_config(ctx);
        self.size(surface_conf.width, surface_conf.height)
    }
}

pub struct FrameBuffer {
    texture: render::ArcTexture,
    view: render::ArcTextureView,
    format: wgpu::TextureFormat,
    builder: FrameBufferBuilder,
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
    pub fn target(&self) -> wgpu::ColorTargetState {
        wgpu::ColorTargetState {
            format: self.format,
            write_mask: wgpu::ColorWrites::ALL,
            blend: None,
        }
    }
    pub fn resize(&mut self, ctx: &Context, width: u32, height: u32) {
        *self = self.builder.size(width, height).build(ctx);
    }
    pub fn resize_screen(&mut self, ctx: &Context) {
        *self = self.builder.screen_size(ctx).build(ctx);
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
        let mut framebuffer_builder = FrameBufferBuilder::new();
        framebuffer_builder.format(wgpu::TextureFormat::Depth32Float);
        Self {
            framebuffer_builder,
            depth_compare: wgpu::CompareFunction::Less,
            depth_write_enabled: true,
        }
    }
    pub fn build(&mut self, ctx: &Context) -> DepthBuffer {
        let framebuffer = self.framebuffer_builder.build(ctx);
        DepthBuffer {
            framebuffer,
            depth_compare: self.depth_compare,
            depth_write_enabled: self.depth_write_enabled,
        }
    }

    pub fn label(&mut self, label: &str) -> &mut Self {
        self.framebuffer_builder.label = Some(label.to_string());
        self
    }
    pub fn usage(&mut self, usage: wgpu::TextureUsages) -> &mut Self {
        self.framebuffer_builder.usage = usage;
        self
    }
    pub fn size(&mut self, width: u32, height: u32) -> &mut Self {
        self.framebuffer_builder.size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        self
    }
    pub fn screen_size(&mut self, ctx: &Context) -> &mut Self {
        let surface_conf = render::surface_config(ctx);
        self.size(surface_conf.width, surface_conf.height)
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
            format: self.framebuffer.format,
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
    pub fn resize(&mut self, ctx: &Context) {
        self.framebuffer.resize_screen(ctx);
    }
}
