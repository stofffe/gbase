mod arc;
mod bind_group;
mod buffer;
mod cache;
mod framebuffer;
mod mesh;
mod pipeline;
mod render_pass;
mod shader;
mod texture;
mod vertex;
pub use arc::*;
pub use bind_group::*;
pub use buffer::*;
pub use cache::*;
pub use framebuffer::*;
pub use mesh::*;
pub use pipeline::*;
pub use render_pass::*;
pub use shader::*;
pub use texture::*;
pub use vertex::*;

use crate::Context;
use std::sync::Arc;

pub(crate) struct RenderContext {
    surface: Arc<wgpu::Surface<'static>>,
    device: Arc<wgpu::Device>,
    adapter: Arc<wgpu::Adapter>,
    queue: Arc<wgpu::Queue>,
    surface_config: wgpu::SurfaceConfiguration,

    window: Arc<winit::window::Window>,
    window_size: winit::dpi::PhysicalSize<u32>,

    cache: RenderCache,
}

impl RenderContext {
    pub(crate) async fn new(
        window: winit::window::Window,
        vsync: bool,
        device_features: wgpu::Features,
    ) -> Self {
        let window = Arc::new(window);

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            flags: wgpu::InstanceFlags::default(),
            backend_options: wgpu::BackendOptions {
                gl: wgpu::GlBackendOptions::default(),
                dx12: wgpu::Dx12BackendOptions::default(),
            },
        });

        let surface = instance
            .create_surface(window.clone())
            .expect("could not create surface");

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptionsBase {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("could not create adapter");

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: device_features,
                    required_limits: adapter.limits(),
                    label: None,
                    memory_hints: wgpu::MemoryHints::Performance,
                },
                None,
            )
            .await
            .expect("could not get device");

        let surface_capabilities = surface.get_capabilities(&adapter);
        let surface_format = surface_capabilities
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_capabilities.formats[0]);
        log::error!("surface format {:?}", surface_format);
        let window_size = window.inner_size();

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: window_size.width.max(1),
            height: window_size.height.max(1),
            present_mode: if vsync {
                wgpu::PresentMode::AutoVsync
            } else {
                wgpu::PresentMode::AutoNoVsync
            },
            alpha_mode: surface_capabilities.alpha_modes[0],
            view_formats: vec![
                surface_format.remove_srgb_suffix(),
                surface_format.add_srgb_suffix(),
            ],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &surface_config);

        let cache = RenderCache::empty();

        Self {
            device: Arc::new(device),
            adapter: Arc::new(adapter),
            queue: Arc::new(queue),
            surface: Arc::new(surface),

            surface_config,

            window_size,
            window,

            cache,
        }
    }

    /// Resizes the window to a new size
    ///
    /// width and height has to be non zero
    pub(crate) fn resize_window(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width == 0 || new_size.height == 0 {
            return;
        }

        self.window_size = new_size;
        self.surface_config.width = new_size.width;
        self.surface_config.height = new_size.height;
        self.surface.configure(&self.device, &self.surface_config);
    }

    /// Resizes the window to the last safe window size
    pub(crate) fn recover_window(&mut self) {
        self.resize_window(self.window_size)
    }

    pub(crate) fn window(&self) -> &winit::window::Window {
        &self.window
    }

    fn aspect_ratio(&self) -> f32 {
        self.window_size.width as f32 / self.window_size.height as f32
    }
}

// Getter functions for render and window operations

pub fn aspect_ratio(ctx: &Context) -> f32 {
    ctx.render.aspect_ratio()
}
pub fn create_encoder(ctx: &Context, label: Option<&str>) -> wgpu::CommandEncoder {
    ctx.render
        .device
        .create_command_encoder(&wgpu::CommandEncoderDescriptor { label })
}
pub fn surface(ctx: &Context) -> &wgpu::Surface<'_> {
    &ctx.render.surface
}
pub fn device(ctx: &Context) -> &wgpu::Device {
    &ctx.render.device
}
pub fn queue(ctx: &Context) -> &wgpu::Queue {
    &ctx.render.queue
}
pub fn adapter(ctx: &Context) -> &wgpu::Adapter {
    &ctx.render.adapter
}
pub fn window(ctx: &Context) -> &winit::window::Window {
    &ctx.render.window
}
pub fn surface_config(ctx: &Context) -> &wgpu::SurfaceConfiguration {
    &ctx.render.surface_config
}
pub fn surface_format(ctx: &Context) -> wgpu::TextureFormat {
    ctx.render.surface_config.format.add_srgb_suffix()
}
pub fn surface_size(ctx: &Context) -> winit::dpi::PhysicalSize<u32> {
    ctx.render.window_size
}
pub fn cache(ctx: &Context) -> &RenderCache {
    &ctx.render.cache
}
pub fn set_vsync(ctx: &mut Context, vsync: bool) {
    let mut surface_config = surface_config(ctx).clone();
    surface_config.present_mode = if vsync {
        wgpu::PresentMode::AutoVsync
    } else {
        wgpu::PresentMode::AutoNoVsync
    };

    let device = device(ctx);
    let surface = surface(ctx);
    surface.configure(device, &surface_config);
}
