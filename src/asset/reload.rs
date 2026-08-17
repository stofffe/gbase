use crate::asset::DynAssetHandle;
use crate::asset::{AssetCacheLoad, AssetLoader};
use crate::filesystem::FileSystemContext;
use rustc_hash::FxHashMap;
use std::any::{Any, TypeId};
use std::collections::HashSet;
use std::marker::PhantomData;
use std::path::PathBuf;

pub trait DynAssetReload {
    fn reload_handle(&self, loader: &mut AssetCacheLoad, dyn_handle: DynAssetHandle);

    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

pub struct TypedAssetReload<T: AssetLoader> {
    ty: PhantomData<T>,
}

impl<T: AssetLoader + 'static> TypedAssetReload<T> {
    pub fn new() -> Self {
        Self { ty: PhantomData }
    }
}

impl<T: AssetLoader + 'static> DynAssetReload for TypedAssetReload<T> {
    fn reload_handle(&self, loader: &mut AssetCacheLoad, dyn_handle: DynAssetHandle) {
        let Some(handle) = dyn_handle.to_typed::<T::Asset>() else {
            tracing::warn!(
                "trying to convert DynAssetHandle with type {:?} to a AssetHandle with type {:?}",
                dyn_handle.type_id(),
                TypeId::of::<T::Asset>()
            );
            return;
        };

        let Some(typed_loader) = loader.get_typed_cache_ref::<T>() else {
            tracing::warn!(
                "trying to reload handle {:?} but no typed loader exists",
                handle.id
            );
            return;
        };

        let Some(settings) = typed_loader.handle_to_settings.get(&handle) else {
            tracing::warn!(
                "trying to get settings for {:?} but none were found",
                handle.id
            );
            return;
        };

        loader.load_asset_with_handle::<T>(handle, settings.clone());
    }

    fn as_any(&self) -> &dyn Any {
        self as &dyn Any
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self as &mut dyn Any
    }
}

pub struct AssetCacheReload {
    filesystem_ctx: FileSystemContext,

    handle_to_loader_type: FxHashMap<DynAssetHandle, TypeId>,
    typed_reloaders: FxHashMap<TypeId, Box<dyn DynAssetReload>>,

    /// which handles depend to a certain path
    reload_handles: FxHashMap<PathBuf, HashSet<DynAssetHandle>>,

    // channel for requesting reloads
    reload_receiver: async_channel::Receiver<ReloadHandleRequest>,

    // keep watcher handle alive
    reload_watcher:
        notify_debouncer_mini::Debouncer<notify_debouncer_mini::notify::RecommendedWatcher>,
}

pub struct ReloadHandleRequest {
    path: PathBuf,
}

impl AssetCacheReload {
    pub fn new(filesystem_ctx: FileSystemContext) -> Self {
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
            handle_to_loader_type: FxHashMap::default(),
            typed_reloaders: FxHashMap::default(),
            reload_watcher,

            reload_receiver,

            reload_handles: FxHashMap::default(),
        }
    }

    pub fn get_typed_cache_ref<T: AssetLoader + 'static>(&self) -> Option<&TypedAssetReload<T>> {
        self.typed_reloaders
            .get(&TypeId::of::<T>())
            .map(|dyn_reloader| {
                dyn_reloader
                    .as_any()
                    .downcast_ref::<TypedAssetReload<T>>()
                    .expect("could not downcast typed storage cache")
            })
    }

    /// Get mutable typed cache or create if it doesnt exist
    pub fn get_typed_cache_mut<T: AssetLoader + 'static>(&mut self) -> &mut TypedAssetReload<T> {
        let entry = self
            .typed_reloaders
            .entry(TypeId::of::<T>())
            .or_insert(Box::new(TypedAssetReload::<T>::new()));
        entry
            .as_any_mut()
            .downcast_mut::<TypedAssetReload<T>>()
            .expect("could not downcast typed storage cache")
    }

    // checks if any files changed and spawns a thread which reloads the data
    pub fn poll_reload(&mut self, loader: &mut AssetCacheLoad) {
        while let Ok(reload_request) = self.reload_receiver.try_recv() {
            if let Some(handles) = self.reload_handles.get(&reload_request.path) {
                for handle in handles.clone() {
                    tracing::info!("POLL RELOAD FOR {:?}", reload_request.path);
                    self.reload(handle, loader);
                }
            }
        }
    }

    /// Queue a reload just like file watcher would
    pub fn reload(&mut self, handle: DynAssetHandle, loader: &mut AssetCacheLoad) {
        let Some(loader_type_id) = self.handle_to_loader_type.get(&handle) else {
            tracing::warn!("could not get loader type id for {}", handle.id());
            return;
        };

        let Some(dyn_reloader) = self.typed_reloaders.get(loader_type_id) else {
            tracing::warn!("could not get typed reloader for {:?}", loader_type_id);
            return;
        };

        dyn_reloader.reload_handle(loader, handle);
    }

    /// register last loader type used to load handle
    pub fn register_loader_type<T: AssetLoader + 'static>(&mut self, handle: DynAssetHandle) {
        self.handle_to_loader_type.insert(handle, TypeId::of::<T>());

        self.get_typed_cache_mut::<T>();
    }

    /// start watching path and notify handle when path changes
    pub fn add_watch(&mut self, path: PathBuf, handle: DynAssetHandle) {
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
}
