use super::{Asset, AssetLoader};
use crate::{
    asset::{
        self,
        convert::{AssetCacheDerived, ConvertAssetResult},
        AssetCacheLoad, AssetCacheStorage, AssetConverter, AssetHandle, ConvertAssetStatus,
        GetAssetResult, InsertAssetBuilder, LoadAssetBuilder, LoadAssetResult,
    },
    render::ArcHandle,
    Context,
};
use std::{
    any::TypeId,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

// TODO: maybe move this to load context
#[derive(Debug, Clone)]
pub struct AssetHandleContext {
    id: Arc<Mutex<u64>>,
}

impl AssetHandleContext {
    pub fn new() -> Self {
        Self {
            id: Arc::new(Mutex::new(0)),
        }
    }
    pub fn next_id(&self) -> u64 {
        let mut id_guard = self.id.lock().expect("could not unlock asset id lock");
        let id = *id_guard;
        *id_guard += 1;
        id
    }
}

pub struct AssetCache {
    asset_handle_ctx: AssetHandleContext,

    storage: AssetCacheStorage,

    loader: AssetCacheLoad,

    derived: AssetCacheDerived,

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) reloader: asset::reload::AssetCacheReload,
}

impl AssetCache {
    pub fn new(ctx: &Context) -> Self {
        let asset_handle_ctx = AssetHandleContext::new();

        let storage = AssetCacheStorage::new(asset_handle_ctx.clone());

        let loader = AssetCacheLoad::new(asset_handle_ctx.clone(), ctx.filesystem.clone());
        loader.start_background_loader();

        let derived = AssetCacheDerived::new();

        Self {
            asset_handle_ctx,

            storage,

            loader,
            derived,

            #[cfg(not(target_arch = "wasm32"))]
            reloader: asset::AssetCacheReload::new(),
        }
    }

    pub fn poll(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        self.reloader.poll_reload();

        self.loader
            .poll_loaded(&mut self.storage.cache, &mut self.derived);
    }

    pub(crate) fn load<T: AssetLoader + 'static>(
        &mut self,
        handle: AssetHandle<T::Asset>,
        path: &Path,
        settings: T::Settings,
    ) -> AssetHandle<T::Asset> {
        // set current status to loading
        self.storage
            .cache
            .insert(handle.as_any(), LoadAssetResult::Loading);

        // request load
        self.loader
            .request_load::<T>(handle.clone(), path, settings.clone());

        // register reload
        #[cfg(not(target_arch = "wasm32"))]
        self.reloader.register_reloadable::<T>(
            handle.clone(),
            path.to_path_buf(),
            settings.clone(),
            self.loader.load_ctx.clone(),
        );

        handle
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn load_sync<T: AssetLoader + Send + Sync + 'static>(
        &mut self,
        handle: AssetHandle<T::Asset>,
        path: &Path,
        settings: T::Settings,
    ) -> AssetHandle<T::Asset> {
        // load sync
        let data = pollster::block_on(T::load(
            self.loader.load_ctx.clone(),
            path,
            settings.clone(),
        ));
        match data {
            Ok(asset) => {
                self.storage
                    .cache
                    .insert(handle.as_any(), LoadAssetResult::Success(Box::new(asset)));
            }
            Err(err) => {
                tracing::error!("error loading asset {:?}: {}", path, err);
                self.storage
                    .cache
                    .insert(handle.as_any(), LoadAssetResult::Error);
            }
        }

        // TODO: should failed loads be put here?
        self.loader.just_loaded.insert(handle.as_any());

        // register reload
        #[cfg(not(target_arch = "wasm32"))]
        self.reloader.register_reloadable::<T>(
            handle.clone(),
            path.to_path_buf(),
            settings.clone(),
            self.loader.load_ctx.clone(),
        );

        handle
    }

    pub fn new_empty_handle<T>(&self) -> AssetHandle<T> {
        AssetHandle::new(&self.asset_handle_ctx)
    }

    //
    // Builders re-exports
    //

    pub fn insert_builder<T: Asset>(&mut self, value: T) -> InsertAssetBuilder<T> {
        asset::AssetBuilder::insert(value)
    }

    pub fn load_builder<T: AssetLoader<Settings: Default>>(
        &mut self,
        path: impl Into<PathBuf>,
    ) -> LoadAssetBuilder<T> {
        asset::AssetBuilder::load(path)
    }

    //
    // Storage re-exports
    //

    pub fn insert_new_handle<T: Asset + 'static>(&mut self, data: T) -> AssetHandle<T> {
        self.storage.insert_new_handle(data)
    }

    pub fn insert_existing_handle<T: Asset + 'static>(
        &mut self,
        data: T,
        handle: AssetHandle<T>,
    ) -> AssetHandle<T> {
        self.storage.insert_existing_handle(data, handle)
    }

    pub fn get<'a, T: Asset + 'static>(&'a self, handle: AssetHandle<T>) -> GetAssetResult<'a, T> {
        self.storage.get(handle)
    }

    pub fn handle_successfully_loaded<T: Asset>(&self, handle: AssetHandle<T>) -> bool {
        self.storage.handle_successfully_loaded(handle)
    }

    pub fn clear_handle<T: Asset>(&mut self, handle: AssetHandle<T>) {
        self.storage.clear_handle(handle);
    }

    pub fn clear_cpu_handles(&mut self) {
        self.storage.clear_unused_handles();
    }

    //
    // Load re-exports
    //

    pub fn handle_just_loaded<T: Asset>(&self, handle: AssetHandle<T>) -> bool {
        self.loader.handle_just_loaded(handle)
    }

    //
    // Convert re-exports
    //

    pub fn clear_derived_handles(&mut self) {
        self.derived.clear_unused_handles();
    }

    pub fn convert<G: AssetConverter>(
        &mut self,
        ctx: &mut Context,
        handle: AssetHandle<G::SourceAsset>,
        settings: &G::Settings,
    ) -> ConvertAssetResult<G::TargetAsset> {
        self.derived
            .convert::<G>(ctx, &self.storage, handle, settings)
    }

    //
    // Reload re-exports
    //

    /// Reload an existing asset while reusing the last path and loader
    #[cfg(not(target_arch = "wasm32"))]
    pub fn reload<T: AssetLoader + 'static>(&mut self, handle: AssetHandle<T::Asset>) {
        self.reloader.reload(handle.as_any());
    }

    /// Reload an existing asset while reusing the last path and loader
    #[cfg(not(target_arch = "wasm32"))]
    pub fn reload_sync<T: AssetLoader + 'static>(&mut self, handle: AssetHandle<T::Asset>) {
        self.reloader
            .reload_sync(&mut self.storage.cache, &mut self.derived, handle.as_any());
    }
}
