mod dll;
pub use dll::*;
extern crate dlopen;
use notify::Watcher;
use std::{path::Path, sync::mpsc};

pub(crate) fn dllname() -> String {
    let dll_name = std::env::current_exe()
        .unwrap()
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    dlopen::utils::platform_file_name(dll_name)
        .to_str()
        .unwrap()
        .to_string()
}

pub(crate) struct HotReloadContext {
    force_reload: bool,
    force_restart: bool,

    dll_watcher: notify::FsEventWatcher, // keep reference alive
    dll_change_channel: mpsc::Receiver<Result<notify::Event, notify::Error>>,
}

impl HotReloadContext {
    pub(crate) fn pre_update(&mut self) {
        self.force_reload = false;
        self.force_restart = false;
    }
}

impl HotReloadContext {
    pub(crate) fn new() -> Self {
        tracing::info!("Hot Reload is enabled");

        let (tx, rx) = mpsc::channel();

        let mut watcher = notify::recommended_watcher(tx).expect("could not create file watcher");
        watcher
            .watch(Path::new(&dllname()), notify::RecursiveMode::NonRecursive)
            .expect("could not watch dll");

        Self {
            force_reload: false,
            force_restart: false,
            dll_watcher: watcher,
            dll_change_channel: rx,
        }
    }

    fn dll_changed(&self) -> bool {
        if let Ok(Ok(event)) = self.dll_change_channel.try_recv() {
            if let notify::EventKind::Modify(_) | notify::EventKind::Create(_) = event.kind {
                return true;
            }
        }

        false
    }

    pub(crate) fn should_reload(&self) -> bool {
        self.dll_changed() || self.force_reload
    }
    pub(crate) fn should_restart(&self) -> bool {
        self.force_restart
    }
}

//
// Commands
//

pub fn hot_reload(ctx: &mut crate::Context) {
    ctx.hot_reload.force_reload = true;
}

pub fn hot_restart(ctx: &mut crate::Context) {
    ctx.hot_reload.force_restart = true;
}
