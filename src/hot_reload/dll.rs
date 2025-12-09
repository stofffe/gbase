use crate::{asset, CallbackResult, Context};

#[rustfmt::skip]
type NewFunc<T> = fn(ctx: &mut crate::Context, cache: &mut asset::AssetCache) -> T;
#[rustfmt::skip]
type ShutdownFunc<T> = fn(callbacks: &mut T, ctx: &mut crate::Context, cache: &mut asset::AssetCache);
#[rustfmt::skip]
type RenderFunc<T> = fn(callbacks: &mut T, ctx: &mut crate::Context, cache: &mut asset::AssetCache, screen_view: &wgpu::TextureView) -> CallbackResult;
#[rustfmt::skip]
type FixedUpdateFunc<T> = fn(callbacks: &mut T, ctx: &mut crate::Context, cache: &mut asset::AssetCache,) -> CallbackResult;
#[rustfmt::skip]
type ResizeFunc<T> = fn(callbacks: &mut T, ctx: &mut crate::Context, cache: &mut asset::AssetCache, new_size: winit::dpi::PhysicalSize<u32>,) -> CallbackResult;
#[rustfmt::skip]
type ReloadFunc<T> = fn(callbacks: &mut T, ctx: &mut crate::Context, cache: &mut asset::AssetCache);

#[cfg(feature = "egui")]
#[rustfmt::skip]
type RenderEguiFunc<T> = fn(callbacks: &mut T, ctx: &mut crate::Context, cache: &mut asset::AssetCache, egui_ctx: &mut crate::egui_ui::EguiContext,) -> CallbackResult;

pub struct DllApi<T> {
    new_callback: NewFunc<T>,
    shutdown_callback: Option<ShutdownFunc<T>>,
    render_callback: Option<RenderFunc<T>>,
    fixed_update_callback: Option<FixedUpdateFunc<T>>,
    resize_callback: Option<ResizeFunc<T>>,
    reload_callback: Option<ReloadFunc<T>>,

    #[cfg(feature = "egui")]
    render_egui_callback: Option<RenderEguiFunc<T>>,
}

/// Wrapper for callbacks + dll
pub struct DllCallbacks<T> {
    pub callbacks: T,
    pub dll: DllApi<T>,
    pub dll_index: u32,
}

impl<T> crate::Callbacks for DllCallbacks<T> {
    // NOTE:
    // init_ctx should only be called at program startup and never through dll
    // so we never store it here and we panic if called on DllContext
    fn init_ctx() -> crate::ContextBuilder {
        panic!("init_ctx on DllCallbacks should never be called");
    }

    fn new(ctx: &mut crate::Context, cache: &mut asset::AssetCache) -> Self {
        let dll_index = 0;
        let dll = load_dll(dll_index);

        let mut callbacks = (dll.new_callback)(ctx, cache);

        if let Some(hot_reload) = dll.reload_callback {
            hot_reload(&mut callbacks, ctx, cache);
        }

        Self {
            callbacks,
            dll,
            dll_index,
        }
    }

    #[rustfmt::skip]
    fn shutdown(&mut self, ctx: &mut Context, cache: &mut asset::AssetCache) {
        // call user shutdown
        match self.dll.shutdown_callback {
            Some(shutdown) => shutdown(&mut self.callbacks, ctx, cache),
            None => (),
        }

        // remove current dll
        remove_dll(self.dll_index);
    }

    #[rustfmt::skip]
    fn render(&mut self, ctx: &mut crate::Context, cache: &mut asset::AssetCache, screen_view: &wgpu::TextureView) -> CallbackResult {
        match self.dll.render_callback {
            Some(render) => render(&mut self.callbacks, ctx, cache, screen_view),
            None => CallbackResult::Continue,
        }
    }

    #[rustfmt::skip]
    fn fixed_update(&mut self, ctx: &mut crate::Context, cache: &mut asset::AssetCache) -> CallbackResult {
        match self.dll.fixed_update_callback {
            Some(fixed_update) => fixed_update(&mut self.callbacks, ctx, cache),
            None => CallbackResult::Continue,
        }
    }

