use crate::asset::{
    invalidate_render_cache, AssetLoader, DerivedAssetKey, DynAssetHandle, DynAssetLoadFn,
    DynDerivedAsset, LoadAssetResult, LoadContext,
};
use crate::{asset::AssetHandle, filesystem::FileSystemContext};
use rustc_hash::{FxHashMap, FxHashSet};
use std::any::TypeId;
use std::path::Path;
use std::path::PathBuf;

pub struct AssetCacheExt {
    /// which handles map to a certain path
    reload_handles: FxHashMap<PathBuf, Vec<DynAssetHandle>>,
    // functions for reloading handles sync
    // use same settings as when it was initially loaded
    reload_functions_sync: FxHashMap<DynAssetHandle, DynAssetLoadFn>,

    // channel for requesting reloads
    reload_sender: async_channel::Sender<PathBuf>,
    reload_receiver: async_channel::Receiver<PathBuf>,

    // keep watcher handle alive
    reload_watcher:
        notify_debouncer_mini::Debouncer<notify_debouncer_mini::notify::RecommendedWatcher>,
}

impl AssetCacheExt {
    pub fn new() -> Self {
        let (reload_sender, reload_receiver) = async_channel::unbounded();
        let sender_copy = reload_sender.clone();
        let reload_watcher = notify_debouncer_mini::new_debouncer(
            std::time::Duration::from_millis(100),
            move |res: notify_debouncer_mini::DebounceEventResult| match res {
                Ok(events) => {
                    for event in events {
                        sender_copy.try_send(event.path).expect("could not send");
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

            reload_handles: FxHashMap::default(),
            reload_functions_sync: FxHashMap::default(),
        }
    }

    pub fn register_load<T: AssetLoader + 'static>(&mut self, handle: DynAssetHandle, loader: T) {
        // store reload function
        self.reload_functions_sync
            .entry(handle.as_any())
            .or_insert_with(|| {
                Box::new(move |load_ctx, path| {
                    let result = pollster::block_on(loader.clone().load(load_ctx, path));
                    match result {
                        Ok(asset) => LoadAssetResult::Success(Box::new(asset)),
                        Err(err) => {
                            tracing::error!("could not reload asset {:?}: {}", path, err);
                            LoadAssetResult::Error
                        }
                    }
                })
            });
    }

    /// Register asset for being watched for hot reloads
    pub fn watch<T: AssetLoader + 'static>(
        &mut self,
        filesystem_ctx: &FileSystemContext,
        handle: AssetHandle<T::Asset>,
        path: &Path,
    ) {
        let path = filesystem_ctx.format_asset_path(path);
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
        handles.push(handle.as_any());
    }

    // checks if any files changed and spawns a thread which reloads the data
    pub fn poll_reload_sync(
        &mut self,
        cache: &mut FxHashMap<DynAssetHandle, LoadAssetResult>,
        render_cache: &mut FxHashMap<DerivedAssetKey, DynDerivedAsset>,
        render_cache_invalidate_lookup: &FxHashMap<DynAssetHandle, FxHashSet<TypeId>>,
        just_loaded: &mut FxHashSet<DynAssetHandle>,
        load_ctx: LoadContext,
    ) {
        while let Ok(path) = self.reload_receiver.try_recv() {
            if let Some(handles) = self.reload_handles.get_mut(&path) {
                for handle in handles {
                    println!("reload {:?}", path);
                    just_loaded.insert(handle.clone());

                    // load new fn
                    let loader_fn_sync = self
                        .reload_functions_sync
                        .get(&handle.as_any())
                        .expect("could not get loader fn");
                    let asset = loader_fn_sync(load_ctx.clone(), &path);

                    // insert into cache
                    cache.insert(handle.clone(), asset);

                    // invalidate render cache
                    invalidate_render_cache(
                        render_cache,
                        render_cache_invalidate_lookup,
                        handle.as_any(),
                    );
                }
            }
        }
    }

    /// Queue a reload just like file watcher would
    pub fn reload(&mut self, paths: &FxHashMap<DynAssetHandle, PathBuf>, handle: DynAssetHandle) {
        let Some(path) = paths.get(&handle.as_any()) else {
            tracing::warn!("could not get path for handle {:?}", handle.id());
            return;
        };

        self.reload_sender
            .try_send(path.clone())
            .expect("could not send reload request");
    }

    /// Immediately call the reload function sync
    pub fn reload_sync(
        &mut self,
        cache: &mut FxHashMap<DynAssetHandle, LoadAssetResult>,
        render_cache: &mut FxHashMap<DerivedAssetKey, DynDerivedAsset>,
        render_cache_invalidate_lookup: &FxHashMap<DynAssetHandle, FxHashSet<TypeId>>,
        paths: &FxHashMap<DynAssetHandle, PathBuf>,
        load_ctx: LoadContext,
        handle: DynAssetHandle,
    ) {
        let Some(path) = paths.get(&handle.as_any()) else {
            tracing::warn!("could not get path for handle {:?}", handle.id());
            return;
        };

        let Some(loader_fn_sync) = self.reload_functions_sync.get(&handle.as_any()) else {
            tracing::warn!("could not get asset handle {}", handle.id());
            return;
        };

        let asset = loader_fn_sync(load_ctx, path);

        cache.insert(handle.clone(), asset);
        invalidate_render_cache(render_cache, render_cache_invalidate_lookup, handle);
    }
}
