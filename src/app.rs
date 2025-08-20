#[cfg(feature = "hot_reload")]
use crate::hot_reload::{self, DllCallbacks};

use crate::{asset::AssetCache, audio, filesystem, input, random, render, time, Context};
use std::path::PathBuf;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use wgpu::SurfaceError;
use winit::{
    event::{self, DeviceEvent, WindowEvent},
    keyboard::PhysicalKey,
    window::WindowAttributes,
};

/// User callbaks
pub trait Callbacks {
    /// Use a custom `ContextBuilder`
    fn init_ctx() -> ContextBuilder {
        ContextBuilder::new()
    }

    /// Called after context initilization and before game/update loop
    fn new(_ctx: &mut Context, cache: &mut AssetCache) -> Self;

    /// Called once per frame after update
    ///
    /// Return value determines wether to exit game or not
    ///
    /// Must submit at least one render pass, panics otherwise
    fn render(
        &mut self,
        _ctx: &mut Context,
        _cache: &mut AssetCache,
        _screen_view: &wgpu::TextureView,
    ) -> bool {
        false
    }

    /// Called after window resize
    fn resize(
        &mut self,
        _ctx: &mut Context,
        _cache: &mut AssetCache,
        _new_size: winit::dpi::PhysicalSize<u32>,
    ) {
    }

    fn window_event(&mut self, _ctx: &mut Context, _event: &winit::event::WindowEvent) {}
}

pub async fn run<C: Callbacks>() {
    C::init_ctx().init_logging();

    let event_loop = winit::event_loop::EventLoop::with_user_event()
        .build()
        .expect("could not create event loop");

    let mut app = App::Uninitialized::<C> {
        proxy: Some(event_loop.create_proxy()),
        builder: C::init_ctx(),
    };

    event_loop
        .run_app(&mut app)
        .expect("could not run event loop");
}

#[cfg(not(target_arch = "wasm32"))]
pub fn run_sync<C: Callbacks>() {
    pollster::block_on(run::<C>())
}

/// general engine state
///
/// can be both initialized and uninitialized
#[allow(clippy::large_enum_variant)]
enum App<C: Callbacks> {
    Uninitialized {
        proxy: Option<winit::event_loop::EventLoopProxy<Context>>,
        builder: ContextBuilder,
    },
    Initialized {
        ctx: Context,

        cache: AssetCache,

        #[cfg(not(feature = "hot_reload"))]
        callbacks: C,
        #[cfg(feature = "hot_reload")]
        callbacks: DllCallbacks<C>,
    },
}

impl<C: Callbacks> winit::application::ApplicationHandler<Context> for App<C> {
    fn user_event(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop, mut ctx: Context) {
        // TODO: init here?
        let mut cache = AssetCache::new();

        #[cfg(not(feature = "hot_reload"))]
        let callbacks = C::new(&mut ctx, &mut cache);

        #[cfg(feature = "hot_reload")]
        let callbacks = DllCallbacks::new(&mut ctx, &mut cache);

        *self = App::Initialized {
            callbacks,
            ctx,
            cache,
        };
    }

    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        // Ignore if already initialized
        let App::Uninitialized { proxy, builder } = self else {
            return;
        };
        if proxy.is_none() {
            return;
        }

        // initialize context
        async fn init(
            window: winit::window::Window,
            builder: ContextBuilder,
            proxy: winit::event_loop::EventLoopProxy<Context>,
        ) {
            let input = input::InputContext::new();
            let time = time::TimeContext::default();
            let filesystem = filesystem::FileSystemContext::new();
            let audio = audio::AudioContext::new();
            let render = render::RenderContext::new(window, &builder).await;
            let random = random::RandomContext::new();

            let ctx = Context {
                input,
                time,
                filesystem,
                audio,
                render,
                random,

                #[cfg(feature = "hot_reload")]
                hot_reload: hot_reload::HotReloadContext::new(),
            };

            let sucess = proxy.send_event(ctx).is_ok();
            assert!(sucess, "could not send context event");
        }

        let proxy = proxy.take().unwrap();
        let mut builder = builder.clone();

