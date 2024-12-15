pub mod core;
pub use core::*;

pub mod helpers;
pub use helpers::*;

use crate::Context;
use std::sync::Arc;
use winit::dpi::PhysicalSize;

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

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            dx12_shader_compiler: wgpu::Dx12Compiler::default(),
            gles_minor_version: wgpu::Gles3MinorVersion::default(),
            flags: wgpu::InstanceFlags::default(),
        });

        let surface = instance
            .create_surface(window.clone())
            .expect("could not create surface");
        // let surface = unsafe { instance.create_surface(window) }.expect("could not create surface");

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
        let window_size = window.inner_size();
        // let window_size = PhysicalSize::new(400, 400);
        // log::warn!("window_size {:?}", window_size);

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
            view_formats: vec![],
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
    pub(crate) fn resize_window(&mut self, new_size: PhysicalSize<u32>) {
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

    // pub(crate) fn window_size(&self) -> PhysicalSize<u32> {
    //     self.window_size
    // }

    pub(crate) fn aspect_ratio(&self) -> f32 {
        self.window_size.width as f32 / self.window_size.height as f32
    }
}

// Getter functions for render and window operations

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
pub fn surface_size(ctx: &Context) -> winit::dpi::PhysicalSize<u32> {
    ctx.render.window_size
}
pub fn cache(ctx: &Context) -> &RenderCache {
    &ctx.render.cache
}
