use crate::{audio, filesystem, input, render, time, window, Context};

#[cfg(feature = "hot_reload")]
use crate::hot_reload::{self, DllCallbacks};

use std::path::PathBuf;
use wgpu::SurfaceError;

/// User callbaks
pub trait Callbacks {
    /// Use a custom `ContextBuilder`
    fn init_ctx() -> ContextBuilder {
        ContextBuilder::new()
    }

    /// Called after context initilization and before game/update loop
    fn new(_ctx: &mut Context) -> Self;

    /// Called once per frame before rendering
    ///
    /// Return value determines wether to exit game or not
    fn update(&mut self, _ctx: &mut Context) -> bool {
        false
    }

    /// Called once per frame after update
    ///
    /// Return value determines wether to exit game or not
    ///
    /// Must submit at least one render pass, panics otherwise
    fn render(&mut self, ctx: &mut Context, screen_view: &wgpu::TextureView) -> bool {
        let device = render::device(ctx);
        let queue = render::queue(ctx);
        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("default render pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: screen_view,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
                resolve_target: None,
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        queue.submit(Some(encoder.finish()));
        false
    }

    /// Called after window resize
    fn resize(&mut self, _ctx: &mut Context, _new_size: winit::dpi::PhysicalSize<u32>) {}
}

/// Main App
/// Contains all data to run application
pub(crate) struct App<C: Callbacks> {
    #[cfg(not(feature = "hot_reload"))]
    pub(crate) callbacks: C,

    #[cfg(feature = "hot_reload")]
    pub(crate) callbacks: DllCallbacks<C>,
}

/// Functions implemented on App
impl<C: Callbacks + 'static> App<C> {
    pub(crate) fn update_and_render(&mut self, ctx: &mut Context) -> bool {
        // time
        ctx.time.update_time();

        #[cfg(feature = "hot_reload")]
        ctx.hot_reload.reset();

        // update
        if self.callbacks.update(ctx) {
            return true;
        }

        // render
        let surface = render::surface(ctx);
        let output = surface.get_current_texture();
        let output = match output {
            Ok(val) => val,
            Err(SurfaceError::Timeout) => {
                log::error!("timed out getting surface");
                return true;
            }
            Err(SurfaceError::Lost | SurfaceError::Outdated) => {
                ctx.render.recover_window();
                return false;
            }
            Err(err) => {
                log::warn!("{}", err);
                return false;
            }
        };
        let view = output // TODO: make this ARC?
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        if self.callbacks.render(ctx, &view) {
            return true;
        }
        output.present();

        // input
        ctx.input.keyboard.store_keys();
        ctx.input.keyboard.store_modifiers();
        ctx.input.mouse.store_buttons();
        ctx.input.mouse.set_mouse_delta((0.0, 0.0));

        false
    }
}

/// What level of info that should be logged
#[derive(Debug, Clone, Copy)]
pub enum LogLevel {
    None,
    Info,
    Warn,
    Error,
    Debug,
    Trace,
}

impl ContextBuilder {
    /// Initialize init_logging
    ///
    /// Panics if called multiple times
    pub fn init_logging(&self) {
        let log_level = self.log_level;

        if let LogLevel::None = log_level {
            return;
        }

        #[cfg(target_arch = "wasm32")]
        {
            let log_level = match log_level {
                LogLevel::Info => log::Level::Info,
                LogLevel::Warn => log::Level::Warn,
                LogLevel::Error => log::Level::Error,
                LogLevel::Debug => log::Level::Debug,
                LogLevel::Trace => log::Level::Trace,
                LogLevel::None => panic!("unreachable"),
            };
            std::panic::set_hook(Box::new(console_error_panic_hook::hook));
            console_log::init_with_level(log_level).expect("Couldn't initialize logger");
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let log_level = match log_level {
                LogLevel::Info => log::LevelFilter::Info,
                LogLevel::Warn => log::LevelFilter::Warn,
                LogLevel::Error => log::LevelFilter::Error,
                LogLevel::Debug => log::LevelFilter::Debug,
                LogLevel::Trace => log::LevelFilter::Trace,
                LogLevel::None => panic!("unreachable"),
            };
            match env_logger::Builder::new()
                .filter_level(log_level)
                .try_init()
            {
                Ok(_) => log::info!("Sucessfully initialized logging"),
                Err(err) => println!("Error initalizing logging: {}", err),
            }
        }
    }
}

/// Build the context for running an application
#[derive(Debug)]
pub struct ContextBuilder {
    window_builder: Option<winit::window::WindowBuilder>,
    assets_path: PathBuf,
    log_level: LogLevel,
    vsync: bool, // can be set later
    device_features: wgpu::Features,
}

#[allow(clippy::new_without_default)]
impl ContextBuilder {
    pub fn new() -> Self {
        Self {
            log_level: LogLevel::Info,
            assets_path: PathBuf::from("assets"),
            vsync: true,
            device_features: wgpu::Features::default(),
            window_builder: None,
        }
    }

    pub fn assets_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.assets_path = path.into();
        self
    }

    pub fn log_level(mut self, log_level: LogLevel) -> Self {
        self.log_level = log_level;
        self
    }

    pub fn vsync(mut self, vsync: bool) -> Self {
        self.vsync = vsync;
        self
    }

    pub fn device_features(mut self, device_features: wgpu::Features) -> Self {
        self.device_features = device_features;
        self
    }

    pub fn window_builder(mut self, window_builder: winit::window::WindowBuilder) -> Self {
        self.window_builder = Some(window_builder);
        self
    }
}

pub async fn run<C: Callbacks + 'static>() {
    let builder = C::init_ctx();
    builder.init_logging();

    let (window, event_loop) = window::new_window(builder.window_builder);
    let input = input::InputContext::default();
    let time = time::TimeContext::default();
    let filesystem = filesystem::FileSystemContext::new();
    let audio = audio::AudioContext::new();
    let render = render::RenderContext::new(window, builder.vsync, builder.device_features).await;

    let mut ctx = Context {
        input,
        time,
        filesystem,
        audio,
        render,

        #[cfg(feature = "hot_reload")]
        hot_reload: hot_reload::HotReloadContext::new(),
    };

    #[cfg(not(feature = "hot_reload"))]
    let callbacks = C::new(&mut ctx);

    #[cfg(feature = "hot_reload")]
    let callbacks = DllCallbacks::<C>::new(&mut ctx);

    let app = App { callbacks };

    window::run_window(event_loop, app, ctx);
}

#[cfg(not(target_arch = "wasm32"))]
pub fn run_sync<C: Callbacks + 'static>() {
    pollster::block_on(run::<C>())
}
