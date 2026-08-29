use super::{Asset, AssetLoader};
use crate::{
    asset::{
        self, AssetCacheConvert, AssetCacheDependency, AssetCacheInsert, AssetCacheLoad,
        AssetCacheRegistry, AssetCacheStorage, AssetConverter, AssetHandle, AssetHandleContext,
        AssetInserter, AssetState,
    },
    filesystem::FileSystemContext,
    Context,
};

pub struct AssetCache {
    asset_handle_ctx: AssetHandleContext,
    filesystem_ctx: FileSystemContext,

    storage: AssetCacheStorage,

    inserter: AssetCacheInsert,
    loader: AssetCacheLoad,
    converter: AssetCacheConvert,
    registry: AssetCacheRegistry,

    dependency: AssetCacheDependency,

    #[cfg(not(target_arch = "wasm32"))]
    reloader: asset::reload::AssetCacheReload,
}

impl AssetCache {
    pub(crate) fn new(ctx: &Context) -> Self {
        let asset_handle_ctx = AssetHandleContext::new();
        let filesystem_ctx = ctx.filesystem.clone();
        let task_executor = ctx.task.clone();

        let storage = AssetCacheStorage::new();

        let inserter = AssetCacheInsert::new();
        let loader = AssetCacheLoad::new(task_executor.clone(), filesystem_ctx.clone());
        let converter = AssetCacheConvert::new();

        let registry = AssetCacheRegistry::new(asset_handle_ctx.clone());

        let dependency = AssetCacheDependency::new();

        #[cfg(not(target_arch = "wasm32"))]
        let reloader = asset::AssetCacheReload::new(filesystem_ctx.clone());

        Self {
            asset_handle_ctx,
            filesystem_ctx,

            storage,

            inserter,
            loader,
            converter,

            registry,

            dependency,

            #[cfg(not(target_arch = "wasm32"))]
            reloader,
        }
    }

    pub(crate) fn poll(&mut self, ctx: &mut Context) {
        // reload
        #[cfg(not(target_arch = "wasm32"))]
        self.reloader
            .poll_reload(&mut self.loader, &mut self.converter, &mut self.registry);

        // registry
        self.registry.clear_just_available();

        // loading
        self.loader.poll_load_requests(&mut self.registry);
        self.loader
            .poll_insert_requests(&mut self.registry, &mut self.storage, &mut self.inserter);
        self.loader.poll_loaded(
            &mut self.storage,
            &mut self.registry,
            &mut self.converter,
            &mut self.dependency,
            #[cfg(not(target_arch = "wasm32"))]
            &mut self.reloader,
        );
        self.loader.poll_queue_loads(&mut self.registry);

        // convert
        self.converter.poll_conversions(
            ctx,
            &mut self.storage,
            &mut self.loader,
            &mut self.dependency,
            &mut self.registry,
        );
    }

    //
    // Storage re-exports
    //

    pub fn insert_asset<T: Asset + 'static, I: AssetInserter + 'static>(
        &mut self,
        key: impl Into<I::Key>,
        asset: T,
    ) -> AssetHandle<T> {
        self.inserter
            .insert_asset::<T, I>(&mut self.registry, &mut self.storage, key.into(), asset)
    }

    pub fn insert_asset_force<T: Asset + 'static>(&mut self, asset: T) -> AssetHandle<T> {
        self.inserter.insert_asset_with_new_handle::<T>(
            &mut self.registry,
            &mut self.storage,
            asset,
        )
    }

    pub fn load_asset<T: AssetLoader + 'static>(
        &mut self,
        settings: &T::Settings,
    ) -> AssetHandle<T::Asset> {
        tracing::info!("register load {:?}", settings);
        self.loader.register_load::<T>(&mut self.registry, settings)
    }

    pub fn convert_asset<T: AssetConverter + 'static>(
        &mut self,
        settings: &T::Settings,
    ) -> AssetHandle<T::Asset> {
        self.converter
            .register_conversion::<T>(&mut self.registry, settings)
    }

    pub fn get_asset<T: Asset + 'static>(
        &mut self,
        handle: &AssetHandle<T>,
    ) -> Result<&T, AssetState> {
        if let Some(asset) = self.storage.get_asset(handle) {
            return Ok(asset);
        }

        let state = self.registry.get_status(handle.to_dyn());
        match state {
            AssetState::Loading => {
                tracing::info!("waiting for {}", handle);
            }
            AssetState::Failed => {
                tracing::info!("erron in {}", handle);
            }
            AssetState::Ready => panic!(
                "could not get asset from storage but status is ready {}",
                handle
            ),
            AssetState::NotRegistered => panic!("trying to get unregistered asset {}", handle),
        }
        Err(state)
    }

    pub fn get_or_convert_asset<T: AssetConverter + 'static>(
        &mut self,
        settings: &T::Settings,
    ) -> Result<&T::Asset, AssetState> {
        let handle = self.convert_asset::<T>(settings);
        self.get_asset(&handle)
    }

    pub fn handle_successfully_loaded<T: Asset>(&mut self, handle: &AssetHandle<T>) -> bool {
        let status = self.registry.get_status(handle.to_dyn());
        matches!(status, AssetState::Ready)
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
    // Dependency re-exports
    //

    pub fn debug_asset_dependency_graph(&self) {
        self.dependency.debug_graph();
    }

    //
    // Load re-exports
    //

    // TODO: does this keep re registering?
    // can and should this overwrite?

    pub fn handle_just_loaded<T: Asset>(&self, handle: &AssetHandle<T>) -> bool {
        self.registry.handle_just_available(&handle.to_dyn())
    }

    //
    // Convert re-exports
    //

    pub fn clear_handles(&mut self) {
        // TODO:
        // self.derived.clear_unused_handles();
    }

    //
    // Reload re-exports
    //

    /// Reload an existing asset while reusing the last path and loader
    #[cfg(not(target_arch = "wasm32"))]
    pub fn reload<T: AssetLoader + 'static>(&mut self, handle: AssetHandle<T::Asset>) {
        self.reloader.reload(
            handle.to_dyn(),
            &mut self.loader,
            &mut self.converter,
            &mut self.registry,
        );
    }
}