        #[cfg(target_arch = "wasm32")]
        {
            use web_sys::wasm_bindgen::JsCast;
            use winit::platform::web::WindowAttributesExtWebSys;

            let win = web_sys::window().unwrap();
            let document = win.document().unwrap();
            let canvas = document.get_element_by_id("gbase").unwrap();
            // let html_canvas_element = canvas.unchecked_into();
            let canvas = document
                .get_element_by_id("gbase")
                .expect("could not find canvas")
                .dyn_into::<web_sys::HtmlCanvasElement>()
                .expect("element was not a canvas");
            let (width, height) = (canvas.width(), canvas.height());
            builder.window_attributes = builder
                .window_attributes
                .with_canvas(Some(canvas))
                .with_inner_size(winit::dpi::LogicalSize::new(width, height));
        }

        let window = event_loop
            .create_window(builder.window_attributes.clone())
            .expect("could not create window");

        #[cfg(target_arch = "wasm32")]
        wasm_bindgen_futures::spawn_local(init(window, builder, proxy));

        #[cfg(not(target_arch = "wasm32"))]
        pollster::block_on(init(window, builder, proxy));
    }

    fn about_to_wait(&mut self, _event_loop: &winit::event_loop::ActiveEventLoop) {
        let App::Initialized { ref mut ctx, .. } = self else {
            tracing::warn!("app not initialized while receiving about to wait event -> skipping");
            return;
        };

        ctx.render.window().request_redraw();
    }

    fn device_event(
        &mut self,
        _event_loop: &winit::event_loop::ActiveEventLoop,
        _device_id: event::DeviceId,
        event: event::DeviceEvent,
    ) {
        let App::Initialized { ref mut ctx, .. } = self else {
            tracing::warn!("app not initialized while receiving device event -> skipping");
            return;
        };

        #[allow(clippy::single_match)]
        match event {
            DeviceEvent::MouseMotion { delta } => {
                if ctx.input.mouse.mouse_on_screen() {
                    ctx.input.mouse.set_mouse_delta(delta);
                }
            }
            _ => {}
        }
    }

    fn window_event(
        &mut self,
        event_loop: &winit::event_loop::ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: event::WindowEvent,
    ) {
        let App::Initialized {
            ref mut ctx,
            callbacks,
            cache,
        } = self
        else {
            tracing::warn!("app not initialized while receiving window event -> skipping");
            return;
        };

        // TODO: temp for egui
        callbacks.window_event(ctx, &event);

        match event {
            WindowEvent::RedrawRequested => {
                // hot reload
                #[cfg(feature = "hot_reload")]
                {
                    if ctx.hot_reload.should_reload() {
                        tracing::info!("Hot reload");
                        callbacks.hot_reload(ctx, cache);
                    }
                    if ctx.hot_reload.should_restart() {
                        tracing::info!("Hot restart");
                        callbacks.hot_restart(ctx, cache);
                    }
                }

                // update
                if update_and_render(ctx, cache, callbacks) {
                    event_loop.exit();
                }
            }
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(new_size) => {
                ctx.render.resize_window(new_size);
                callbacks.resize(ctx, cache, new_size);
            }
            // Keyboard
            WindowEvent::KeyboardInput { event, .. } => {
                let (key, pressed) = (event.physical_key, event.state.is_pressed());
                match (key, pressed) {
                    (PhysicalKey::Code(code), true) => ctx.input.keyboard.set_key(code),
                    (PhysicalKey::Code(code), false) => ctx.input.keyboard.release_key(code),
                    (PhysicalKey::Unidentified(code), _) => {
                        tracing::error!("pressed/released unidentified key {:?}", code)
                    }
                };
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                ctx.input.keyboard.modifiers_changed(modifiers)
            }
            // Mouse
            WindowEvent::MouseInput { state, button, .. } => {
                match state {
                    winit::event::ElementState::Pressed => ctx.input.mouse.press_button(button),
                    winit::event::ElementState::Released => ctx.input.mouse.release_button(button),
                };
            }
            WindowEvent::MouseWheel { delta, .. } => match delta {
                winit::event::MouseScrollDelta::LineDelta(x, y) => {
                    ctx.input.mouse.set_scroll_delta((x as f64, y as f64));
                }
                winit::event::MouseScrollDelta::PixelDelta(pos) => {
                    ctx.input.mouse.set_scroll_delta((pos).into());
                }
            },
            WindowEvent::CursorMoved { position, .. } => {
                ctx.input.mouse.set_pos(position.into());
            }
            WindowEvent::CursorEntered { .. } => {
                ctx.input.mouse.set_on_screen(true);
            }
            WindowEvent::CursorLeft { .. } => {
                ctx.input.mouse.set_on_screen(false);
            }
            _ => {}
        }
    }
}

