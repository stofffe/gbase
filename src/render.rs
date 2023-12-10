use crate::Context;
use std::sync::Arc;
use wgpu::SurfaceConfiguration;
use winit::dpi::PhysicalSize;

pub(crate) struct RenderContext {
    surface: Arc<wgpu::Surface>,
    device: Arc<wgpu::Device>,
    adapter: Arc<wgpu::Adapter>,
    queue: Arc<wgpu::Queue>,

    surface_config: wgpu::SurfaceConfiguration,
    window_size: winit::dpi::PhysicalSize<u32>,
    window: Arc<winit::window::Window>,
}

impl RenderContext {
    pub(crate) async fn new(window: winit::window::Window) -> Self {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            dx12_shader_compiler: wgpu::Dx12Compiler::default(),
            gles_minor_version: wgpu::Gles3MinorVersion::default(),
            flags: wgpu::InstanceFlags::default(),
        });

        let surface =
            unsafe { instance.create_surface(&window) }.expect("could not create surface");

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
                    features: wgpu::Features::default(),
                    limits: adapter.limits(),
                    label: None,
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
        log::warn!("window_size {:?}", window_size);
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT, // TODO might want to add more here
            format: surface_format,
            width: window_size.width.max(1),
            height: window_size.height.max(1),
            present_mode: wgpu::PresentMode::AutoNoVsync, // TODO add option?
            alpha_mode: surface_capabilities.alpha_modes[0],
            view_formats: vec![],
        };
        surface.configure(&device, &surface_config);

        Self {
            device: Arc::new(device),
            adapter: Arc::new(adapter),
            queue: Arc::new(queue),
            surface: Arc::new(surface),

            surface_config,

            window_size,
            window: Arc::new(window),
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

    pub(crate) fn request_redraw(&self) {
        self.window.request_redraw();
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
}

impl Vertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &wgpu::vertex_attr_array![
                0 =>Float32x3,
            ],
            // attributes: &Self::ATTRIBUTES,
        }
    }
}

// Getter functions for render and window operations

pub fn surface(ctx: &mut Context) -> Arc<wgpu::Surface> {
    ctx.render.surface.clone()
}
pub fn device(ctx: &mut Context) -> Arc<wgpu::Device> {
    ctx.render.device.clone()
}
pub fn queue(ctx: &mut Context) -> Arc<wgpu::Queue> {
    ctx.render.queue.clone()
}
pub fn adapter(ctx: &mut Context) -> Arc<wgpu::Adapter> {
    ctx.render.adapter.clone()
}
pub fn window(ctx: &mut Context) -> Arc<winit::window::Window> {
    ctx.render.window.clone()
}
pub fn surface_config(ctx: &mut Context) -> wgpu::SurfaceConfiguration {
    ctx.render.surface_config.clone()
}
pub fn recover_window(ctx: &mut Context) {
    ctx.render.recover_window();
}

// /// Creates a render pass which renders to the current window
// pub fn screen_render_pass<RenderFunc>(
//     ctx: &mut Context,
//     mut render_func: RenderFunc,
//     clear_color: wgpu::Color,
// ) where
//     RenderFunc: FnMut(&wgpu::RenderPass, &wgpu::CommandEncoder),
// {
//     let output = ctx.render.surface.get_current_texture().unwrap();
//
//     let view = output
//         .texture
//         .create_view(&wgpu::TextureViewDescriptor::default());
//
//     let mut encoder = ctx
//         .render
//         .device
//         .create_command_encoder(&wgpu::CommandEncoderDescriptor {
//             label: Some("screen render encodeer"),
//         });
//
//     {
//         let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
//             label: Some("screen render pass"),
//             color_attachments: &[Some(wgpu::RenderPassColorAttachment {
//                 view: &view,
//                 ops: wgpu::Operations {
//                     load: wgpu::LoadOp::Clear(wgpu::Color::BLUE),
//                     store: wgpu::StoreOp::Store,
//                 },
//                 resolve_target: None,
//             })],
//             depth_stencil_attachment: None,
//             timestamp_writes: None,
//             occlusion_query_set: None,
//         });
//         render_func(&render_pass, &encoder);
//     }
//
//     ctx.render.queue.submit(Some(encoder.finish()));
//     output.present();
// }
