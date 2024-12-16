use dlopen::wrapper::{Container, WrapperApi};
use dlopen_derive::WrapperApi;

/// Dll Api for Callbacks
#[derive(WrapperApi)]
pub struct DllApi<T> {
    new: fn(ctx: &mut crate::Context) -> T,
    update: fn(callbacks: &mut T, ctx: &mut crate::Context) -> bool,
    render:
        fn(callbacks: &mut T, ctx: &mut crate::Context, screen_view: &wgpu::TextureView) -> bool,
    resize: fn(callbacks: &mut T, ctx: &mut crate::Context),
}

/// Wrapper for callbacks + dll
pub struct DllCallbacks<T> {
    pub callbacks: T,
    pub dll: Container<DllApi<T>>,
}

impl<T> crate::Callbacks for DllCallbacks<T> {
    fn new(ctx: &mut crate::Context) -> Self {
        let dll: Container<DllApi<T>> = unsafe { Container::load(super::DLL_NAME) }
            .expect("Could not open library or load symbols");
        let callbacks = dll.new(ctx);

        Self { callbacks, dll }
    }

    fn update(&mut self, ctx: &mut crate::Context) -> bool {
        self.dll.update(&mut self.callbacks, ctx)
    }

    fn render(&mut self, ctx: &mut crate::Context, screen_view: &wgpu::TextureView) -> bool {
        self.dll.render(&mut self.callbacks, ctx, screen_view)
    }

    fn resize(&mut self, ctx: &mut crate::Context) {
        self.dll.resize(&mut self.callbacks, ctx)
    }
}

impl<T> DllCallbacks<T> {
    /// reload dll file
    ///
    /// keep game state
    pub fn hot_reload(&mut self) {
        self.dll = unsafe { Container::load(super::DLL_NAME) }
            .expect("Could not open library or load symbols");
    }

    /// reload dll file
    ///
    /// reset game state
    pub fn hot_restart(&mut self, ctx: &mut crate::Context) {
        self.hot_reload();
        self.callbacks = self.dll.new(ctx);
    }
}
