use crate::asset::{
    invalidate_render_cache, AssetLoader, DerivedAssetKey, DynAssetHandle, DynAssetLoadFn,
    DynAssetLoadFnSync, DynDerivedAsset, LoadAssetResult, LoadContext,
};
use crate::{asset::AssetHandle, filesystem::FileSystemContext};
use rustc_hash::{FxHashMap, FxHashSet};
use std::any::TypeId;
use std::future::Future;
use std::path::Path;
use std::path::PathBuf;

pub struct AssetCacheExt {
    /// which handles map to a certain path
    reload_handles: FxHashMap<PathBuf, Vec<DynAssetHandle>>,
    // functions for reloading handles sync
    // use same settings as when it was initially loaded
    reload_functions: FxHashMap<DynAssetHandle, DynAssetLoadFn>,
    reload_functions_sync: FxHashMap<DynAssetHandle, DynAssetLoadFnSync>,

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
            reload_functions: FxHashMap::default(),
            reload_functions_sync: FxHashMap::default(),
        }
    }

    pub fn register_load<T, F, R>(
        &mut self,
        load_ctx: LoadContext,
        handle: AssetHandle<T::Asset>,
        path: PathBuf,
        settings: T::Settings,

        spawn_load_fn: F,
    ) where
        T: AssetLoader + 'static,
        F: Fn(LoadContext, AssetHandle<T::Asset>, PathBuf, T::Settings) -> R
            + Send
            + Sync
            + 'static
            + Clone,
        R: Future<Output = ()>,
    {
        let path_clone = path.clone();
        let handle_clone = handle.clone();
        let load_ctx_clone = load_ctx.clone();
        let settings_clone = settings.clone();

        // NOTE:
        // this currently captures load_ctx, path and settings
        // load_ctx and path can be used as paramters if they are stored
        // and loaded from handles hash maps
        // settings wont know about the type so it must be captured
        // storing it as dyn Any and downcasting might work, currently I
        // dont see the benefit it gives so ill keep it like this for now
        //
        // store reload functions async
        self.reload_functions
            .entry(handle.as_any())
            .or_insert_with(|| {
                Box::new(move || {
                    let path_clone = path_clone.clone();
                    let handle_clone = handle_clone.clone();
                    let load_ctx_clone = load_ctx_clone.clone();
                    let settings_clone = settings_clone.clone();
                    let spawn_load_fn_clone = spawn_load_fn.clone();

                    // load async
                    std::thread::spawn(move || {
                        pollster::block_on(spawn_load_fn_clone(
                            load_ctx_clone,
                            handle_clone,
                            path_clone,
                            settings_clone,
                        ))
                    });
                })
            });

        // store reload function sync
        let path_clone = path.clone();
        let load_ctx_clone = load_ctx.clone();
        let settings_clone = settings.clone();
        self.reload_functions_sync
            .entry(handle.as_any())
            .or_insert_with(|| {
                Box::new(move || {
                    let result = pollster::block_on(T::load(
                        load_ctx_clone.clone(),
                        &path_clone,
                        settings_clone.clone(),
                    ));

                    match result {
                        Ok(asset) => LoadAssetResult::Success(Box::new(asset)),
                        Err(err) => {
                            tracing::error!("could not reload asset {:?}: {}", path_clone, err);
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
    pub fn poll_reload(&mut self) {
        while let Ok(path) = self.reload_receiver.try_recv() {
            if let Some(handles) = self.reload_handles.get(&path) {
                for handle in handles.clone() {
                    self.reload(handle.as_any());
                }
            }
        }
    }

    /// Queue a reload just like file watcher would
    pub fn reload(&mut self, handle: DynAssetHandle) {
        let Some(loader_fn_sync) = self.reload_functions.get(&handle.as_any()) else {
            tracing::warn!("could not get asset handle {}", handle.id());
            return;
        };

        loader_fn_sync();
    }

    /// Immediately call the reload function sync
    pub fn reload_sync(
        &mut self,
        cache: &mut FxHashMap<DynAssetHandle, LoadAssetResult>,
        render_cache: &mut FxHashMap<DerivedAssetKey, DynDerivedAsset>,
        render_cache_invalidate_lookup: &FxHashMap<DynAssetHandle, FxHashSet<TypeId>>,
        handle: DynAssetHandle,
    ) {
        let Some(loader_fn_sync) = self.reload_functions_sync.get(&handle.as_any()) else {
            tracing::warn!("could not get asset handle {}", handle.id());
            return;
        };

        let asset = loader_fn_sync();

        cache.insert(handle.clone(), asset);
        invalidate_render_cache(render_cache, render_cache_invalidate_lookup, handle);
    }
}
