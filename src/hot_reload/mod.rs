mod dll;
pub use dll::*;
extern crate dlopen;
use std::{path::Path, sync::mpsc, time::Duration};

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

    #[allow(dead_code)]
    dll_watcher:
        notify_debouncer_mini::Debouncer<notify_debouncer_mini::notify::RecommendedWatcher>, // keep reference alive
    dll_update_channel: mpsc::Receiver<()>,
}

impl HotReloadContext {
    pub(crate) fn clear_state(&mut self) {
        self.force_reload = false;
        self.force_restart = false;
    }
}

impl HotReloadContext {
    pub(crate) fn new() -> Self {
        tracing::info!("Hot Reload is enabled");

        let (tx, rx) = mpsc::channel();

        let mut dll_watcher = notify_debouncer_mini::new_debouncer(
            Duration::from_millis(100),
            move |res: notify_debouncer_mini::DebounceEventResult| match res {
                Ok(_) => tx.send(()).expect("could not send dll change event"),
                Err(err) => println!("debounced result error: {}", err),
            },
        )
        .expect("could not create watcher");

        dll_watcher
            .watcher()
            .watch(
                Path::new(&dllname()),
                notify_debouncer_mini::notify::RecursiveMode::NonRecursive,
            )
            .expect("could not watch dll");

        Self {
            force_reload: false,
            force_restart: false,
            dll_watcher,
            dll_update_channel: rx,
        }
    }

    fn dll_changed(&self) -> bool {
        if self.dll_update_channel.try_recv().is_ok() {
            return true;
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
