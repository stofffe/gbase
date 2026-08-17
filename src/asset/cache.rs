use super::{Asset, AssetLoader};
use crate::{
    asset::{
        self,
        derive::{AssetCacheDerived, ConvertAssetResult},
        AssetCacheDependency, AssetCacheLoad, AssetCacheStorage, AssetConverter, AssetHandle,
        AssetHandleContext, GetAssetResult, InsertAssetBuilder, LoadAssetBuilder, LoadContext,
        LoadRuntime, LoadState,
    },
    filesystem::FileSystemContext,
    task::TaskContext,
    Context,
};

pub struct AssetCache {
    asset_handle_ctx: AssetHandleContext,
    filesystem_ctx: FileSystemContext,
    task_executor: TaskContext,

    storage: AssetCacheStorage,

    loader: AssetCacheLoad,

    derived: AssetCacheDerived,

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

        let derived = AssetCacheDerived::new();

        let dependency = AssetCacheDependency::new();

        #[cfg(not(target_arch = "wasm32"))]
        let reloader = asset::AssetCacheReload::new(filesystem_ctx.clone());

        Self {
            asset_handle_ctx,
            filesystem_ctx,
            task_executor,

            storage,

            loader,
            derived,
            dependency,

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
    pub fn poll(&mut self, ctx: &Context) {
        #[cfg(not(target_arch = "wasm32"))]
        self.reloader.poll_reload(&mut self.loader);

        self.loader.poll_handle_request();
        self.loader.poll_loaded(
            &mut self.storage,
            &mut self.derived,
            &mut self.dependency,
            #[cfg(not(target_arch = "wasm32"))]
            &mut self.reloader,
        );
    }

    pub(crate) fn load<T: AssetLoader + 'static>(
        &mut self,
        handle: AssetHandle<T::Asset>,
        settings: T::Settings,
    ) {
        self.loader.load_asset_with_handle::<T>(handle, settings);
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

    pub fn overwrite_handle<T: Asset + 'static>(
        &mut self,
        data: T,
        handle: AssetHandle<T>,
    ) -> AssetHandle<T> {
        self.storage
            .insert_successful_existing_handle(&mut self.derived, handle, data)
    }

    pub fn get<T: Asset + 'static>(&mut self, handle: &AssetHandle<T>) -> GetAssetResult<'_, T> {
        if let Some(success) = self.storage.get(handle) {
            return GetAssetResult::Success(success);
        }

        match self.loader.get_status(handle) {
            asset::LoadStatus::Loading => GetAssetResult::Loading,
            asset::LoadStatus::Failed => GetAssetResult::Error,
        }
    }

    pub fn handle_successfully_loaded<T: Asset>(&mut self, handle: AssetHandle<T>) -> bool {
        self.storage.handle_successfully_loaded(handle)
    }

    pub fn clear_asset_handle<T: Asset>(&mut self, handle: AssetHandle<T>) {
        self.storage.clear_handle(&mut self.derived, handle);
    }

    pub fn clear_asset_handles(&mut self) {
        self.storage.clear_unused_handles(&mut self.derived);
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
        self.derived
            .convert::<G>(ctx, &mut self.storage, &mut self.loader, settings)
    }

    //
    // Reload re-exports
    //

    /// Reload an existing asset while reusing the last path and loader
    #[cfg(not(target_arch = "wasm32"))]
    pub fn reload<T: AssetLoader + 'static>(&mut self, handle: AssetHandle<T::Asset>) {
        self.reloader.reload(handle.to_dyn(), &mut self.loader);
    }
}
