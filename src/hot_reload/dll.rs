// NOTE:
// init_ctx should only be called at program startup and never through dll
// so we never store it here and we panic if called on DllContext

#[rustfmt::skip]
type RenderFunc<T> = fn(callbacks: &mut T, ctx: &mut crate::Context, screen_view: &wgpu::TextureView) -> bool;
type NewFunc<T> = fn(ctx: &mut crate::Context) -> T;
type UpdateFunc<T> = fn(callbacks: &mut T, ctx: &mut crate::Context) -> bool;
type ResizeFunc<T> =
    fn(callbacks: &mut T, ctx: &mut crate::Context, new_size: winit::dpi::PhysicalSize<u32>);

pub struct DllApi<T> {
    new: NewFunc<T>,
    update: Option<UpdateFunc<T>>,
    render: Option<RenderFunc<T>>,
    resize: Option<ResizeFunc<T>>,
}

/// Wrapper for callbacks + dll
pub struct DllCallbacks<T> {
    pub callbacks: T,
    pub dll: DllApi<T>,
}

impl<T> crate::Callbacks for DllCallbacks<T> {
    fn init_ctx() -> crate::ContextBuilder {
        panic!("init_ctx on DllCallbacks should never be called");
    }

    fn new(ctx: &mut crate::Context) -> Self {
        let dll = load_dll();
        let callbacks = (dll.new)(ctx);
        Self { callbacks, dll }
    }

    fn update(&mut self, ctx: &mut crate::Context) -> bool {
        match self.dll.update {
            Some(update) => update(&mut self.callbacks, ctx),
            None => false,
        }
    }

    fn render(&mut self, ctx: &mut crate::Context, screen_view: &wgpu::TextureView) -> bool {
        match self.dll.render {
            Some(render) => render(&mut self.callbacks, ctx, screen_view),
            None => false,
        }
    }

    fn resize(&mut self, ctx: &mut crate::Context, new_size: winit::dpi::PhysicalSize<u32>) {
        #[allow(clippy::single_match)]
        match self.dll.resize {
            Some(resize) => resize(&mut self.callbacks, ctx, new_size),
            None => {}
        }
    }
}

impl<T> DllCallbacks<T> {
    /// reload dll file
    ///
    /// keep game state
    pub fn hot_reload(&mut self) {
        self.dll = load_dll();
    }

    /// reload dll file
    ///
    /// reset game state
    pub fn hot_restart(&mut self, ctx: &mut crate::Context) {
        self.hot_reload();
        self.callbacks = (self.dll.new)(ctx);
    }
}

fn load_dll<T>() -> DllApi<T> {
    let lib = dlopen::symbor::Library::open(super::dllname()).unwrap();

    let new = match unsafe { lib.symbol::<NewFunc<T>>("new") } {
        Ok(f) => *f,
        Err(err) => {
            log::error!("could not find function new");
            log::error!("TIP: make sure callbacks are defined in library and not main.rs");
            log::error!("TIP: make sure functions are marked with #[no_mangle]");
            panic!("{}", err);
        }
    };

    let update = match unsafe { lib.symbol::<UpdateFunc<T>>("update") } {
        Ok(f) => Some(*f),
        Err(err) => {
            log::warn!("could not find function update: {}", err);
            None
        }
    };
    let render = match unsafe { lib.symbol::<RenderFunc<T>>("render") } {
        Ok(f) => Some(*f),
        Err(err) => {
            log::warn!("could not find function render: {}", err);
            None
        }
    };
    let resize = match unsafe { lib.symbol::<ResizeFunc<T>>("resize") } {
        Ok(f) => Some(*f),
        Err(err) => {
            log::warn!("could not find function resize: {}", err);
            None
        }
    };

    DllApi {
        new,
        update,
        render,
        resize,
    }
}
