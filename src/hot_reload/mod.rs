mod dll;
pub use dll::*;
use notify_debouncer_mini::notify::{self, Watcher};
extern crate dlopen;
use std::{env, path::PathBuf, sync::mpsc, time};

fn format_dll_input() -> PathBuf {
    let current_exe = env::current_exe().expect("could not get current exe path");
    let folder_path = current_exe
        .parent()
        .expect("could not get current exe parent folder");
    let file_name = current_exe
        .file_stem()
        .expect("could not get current exe file stem");
    let file_name = file_name
        .to_str()
        .expect("could not convert os string to &str")
        .replace("-", "_");
    let path = PathBuf::new()
        .join(folder_path)
        .join(dlopen::utils::platform_file_name(file_name));

    path
}

fn format_dll_output(dll_index: u32) -> PathBuf {
    let current_exe = env::current_exe().expect("could not get current exe path");
    let folder_path = current_exe
        .parent()
        .expect("could not get current exe parent folder");
    let file_name = current_exe
        .file_stem()
        .expect("could not get current exe file stem");
    let file_name = file_name
        .to_str()
        .expect("could not convert os string to &str")
        .replace("-", "_");

    let file_name = format!("{file_name}_{dll_index}");

    let path = PathBuf::new()
        .join(folder_path)
        .join(dlopen::utils::platform_file_name(file_name));

    path
}

pub(crate) struct HotReloadContext {
    force_reload: bool,
    force_restart: bool,

    #[allow(dead_code)]
    dll_watcher: notify::RecommendedWatcher, // keep reference alive
    dll_update_channel: mpsc::Receiver<()>,
    dll_last_update: time::Instant,
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

        let mut dll_watcher = notify::recommended_watcher(move |res| match res {
            Ok(_) => {
                tx.send(()).expect("could not send dll change event");
            }
            Err(err) => println!("debounced result error: {}", err),
        })
        .expect("could not create hot reload dll watcher");

        dll_watcher
            .watch(&format_dll_input(), notify::RecursiveMode::NonRecursive)
            .unwrap_or_else(|err| {
                panic!(
                    "could not watch {}: {:?}",
                    format_dll_input().display(),
                    err
                )
            });

        Self {
            force_reload: false,
            force_restart: false,
            dll_last_update: time::Instant::now(),
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

    pub(crate) fn should_reload(&mut self) -> bool {
        if self.force_reload {
            return true;
        }

        if !self.dll_changed() {
            return false;
        }

        const DLL_RELOAD_DELAY: f32 = 0.1;
        if self.dll_last_update.elapsed().as_secs_f32() < DLL_RELOAD_DELAY {
            return false;
        }

        self.dll_last_update = time::Instant::now();

        true
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
