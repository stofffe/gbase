mod dll;
pub use dll::*;
extern crate dlopen;
use notify::Watcher;
use std::{path::Path, sync::mpsc};

const DLL_NAME: &str = "libhot_reload.dylib";

pub(crate) struct HotReloadContext {
    force_reload: bool,
    force_restart: bool,

    #[allow(dead_code)]
    dll_watcher: notify::FsEventWatcher, // keep reference alive
    dll_change_channel: mpsc::Receiver<Result<notify::Event, notify::Error>>,
}

impl HotReloadContext {
    pub(crate) fn new() -> Self {
        let (tx, rx) = mpsc::channel();

        let mut watcher = notify::recommended_watcher(tx).unwrap();
        watcher
            .watch(Path::new(DLL_NAME), notify::RecursiveMode::NonRecursive)
            .unwrap();

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

    pub(crate) fn reset(&mut self) {
        self.force_reload = false;
        self.force_restart = false;
    }
}

//

//
// Commands
//

pub fn hot_reload(ctx: &mut crate::Context) {
    ctx.hot_reload.force_reload = true;
}

pub fn hot_restart(ctx: &mut crate::Context) {
    ctx.hot_reload.force_restart = true;
}
