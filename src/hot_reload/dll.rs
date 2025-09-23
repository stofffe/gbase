use crate::CallbackResult;

type NewFunc<T> = fn(ctx: &mut crate::Context, cache: &mut crate::asset::AssetCache) -> T;
type RenderFunc<T> = fn(
    callbacks: &mut T,
    ctx: &mut crate::Context,
    cache: &mut crate::asset::AssetCache,
    screen_view: &wgpu::TextureView,
) -> CallbackResult;
type FixedUpdateFunc<T> = fn(
    callbacks: &mut T,
    ctx: &mut crate::Context,
    cache: &mut crate::asset::AssetCache,
) -> CallbackResult;
type ResizeFunc<T> = fn(
    callbacks: &mut T,
    ctx: &mut crate::Context,
    cache: &mut crate::asset::AssetCache,
    new_size: winit::dpi::PhysicalSize<u32>,
) -> CallbackResult;
type ReloadFunc<T> =
    fn(callbacks: &mut T, ctx: &mut crate::Context, cache: &mut crate::asset::AssetCache);
#[cfg(feature = "egui")]
type RenderEguiFunc<T> = fn(
    callbacks: &mut T,
    ctx: &mut crate::Context,
    egui_ctx: &mut crate::egui_ui::EguiContext,
) -> CallbackResult;

pub struct DllApi<T> {
    new_callback: NewFunc<T>,
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
}

impl<T> crate::Callbacks for DllCallbacks<T> {
    // NOTE:
    // init_ctx should only be called at program startup and never through dll
    // so we never store it here and we panic if called on DllContext
    fn init_ctx() -> crate::ContextBuilder {
        panic!("init_ctx on DllCallbacks should never be called");
    }

    fn new(ctx: &mut crate::Context, cache: &mut crate::asset::AssetCache) -> Self {
        let dll = load_dll();

        let mut callbacks = (dll.new_callback)(ctx, cache);

        if let Some(hot_reload) = dll.reload_callback {
            hot_reload(&mut callbacks, ctx, cache);
        }

        Self { callbacks, dll }
    }

    fn render(
        &mut self,
        ctx: &mut crate::Context,
        cache: &mut crate::asset::AssetCache,
        screen_view: &wgpu::TextureView,
    ) -> CallbackResult {
        match self.dll.render_callback {
            Some(render) => render(&mut self.callbacks, ctx, cache, screen_view),
            None => CallbackResult::Continue,
        }
    }

    fn fixed_update(
        &mut self,
        ctx: &mut crate::Context,
        cache: &mut crate::asset::AssetCache,
    ) -> CallbackResult {
        match self.dll.fixed_update_callback {
            Some(fixed_update) => fixed_update(&mut self.callbacks, ctx, cache),
            None => CallbackResult::Continue,
        }
    }

    fn resize(
        &mut self,
        ctx: &mut crate::Context,
        cache: &mut crate::asset::AssetCache,
        new_size: winit::dpi::PhysicalSize<u32>,
    ) -> CallbackResult {
        #[allow(clippy::single_match)]
        match self.dll.resize_callback {
            Some(resize) => resize(&mut self.callbacks, ctx, cache, new_size),
            None => CallbackResult::Continue,
        }
    }

    #[cfg(feature = "egui")]
    fn render_egui(
        &mut self,
        ctx: &mut crate::Context,
        egui_ctx: &mut crate::egui_ui::EguiContext,
    ) -> CallbackResult {
        #[allow(clippy::single_match)]
        match self.dll.render_egui_callback {
            Some(render_egui) => render_egui(&mut self.callbacks, ctx, egui_ctx),
            None => CallbackResult::Continue,
        }
    }
}

impl<T> DllCallbacks<T> {
    /// reload dll file
    ///
    /// keep game state
    pub fn hot_reload(&mut self, ctx: &mut crate::Context, cache: &mut crate::asset::AssetCache) {
        self.dll = load_dll();

        if let Some(hot_reload) = self.dll.reload_callback {
            hot_reload(&mut self.callbacks, ctx, cache);
        }
    }

    /// reload dll file
    ///
    /// reset game state
    pub fn hot_restart(&mut self, ctx: &mut crate::Context, cache: &mut crate::asset::AssetCache) {
        self.hot_reload(ctx, cache);
        self.callbacks = (self.dll.new_callback)(ctx, cache);
    }
}

fn load_dll<T>() -> DllApi<T> {
    let lib = dlopen::symbor::Library::open(super::dllname()).unwrap();

    let new_callback = match unsafe { lib.symbol::<NewFunc<T>>("new") } {
        Ok(f) => *f,
        Err(err) => {
            tracing::error!("could not find function new");
            tracing::error!("TIP: make sure callbacks are defined in library and not main.rs");
            tracing::error!("TIP: make sure functions are marked with #[no_mangle]");
            panic!("{}", err);
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
        render_callback,
        fixed_update_callback,
        resize_callback,
        reload_callback,

        #[cfg(feature = "egui")]
        render_egui_callback,
    }
}
