use crate::{
    asset::{
        ArcAssetCacheRegistry, AssetCacheDependency, AssetCacheLoad, AssetCacheStorage,
        BackendToBackendMessage, DynAssetHandle,
    },
    filesystem::FileSystemRuntime,
};
use rustc_hash::{FxHashMap, FxHashSet};
use std::{collections::HashSet, path::PathBuf};

//
// Types
//

pub(crate) struct ReloadHandleRequest {
    pub(crate) path: PathBuf,
}

//
// Generic
//

pub struct AssetCacheReload {
    filesystem_ctx: FileSystemRuntime,

    // used to track if load responses are from reload or not
    currently_reloading: FxHashSet<DynAssetHandle>,

    /// which handles depend to a certain path
    reload_handles: FxHashMap<PathBuf, HashSet<DynAssetHandle>>,

    // keep watcher handle alive
    reload_watcher:
        notify_debouncer_mini::Debouncer<notify_debouncer_mini::notify::RecommendedWatcher>,
}

impl AssetCacheReload {
    pub(crate) fn new(
        filesystem_ctx: FileSystemRuntime,
        backend_message_sender: async_channel::Sender<BackendToBackendMessage>,
    ) -> Self {
        let reload_watcher = notify_debouncer_mini::new_debouncer(
            std::time::Duration::from_millis(100),
            move |res: notify_debouncer_mini::DebounceEventResult| match res {
                Ok(events) => {
                    for event in events {
                        backend_message_sender
                            .try_send(BackendToBackendMessage::Reload(ReloadHandleRequest {
                                path: event.path,
                            }))
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
            reload_handles: FxHashMap::default(),
        }
    }

    #[tracing::instrument(name = "reload", skip_all)]
    pub(crate) fn handle_path_changed(
        &mut self,
        path: &PathBuf,
        registry: &ArcAssetCacheRegistry,
        loader: &mut AssetCacheLoad,
        storage: &mut AssetCacheStorage,
    ) {
        tracing::info!("Path changed {:?}", path);
        if let Some(handles) = self.reload_handles.get(path) {
            tracing::info!("reload {:?}", handles);
            for handle in handles.clone() {
                self.reload(handle, registry, loader, storage);
            }
        }
    }

    /// Queue a reload just like file watcher would
    pub(crate) fn reload(
        &mut self,
        dyn_handle: DynAssetHandle,
        registry: &ArcAssetCacheRegistry,
        loader: &mut AssetCacheLoad,
        storage: &mut AssetCacheStorage,
    ) {
        tracing::info!("reload {}", dyn_handle);
        // mark as curretnly reloading
        // TODO: still needed?
        self.set_currently_reloading(dyn_handle.clone());
        loader.queue_load(registry, storage, dyn_handle.clone());
    }

    pub(crate) fn register_watches(
        &mut self,
        handle: DynAssetHandle,
        watches: &FxHashSet<PathBuf>,
    ) {
        // TODO: we should clear the previous watches
        // its not that problematic since we removce the dependecy on this, but we are still
        // updating this asset when noone depends on it, could be part of a clear unused handles
        // maybe?

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

    pub(crate) fn reload_dependents(
        &mut self,
        dependency: &mut AssetCacheDependency,
        registry: &ArcAssetCacheRegistry,
        loader: &mut AssetCacheLoad,
        storage: &mut AssetCacheStorage,
        handle: &DynAssetHandle,
    ) {
        if let Some(dependents) = dependency.dependents(handle) {
            tracing::info!("reload dependents due to {}, {:?}", handle, dependents);

            for dependent in dependents.iter() {
                tracing::info!("reload {}", dependent);
                self.reload(dependent.clone(), registry, loader, storage);
            }
        }
    }

    fn set_currently_reloading(&mut self, handle: DynAssetHandle) {
        self.currently_reloading.insert(handle);
    }

    pub(crate) fn is_currently_reloading(&mut self, handle: &DynAssetHandle) -> bool {
        self.currently_reloading.remove(handle)
    }
}
