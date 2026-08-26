use crate::asset::AssetCacheLoad;
use crate::asset::{AssetCacheConvert, AssetCacheDependency, AssetCacheRegistry, DynAssetHandle};
use crate::filesystem::FileSystemContext;
use core::panic;
use rustc_hash::{FxHashMap, FxHashSet};
use std::collections::HashSet;
use std::path::PathBuf;

//
// Types
//

struct ReloadHandleRequest {
    path: PathBuf,
}

//
// Generic
//

pub struct AssetCacheReload {
    filesystem_ctx: FileSystemContext,

    // used to track if load responses are from reload or not
    currently_reloading: FxHashSet<DynAssetHandle>,

    /// which handles depend to a certain path
    reload_handles: FxHashMap<PathBuf, HashSet<DynAssetHandle>>,

    // channel for requesting reloads
    reload_receiver: async_channel::Receiver<ReloadHandleRequest>,

    // keep watcher handle alive
    reload_watcher:
        notify_debouncer_mini::Debouncer<notify_debouncer_mini::notify::RecommendedWatcher>,
}

impl AssetCacheReload {
    pub(crate) fn new(filesystem_ctx: FileSystemContext) -> Self {
        let (reload_sender, reload_receiver) = async_channel::unbounded();

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
            filesystem_ctx,
            currently_reloading: FxHashSet::default(),
            reload_watcher,

            reload_receiver,

            reload_handles: FxHashMap::default(),
        }
    }

    // checks if any files changed and spawns a thread which reloads the data
    pub(crate) fn poll_reload(
        &mut self,
        loader: &mut AssetCacheLoad,
        converter: &mut AssetCacheConvert,
        registry: &mut AssetCacheRegistry,
    ) {
        while let Ok(reload_request) = self.reload_receiver.try_recv() {
            if let Some(handles) = self.reload_handles.get(&reload_request.path) {
                for handle in handles.clone() {
                    tracing::info!("POLL RELOAD FOR {:?}", reload_request.path);
                    self.reload(handle, loader, converter, registry);
                }
            }
        }
    }

    /// Queue a reload just like file watcher would
    pub(crate) fn reload(
        &mut self,
        dyn_handle: DynAssetHandle,
        loader: &mut AssetCacheLoad,
        converter: &mut AssetCacheConvert,
        registry: &mut AssetCacheRegistry,
    ) {
        // mark as curretnly reloading
        self.set_currently_reloading(dyn_handle.clone());

        let is_loader = registry.is_loader(&dyn_handle);
        let is_converter = registry.is_converter(&dyn_handle);
        match (is_loader, is_converter) {
            (true, false) => loader.queue_load(registry, dyn_handle.clone()),
            (false, true) => converter.queue_conversion(registry, dyn_handle),
            (true, true) => panic!("a handle cant be both a loader and a converter"),
            (false, false) => panic!("a handle must be either a loader or converter"),
        }
    }

    pub(crate) fn register_watches(
        &mut self,
        handle: DynAssetHandle,
        watches: &FxHashSet<PathBuf>,
    ) {
        for watch in watches.iter() {
            self.register_watch(watch.to_path_buf(), handle.clone());
        }
    }

    /// start watching path and notify handle when path changes
    fn register_watch(&mut self, path: PathBuf, handle: DynAssetHandle) {
        let path = self.filesystem_ctx.format_asset_path(path);
        // path must be canoicalized since watcher will do it internally
        let path = match std::fs::canonicalize(&path) {
            Ok(path) => path,
            Err(err) => {
                tracing::warn!("could not canoicalize path: {:?}: {}", &path, err);
                tracing::warn!("skipping watching");
                return;
            }
        };

        let handles = self.reload_handles.entry(path.clone()).or_default();

        // start watching file path if its not done already
        if handles.is_empty() {
            tracing::info!("start watching {:?}", &path);
            self.reload_watcher
                .watcher()
                .watch(
                    &path,
                    notify_debouncer_mini::notify::RecursiveMode::NonRecursive, // recursive mode does not matter for files
                )
                .unwrap_or_else(|err| panic!("could not watch {}: {:?}", path.display(), err));
        }

        handles.insert(handle);
    }

    pub fn reload_dependents(
        &mut self,
        dependency: &mut AssetCacheDependency,
        loader: &mut AssetCacheLoad,
        converter: &mut AssetCacheConvert,
        registry: &mut AssetCacheRegistry,
        handle: &DynAssetHandle,
    ) {
        if let Some(dependents) = dependency.dependents(handle) {
            tracing::info!("reload dependents due to {}, {:?}", handle, dependents);

            for dependent in dependents.iter() {
                tracing::info!("reload {}", dependent);
                self.reload(dependent.clone(), loader, converter, registry);
            }
        }
    }

    fn set_currently_reloading(&mut self, handle: DynAssetHandle) {
        self.currently_reloading.insert(handle);
    }

    pub fn is_currently_reloading(&mut self, handle: &DynAssetHandle) -> bool {
        self.currently_reloading.remove(handle)
    }
}
