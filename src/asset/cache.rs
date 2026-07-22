use super::{Asset, AssetLoader};
#[cfg(not(target_arch = "wasm32"))]
use crate::asset::{ReloadFnHandleRequest, WatchHandleRequest};
use crate::{
    asset::{
        self,
        derive::{AssetCacheDerived, ConvertAssetResult},
        AssetCacheLoad, AssetCacheStorage, AssetConverter, AssetHandle, GetAssetResult,
        InsertAssetBuilder, LoadAssetBuilder, LoadAssetResult, LoadContext, LoadRequest,
        LoadResponse,
    },
    filesystem::FileSystemContext,
    Context,
};
use std::{
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
    filesystem_ctx: FileSystemContext,

    storage: AssetCacheStorage,

    loader: AssetCacheLoad,

    derived: AssetCacheDerived,

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) reloader: asset::reload::AssetCacheReload,
}

impl AssetCache {
    pub fn new(ctx: &Context) -> Self {
        let asset_handle_ctx = AssetHandleContext::new();
        let filesystem_ctx = ctx.filesystem.clone();

        let storage = AssetCacheStorage::new(asset_handle_ctx.clone());

        let loader = AssetCacheLoad::new();
        loader.start_background_loader();

        let derived = AssetCacheDerived::new();

        #[cfg(not(target_arch = "wasm32"))]
        let reloader = asset::AssetCacheReload::new();

        Self {
            asset_handle_ctx,
            filesystem_ctx,

            storage,

            loader,
            derived,

            #[cfg(not(target_arch = "wasm32"))]
            reloader,
        }
    }

    pub fn load_ctx<T: Asset>(&self, handle: AssetHandle<T>) -> LoadContext {
        LoadContext::new(
            handle.as_any(),
            self.asset_handle_ctx.clone(),
            self.filesystem_ctx.clone(),
            &self.loader,
            #[cfg(not(target_arch = "wasm32"))]
            &self.reloader,
        )
    }

    pub fn poll(&mut self, ctx: &Context) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            // TODO: does order matter?
            self.reloader.poll_reload();
            self.reloader.poll_reload_fns();
            self.reloader.poll_watch(ctx.filesystem.clone());
        }

        self.loader
            .poll_loaded(&mut self.storage.cache, &mut self.derived);
    }

    // TODO: just call load ctx functions from here
    pub(crate) fn load<T: AssetLoader + 'static>(
        &mut self,
        handle: AssetHandle<T::Asset>,
        settings: T::Settings,
        #[cfg(not(target_arch = "wasm32"))] watch: bool,
    ) {
        let mut load_ctx = self.load_ctx(handle.clone());

        // TODO: not done when loading sub assets through load ctx
        self.storage
            .cache
            .insert(handle.as_any(), LoadAssetResult::Loading);

        #[cfg(not(target_arch = "wasm32"))]
        load_ctx.watch(watch);

        // register reload fns
        #[cfg(not(target_arch = "wasm32"))]
        load_ctx.register_reload_fns::<T>(handle.clone(), settings.clone());

        load_ctx.request_load_with_handle::<T>(handle.clone(), settings.clone());
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

    pub fn load_builder<T: AssetLoader>(&mut self) -> LoadAssetBuilder<T> {
        asset::AssetBuilder::load()
    }

    //
    // Storage re-exports
    //

    pub fn insert<T: Asset + 'static>(&mut self, data: T) -> AssetHandle<T> {
        self.storage.insert_new_handle(data)
    }

    pub fn overwrite_handle<T: Asset + 'static>(
        &mut self,
        data: T,
        handle: AssetHandle<T>,
    ) -> AssetHandle<T> {
        self.storage
            .insert_existing_handle(&mut self.derived, data, handle)
    }

    pub fn get<'a, T: Asset + 'static>(&'a self, handle: AssetHandle<T>) -> GetAssetResult<'a, T> {
        self.storage.get(handle)
    }

    pub fn handle_successfully_loaded<T: Asset>(&self, handle: AssetHandle<T>) -> bool {
        self.storage.handle_successfully_loaded(handle)
    }

    pub fn clear_asset_handle<T: Asset>(&mut self, handle: AssetHandle<T>) {
        self.storage.clear_handle(handle);
    }

    pub fn clear_asset_handles(&mut self) {
        self.storage.clear_unused_handles();
    }

    //
    // Load re-exports
    //

    pub fn handle_just_loaded<T: Asset>(&self, handle: AssetHandle<T>) -> bool {
        self.loader.handle_just_loaded(handle)
    }

    //
    // Derive re-exports
    //

    pub fn clear_derived_handles(&mut self) {
        // TODO:
        // self.derived.clear_unused_handles();
    }

    pub fn convert<G: AssetConverter + 'static>(
        &mut self,
        ctx: &mut Context,
        settings: &G::Settings,
    ) -> ConvertAssetResult<G::TargetAsset> {
        self.derived.convert::<G>(ctx, &self.storage, settings)
    }

    //
    // Reload re-exports
    //

    /// Reload an existing asset while reusing the last path and loader
    #[cfg(not(target_arch = "wasm32"))]
    pub fn reload<T: AssetLoader + 'static>(&mut self, handle: AssetHandle<T::Asset>) {
        self.reloader.reload(handle.as_any());
    }
}
