use super::{Asset, AssetLoader};
use crate::{
    asset::{
        self, AssetCacheDependency, AssetCacheDerivedConvert, AssetCacheLoad, AssetCacheRegistry,
        AssetCacheStorage, AssetConverter, AssetHandle, AssetHandleContext, GetAssetResult,
        InsertAssetBuilder, LoadAssetBuilder, LoadContext, LoadRuntime, LoadState, LoadStatus,
    },
    filesystem::FileSystemContext,
    Context,
};

pub struct AssetCache {
    asset_handle_ctx: AssetHandleContext,
    filesystem_ctx: FileSystemContext,

    storage: AssetCacheStorage,
    loader: AssetCacheLoad,

    pub derived_convert: AssetCacheDerivedConvert,
    pub registry: AssetCacheRegistry,

    dependency: AssetCacheDependency,
    #[cfg(not(target_arch = "wasm32"))]
    reloader: asset::reload::AssetCacheReload,
}

impl AssetCache {
    pub fn new(ctx: &Context) -> Self {
        let asset_handle_ctx = AssetHandleContext::new();
        let filesystem_ctx = ctx.filesystem.clone();
        let task_executor = ctx.task.clone();

        let storage = AssetCacheStorage::new(asset_handle_ctx.clone());

        let loader = AssetCacheLoad::new(
            task_executor.clone(),
            filesystem_ctx.clone(),
            asset_handle_ctx.clone(),
        );

        let derived_convert = AssetCacheDerivedConvert::new(asset_handle_ctx.clone());
        let registry = AssetCacheRegistry::new(asset_handle_ctx.clone());

        let dependency = AssetCacheDependency::new();

        #[cfg(not(target_arch = "wasm32"))]
        let reloader = asset::AssetCacheReload::new(filesystem_ctx.clone());

        Self {
            asset_handle_ctx,
            filesystem_ctx,

            storage,

            loader,
            dependency,

            derived_convert,
            registry,

            #[cfg(not(target_arch = "wasm32"))]
            reloader,
        }
    }

    pub fn load_ctx<T: Asset>(&self, handle: AssetHandle<T>) -> LoadContext {
        let state = LoadState::new(handle.to_dyn());
        let runtime = LoadRuntime::new(
            self.asset_handle_ctx.clone(),
            self.filesystem_ctx.clone(),
            &self.loader,
        );

        LoadContext::new(state, runtime)
    }

    // TODO: does order matter?
    pub fn poll(&mut self, ctx: &mut Context) {
        #[cfg(not(target_arch = "wasm32"))]
        self.reloader
            .poll_reload(&mut self.loader, &mut self.registry);

        // loading
        self.loader.poll_handle_requests(&mut self.registry);

        self.loader.poll_loaded(
            &mut self.storage,
            &mut self.registry,
            &mut self.derived_convert,
            &mut self.dependency,
            #[cfg(not(target_arch = "wasm32"))]
            &mut self.reloader,
        );

        // derived
        self.derived_convert.poll_conversions(
            ctx,
            &mut self.storage,
            &mut self.loader,
            &mut self.dependency,
            &mut self.registry,
        );
    }

    pub(crate) fn load<T: AssetLoader + 'static>(
        &mut self,
        handle: AssetHandle<T::Asset>,
        settings: T::Settings,
    ) {
        self.loader
            .load_asset_with_handle::<T>(&mut self.registry, handle, settings);
    }

    pub fn new_empty_handle<T: Asset>(&self) -> AssetHandle<T> {
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
        self.storage.insert_successful_new_handle(data)
    }

    pub fn get<T: Asset + 'static>(&mut self, handle: &AssetHandle<T>) -> GetAssetResult<'_, T> {
        if let Some(success) = self.storage.get(handle) {
            return GetAssetResult::Success(success);
        }

        match self.registry.get_status(&handle.to_dyn()) {
            LoadStatus::Loading => GetAssetResult::Loading,
            LoadStatus::Failed => GetAssetResult::Error,
        }
    }

    pub fn handle_successfully_loaded<T: Asset>(&mut self, handle: AssetHandle<T>) -> bool {
        self.storage.handle_successfully_loaded(handle)
    }

    pub fn clear_asset_handle<T: Asset>(&mut self, handle: AssetHandle<T>) {
        todo!()
        // self.storage.clear_handle(&mut self.derived, handle);
    }

    pub fn clear_asset_handles(&mut self) {
        todo!()
        // self.storage.clear_unused_handles(&mut self.derived);
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

    pub fn convert<T: AssetConverter + 'static>(
        &mut self,
        settings: T::Settings,
    ) -> AssetHandle<T::Asset> {
        self.derived_convert
            .register_conversion::<T>(&mut self.registry, settings)
    }

    //
    // Reload re-exports
    //

    /// Reload an existing asset while reusing the last path and loader
    #[cfg(not(target_arch = "wasm32"))]
    pub fn reload<T: AssetLoader + 'static>(&mut self, handle: AssetHandle<T::Asset>) {
        self.reloader
            .reload(handle.to_dyn(), &mut self.loader, &mut self.registry);
    }
}
