use super::{Asset, AssetLoader};
use crate::{
    asset::{
        self,
        derive::{AssetCacheDerived, ConvertAssetResult},
        AssetCacheLoad, AssetCacheStorage, AssetConverter, AssetHandle, AssetHandleContext,
        GetAssetResult, InsertAssetBuilder, LoadAssetBuilder, LoadAssetResult, LoadContext,
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

    #[cfg(not(target_arch = "wasm32"))]
    reloader: asset::reload::AssetCacheReload,
}

impl AssetCache {
    pub fn new(ctx: &Context) -> Self {
        let asset_handle_ctx = AssetHandleContext::new();
        let filesystem_ctx = ctx.filesystem.clone();
        let task_executor = ctx.task.clone();

        let storage = AssetCacheStorage::new(asset_handle_ctx.clone());

        let loader = AssetCacheLoad::new();

        let derived = AssetCacheDerived::new();

        #[cfg(not(target_arch = "wasm32"))]
        let reloader = asset::AssetCacheReload::new();

        Self {
            asset_handle_ctx,
            filesystem_ctx,
            task_executor,

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
            self.task_executor.clone(),
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
            .poll_loaded(&mut self.storage, &mut self.derived);
    }

    pub(crate) fn load<T: AssetLoader + 'static>(
        &mut self,
        handle: AssetHandle<T::Asset>,
        settings: T::Settings,
        #[cfg(not(target_arch = "wasm32"))] watch: bool,
    ) {
        let mut load_ctx = self.load_ctx(handle.clone());

        // TODO: not done when loading sub assets through load ctx
        self.storage
            .insert::<T::Asset>(handle.clone(), LoadAssetResult::Loading);

        #[cfg(not(target_arch = "wasm32"))]
        if watch {
            load_ctx.reload_ctx.enable_watch::<T>(
                load_ctx.clone(),
                handle.clone(),
                settings.clone(),
            );
        }

        load_ctx.load_asset_with_handle::<T>(handle, settings);
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

    pub fn get<'a, T: Asset + 'static>(
        &'a mut self,
        handle: AssetHandle<T>,
    ) -> GetAssetResult<'a, T> {
        self.storage.get(handle)
    }

    pub fn handle_successfully_loaded<T: Asset>(&mut self, handle: AssetHandle<T>) -> bool {
        self.storage.handle_successfully_loaded(handle)
    }

    pub fn clear_asset_handle<T: Asset>(&mut self, handle: AssetHandle<T>) {
        self.storage
            .clear_handle(&mut self.derived, handle.as_any());
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
        self.derived.convert::<G>(ctx, &mut self.storage, settings)
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
