extern crate dlopen;

use std::{path::Path, sync::mpsc};

use dlopen::wrapper::{Container, WrapperApi};
use dlopen_derive::WrapperApi;
use notify::Watcher;

const DLL_NAME: &str = "libhot_reload.dylib";

/// Dll Api for Callbacks
#[derive(WrapperApi)]
pub struct DllApi<T> {
    new: fn() -> T,
    update: fn(game: &mut T, ctx: &mut crate::Context) -> bool,
    render: fn(game: &mut T, ctx: &mut crate::Context, screen_view: &wgpu::TextureView) -> bool,
    resize: fn(game: &mut T, ctx: &mut crate::Context),
}

/// Wrapper for Game + dll reloading
pub struct DllCallbacks<T> {
    pub callbacks: T,
    pub dll: Container<DllApi<T>>,
    pub dll_watcher: notify::FsEventWatcher,
    pub dll_change_channel: mpsc::Receiver<Result<notify::Event, notify::Error>>,
}

impl<T> crate::Callbacks for DllCallbacks<T> {
    fn new(_ctx: &mut crate::Context) -> Self {
        let dll: Container<DllApi<T>> =
            unsafe { Container::load(DLL_NAME) }.expect("Could not open library or load symbols");
        let game = dll.new();

        let (tx, rx) = mpsc::channel();

        let mut watcher = notify::recommended_watcher(tx).unwrap();
        watcher
            .watch(Path::new(DLL_NAME), notify::RecursiveMode::NonRecursive)
            .unwrap();

        Self {
            callbacks: game,
            dll,
            dll_watcher: watcher,
            dll_change_channel: rx,
        }
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
    // pub fn new(_callbacks: T) -> Self {
    //     let dll: Container<DllApi<T>> =
    //         unsafe { Container::load(DLL_NAME) }.expect("Could not open library or load symbols");
    //     let game = dll.new();
    //
    //     let (tx, rx) = mpsc::channel();
    //
    //     let mut watcher = notify::recommended_watcher(tx).unwrap();
    //     watcher
    //         .watch(Path::new(DLL_NAME), notify::RecursiveMode::NonRecursive)
    //         .unwrap();
    //
    //     Self {
    //         callbacks: game,
    //         dll,
    //         dll_watcher: watcher,
    //         dll_change_channel: rx,
    //     }
    // }

    /// checks if dll file has changed
    pub fn dll_changed(&self) -> bool {
        if let Ok(Ok(event)) = self.dll_change_channel.try_recv() {
            if let notify::EventKind::Modify(_) | notify::EventKind::Create(_) = event.kind {
                return true;
            }
        }
        false
    }

    /// reload dll file
    ///
    /// keep game state
    pub fn hot_reload(&mut self) {
        self.dll =
            unsafe { Container::load(DLL_NAME) }.expect("Could not open library or load symbols");
    }

    // /// reload dll file
    // ///
    // /// reset game state
    // pub fn hot_restart(&mut self) {
    //     self.dll =
    //         unsafe { Container::load(DLL_NAME) }.expect("Could not open library or load symbols");
    //     self.callbacks = self.dll.new();
    // }
}