/// Functions implemented on App
fn update_and_render(
    ctx: &mut Context,
    cache: &mut AssetCache,
    callbacks: &mut impl Callbacks,
) -> bool {
    // time
    ctx.time.pre_update();
    #[cfg(feature = "hot_reload")]
    ctx.hot_reload.pre_update();

    // render
    let surface = render::surface(ctx);
    let output = surface.get_current_texture();
    let output = match output {
        Ok(val) => val,
        Err(SurfaceError::Timeout) => {
            tracing::error!("timed out getting surface");
            return true;
        }
        Err(SurfaceError::Lost | SurfaceError::Outdated) => {
            ctx.render.recover_window();
            return false;
        }
        Err(err) => {
            tracing::warn!("{}", err);
            return false;
        }
    };
    let view = output // TODO: make this ARC?
        .texture
        .create_view(&wgpu::TextureViewDescriptor {
            format: Some(render::surface_format(ctx)), // TODO: add option to avoid gamma correction
            ..Default::default()
        });

    if callbacks.render(ctx, cache, &view) {
        return true;
    }

    output.present();

    // input
    ctx.input.post_update();
    ctx.time.post_update();

    ctx.render.gpu_profiler.readback(
        &ctx.render.device,
        &ctx.render.queue,
        ctx.time.profiler.clone(),
    );

    cache.poll();

    // TODO: dont do this every frame
    cache.clear_cpu_handles();
    cache.clear_gpu_handles();

    false
}

//
// Context builder
//

/// Build the context for running an application
#[derive(Debug, Clone)]
pub struct ContextBuilder {
    pub(crate) window_attributes: winit::window::WindowAttributes,
    pub(crate) device_features: wgpu::Features,
    pub(crate) log_level: tracing::Level,
    pub(crate) assets_path: PathBuf, // can be set later
    pub(crate) vsync_enabled: bool,  // can be set later

    pub(crate) gpu_profiler_enabled: bool, // can be set later
    pub(crate) gpu_profiler_capacity: u32, // can be set later
}

#[allow(clippy::new_without_default)]
impl ContextBuilder {
    pub fn new() -> Self {
        Self {
            log_level: tracing::Level::INFO,
            assets_path: PathBuf::from("assets"),
            vsync_enabled: true,
            device_features: wgpu::Features::default() | wgpu::Features::TIMESTAMP_QUERY,
            window_attributes: WindowAttributes::default(),

            gpu_profiler_enabled: false,
            gpu_profiler_capacity: 64,
        }
    }

    pub fn gpu_profiler_enabled(mut self, enabled: bool) -> Self {
        self.gpu_profiler_enabled = enabled;
        // self.device_features |= wgpu::Features::TIMESTAMP_QUERY;
        self
    }

    pub fn gpu_profiler_capacity(mut self, capacity: u32) -> Self {
        self.gpu_profiler_capacity = capacity;
        self
    }

    pub fn assets_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.assets_path = path.into();
        self
    }

    pub fn log_level(mut self, log_level: tracing::Level) -> Self {
        self.log_level = log_level;
        self
    }

    pub fn vsync(mut self, enabled: bool) -> Self {
        self.vsync_enabled = enabled;
        self
    }

    pub fn device_features(mut self, device_features: wgpu::Features) -> Self {
        self.device_features = device_features;
        self
    }

    pub fn window_attributes(mut self, window_attributes: winit::window::WindowAttributes) -> Self {
        self.window_attributes = window_attributes;
        self
    }
}

// TODO: rework logging?

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
        #[cfg(target_arch = "wasm32")]
        {
            console_error_panic_hook::set_once();

            let wasm_layer = tracing_wasm::WASMLayer::new(
                tracing_wasm::WASMLayerConfigBuilder::new()
                    .set_max_level(self.log_level)
                    .build(),
            );
            let subscriber = tracing_subscriber::registry().with(wasm_layer);
            subscriber.init();
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let filter_layer = tracing_subscriber::filter::LevelFilter::from(self.log_level);
            let format_layer = tracing_subscriber::fmt::layer();
            let subscriber = tracing_subscriber::registry()
                .with(filter_layer)
                .with(format_layer);

            #[cfg(feature = "trace_tracy")]
            let subscriber = subscriber.with(tracing_tracy::TracyLayer::default());

            match subscriber.try_init() {
                Ok(_) => tracing::info!("sucessfully initialized tracing subscriber"),
                Err(err) => {
                    tracing::error!("could not initialize tracing subscriber: {}", err)
                }
            }
        }
    }
}
