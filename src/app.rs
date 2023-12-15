use crate::{audio, filesystem, input, render, time, window, Context};
use std::path::PathBuf;
use wgpu::SurfaceError;
use winit::event_loop::EventLoop;

/// User callbaks
pub trait Callbacks {
    /// Called after context initilization and before game/update loop
    fn init(&mut self, _ctx: &mut Context) {}

    /// Called once per frame before rendering
    ///
    /// Return value determines wether to exit game or not
    fn update(&mut self, _ctx: &mut Context) -> bool {
        false
    }

    fn render(
        &mut self,
        _ctx: &mut Context,
        _encoder: &mut wgpu::CommandEncoder,
        _screen_view: &wgpu::TextureView,
    ) -> bool {
        false
    }
}

/// Main App
/// Contains all data to run application
pub(crate) struct App<C: Callbacks> {
    pub(crate) callbacks: C,
}

/// Functions implemented on App
impl<C> App<C>
where
    C: Callbacks + 'static,
{
    /// Main loop which is called from window event loop
    /// Returns true if app should exit
    pub(crate) fn update(&mut self, ctx: &mut Context) -> bool {
        // Update callback
        if self.callbacks.update(ctx) {
            return true;
        }

        // Context updates
        ctx.time.update_time();
        ctx.input.keyboard.save_keys();
        ctx.input.keyboard.save_modifiers();
        ctx.input.mouse.save_buttons();
        ctx.input.mouse.set_mouse_delta((0.0, 0.0));

        // render
        let surface = render::surface(ctx);
        let device = render::device(ctx);
        let queue = render::queue(ctx);

        let output = surface.get_current_texture();
        let output = match output {
            Ok(val) => val,
            Err(SurfaceError::Timeout) => {
                log::error!("timed out getting surface");
                return true;
            }
            Err(SurfaceError::Lost | SurfaceError::Outdated) => {
                render::recover_window(ctx);
                return false;
            }
            Err(err) => {
                log::warn!("{}", err);
                return false;
            }
        };
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("render encodeer"),
        });

        self.callbacks.render(ctx, &mut encoder, &view);

        queue.submit(Some(encoder.finish()));
        output.present();

        false
    }
}

/// Runs the event loop
/// Calls back to user defined functions thorugh Callback trait
pub async fn run<C: Callbacks + 'static>(
    callbacks: C,
    mut ctx: Context,
    event_loop: EventLoop<()>,
) {
    let mut app = App { callbacks };
    app.callbacks.init(&mut ctx);
    window::run_window(event_loop, app, ctx).await;
}

/// What level of info that should be logged
pub enum LogLevel {
    None,
    Info,
    Warn,
    Error,
    Debug,
    Trace,
}

/// Initialize init_logging
///
/// Panics if called multiple times
fn init_logging(log_level: LogLevel) {
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
        let mut env_logger_builder = env_logger::Builder::new();
        env_logger_builder.filter_level(log_level);
        env_logger_builder.init();
    }
}

/// Build the context for running an application
pub struct ContextBuilder {
    assets_path: PathBuf,
    log_level: LogLevel,
    vsync: bool,
    device_features: wgpu::Features,
}

#[allow(clippy::new_without_default)]
impl ContextBuilder {
    pub fn new() -> Self {
        Self {
            log_level: LogLevel::Warn,
            assets_path: PathBuf::from("assets"),
            vsync: true,
            device_features: wgpu::Features::default(),
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

    pub async fn build(self) -> (Context, EventLoop<()>) {
        init_logging(self.log_level);

        let (window, event_loop) = window::new_window();
        let input = input::InputContext::default();
        let time = time::TimeContext::default();
        let filesystem = filesystem::FileSystemContext::new(&self.assets_path);
        let audio = audio::AudioContext::new();
        let render = render::RenderContext::new(window, self.vsync, self.device_features).await;
        let context = Context {
            input,
            time,
            filesystem,
            audio,
            render,
        };

        (context, event_loop)
    }
}

/// Shortcut for ```ContextBuilder```
pub async fn build_context() -> (Context, EventLoop<()>) {
    ContextBuilder::new()
        .log_level(LogLevel::Warn)
        .build()
        .await
}
