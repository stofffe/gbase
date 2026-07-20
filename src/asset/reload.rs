use crate::asset::{DynAssetHandle, DynAssetLoadFn};
use crate::filesystem::FileSystemContext;
use rustc_hash::FxHashMap;
use std::collections::HashSet;
use std::path::PathBuf;

pub struct AssetCacheReload {
    /// which handles map to a certain path
    reload_handles: FxHashMap<PathBuf, HashSet<DynAssetHandle>>,

    // functions for reloading handles sync
    // use same settings as when it was initially loaded
    reload_functions: FxHashMap<DynAssetHandle, DynAssetLoadFn>,

    // channel for requesting reloads
    pub(crate) reload_sender: async_channel::Sender<ReloadHandleRequest>,
    reload_receiver: async_channel::Receiver<ReloadHandleRequest>,
    // watch
    pub(crate) watch_sender: async_channel::Sender<WatchHandleRequest>,
    watch_receiver: async_channel::Receiver<WatchHandleRequest>,
    // register
    pub(crate) reload_fn_sender: async_channel::Sender<ReloadFnHandleRequest>,
    reload_fn_receiver: async_channel::Receiver<ReloadFnHandleRequest>,

    // keep watcher handle alive
    reload_watcher:
        notify_debouncer_mini::Debouncer<notify_debouncer_mini::notify::RecommendedWatcher>,
}

pub struct ReloadHandleRequest {
    path: PathBuf,
}

pub struct WatchHandleRequest {
    pub handle: DynAssetHandle,
    pub path: PathBuf,
}

pub struct ReloadFnHandleRequest {
    pub handle: DynAssetHandle,
    pub load_fn: DynAssetLoadFn,
}

impl AssetCacheReload {
    pub fn new() -> Self {
        let (reload_sender, reload_receiver) = async_channel::unbounded();
        let (watch_sender, watch_receiver) = async_channel::unbounded();
        let (reload_fn_sender, reload_fn_receiver) = async_channel::unbounded();

        let reload_sender_clone = reload_sender.clone();
        let reload_watcher = notify_debouncer_mini::new_debouncer(
            std::time::Duration::from_millis(100),
            move |res: notify_debouncer_mini::DebounceEventResult| match res {
                Ok(events) => {
                    for event in events {
                        reload_sender_clone
                            .try_send(ReloadHandleRequest { path: event.path })
                            .expect("could not send");
                    }
                }
                Err(err) => println!("debounced result error: {}", err),
            },
        )
        .expect("could not create watcher");

        Self {
            reload_watcher,

            reload_sender,
            reload_receiver,

            watch_sender,
            watch_receiver,
            reload_fn_sender,
            reload_fn_receiver,

            reload_handles: FxHashMap::default(),
            reload_functions: FxHashMap::default(),
        }
    }

    // checks if any files changed and spawns a thread which reloads the data
    pub fn poll_reload(&mut self) {
        while let Ok(reload_request) = self.reload_receiver.try_recv() {
            if let Some(handles) = self.reload_handles.get(&reload_request.path) {
                for handle in handles.clone() {
                    self.reload(handle.as_any());
                }
            }
        }
    }

    pub fn poll_reload_fns(&mut self) {
        while let Ok(reload_fn_request) = self.reload_fn_receiver.try_recv() {
            tracing::info!(
                "RECEIVE RELOAD FN for handle {}",
                reload_fn_request.handle.id()
            );
            self.reload_functions
                .insert(reload_fn_request.handle.clone(), reload_fn_request.load_fn);
        }
    }

    pub fn poll_watch(&mut self, filesystem_ctx: FileSystemContext) {
        while let Ok(watch_request) = self.watch_receiver.try_recv() {
            tracing::info!(
                "RECEIVE REGISTER WATCH FOR {}",
                watch_request.path.display()
            );

            let path = filesystem_ctx.format_asset_path(watch_request.path);
            // path must be canoicalized since watcher will do it internally
            let path = std::fs::canonicalize(path).unwrap();

            // start watching path
            self.reload_watcher
                .watcher()
                .watch(
                    &path,
                    notify_debouncer_mini::notify::RecursiveMode::NonRecursive, // recursive mode does not matter for files
                )
                .unwrap_or_else(|err| panic!("could not watch {}: {:?}", path.display(), err));

            // map path to handle
            let handles = self.reload_handles.entry(path).or_default();
            handles.insert(watch_request.handle);
        }
    }

    /// Queue a reload just like file watcher would
    pub fn reload(&mut self, handle: DynAssetHandle) {
        let Some(reload_fn) = self.reload_functions.get(&handle.as_any()) else {
            tracing::warn!("could not get asset handle {}", handle.id());
            return;
        };

        reload_fn();
    }
}