    #[rustfmt::skip]
    fn resize(&mut self, ctx: &mut crate::Context, cache: &mut asset::AssetCache, new_size: winit::dpi::PhysicalSize<u32>) -> CallbackResult {
        #[allow(clippy::single_match)]
        match self.dll.resize_callback {
            Some(resize) => resize(&mut self.callbacks, ctx, cache, new_size),
            None => CallbackResult::Continue,
        }
    }

    #[cfg(feature = "egui")]
    #[rustfmt::skip]
    fn render_egui(&mut self, ctx: &mut crate::Context, cache: &mut asset::AssetCache, egui_ctx: &mut crate::egui_ui::EguiContext) -> CallbackResult {
        #[allow(clippy::single_match)]
        match self.dll.render_egui_callback {
            Some(render_egui) => render_egui(&mut self.callbacks, ctx, cache, egui_ctx),
            None => CallbackResult::Continue,
        }
    }
}

impl<T> DllCallbacks<T> {
    /// reload dll file
    ///
    /// keep game state
    pub fn hot_reload(&mut self, ctx: &mut crate::Context, cache: &mut asset::AssetCache) {
        self.dll_index += 1;
        self.dll = load_dll(self.dll_index);

        if let Some(hot_reload) = self.dll.reload_callback {
            hot_reload(&mut self.callbacks, ctx, cache);
        }
    }

    /// reload dll file
    ///
    /// reset game state
    pub fn hot_restart(&mut self, ctx: &mut crate::Context, cache: &mut asset::AssetCache) {
        self.hot_reload(ctx, cache);
        self.callbacks = (self.dll.new_callback)(ctx, cache);
    }
}

fn load_dll<T>(dll_index: u32) -> DllApi<T> {
    let input = super::format_dll_input();
    let output = super::format_dll_output(dll_index);

    // copy dll to avoid collisions
    std::fs::copy(&input, &output).expect("could not copy dll");

    let lib = dlopen::symbor::Library::open(&output).unwrap();

    // remove old dlls
    if dll_index > 0 {
        remove_dll(dll_index - 1);
    }

    let new_callback = match unsafe { lib.symbol::<NewFunc<T>>("new") } {
        Ok(f) => *f,
        Err(err) => {
            tracing::error!("could not find function new");
            tracing::error!("TIP: make sure callbacks are defined in library and not main.rs");
            tracing::error!("TIP: make sure functions are marked with #[no_mangle]");
            panic!("{}", err);
        }
    };

    let shutdown_callback = match unsafe { lib.symbol::<ShutdownFunc<T>>("shutdown") } {
        Ok(f) => Some(*f),
        Err(err) => {
            tracing::warn!("could not find function shutdown: {}", err);
            None
        }
    };

    let render_callback = match unsafe { lib.symbol::<RenderFunc<T>>("render") } {
        Ok(f) => Some(*f),
        Err(err) => {
            tracing::warn!("could not find function render: {}", err);
            None
        }
    };
    let fixed_update_callback = match unsafe { lib.symbol::<FixedUpdateFunc<T>>("fixed_update") } {
        Ok(f) => Some(*f),
        Err(err) => {
            tracing::warn!("could not find function render: {}", err);
            None
        }
    };
    let resize_callback = match unsafe { lib.symbol::<ResizeFunc<T>>("resize") } {
        Ok(f) => Some(*f),
        Err(err) => {
            tracing::warn!("could not find function resize: {}", err);
            None
        }
    };
    let reload_callback = match unsafe { lib.symbol::<ReloadFunc<T>>("hot_reload") } {
        Ok(f) => Some(*f),
        Err(err) => {
            tracing::warn!("could not find function hot_reload: {}", err);
            None
        }
    };
    #[cfg(feature = "egui")]
    let render_egui_callback = match unsafe { lib.symbol::<RenderEguiFunc<T>>("render_egui") } {
        Ok(f) => Some(*f),
        Err(err) => {
            tracing::warn!("could not find function egui: {}", err);
            None
        }
    };

    DllApi {
        new_callback,
        shutdown_callback,
        render_callback,
        fixed_update_callback,
        resize_callback,
        reload_callback,

        #[cfg(feature = "egui")]
        render_egui_callback,
    }
}

pub(crate) fn remove_dll(dll_index: u32) {
    let previous_output = super::format_dll_output(dll_index);
    // TODO: maybe just log on error
    std::fs::remove_file(previous_output).expect("could not remove old dll");
}
