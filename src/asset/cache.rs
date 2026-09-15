use super::{Asset, AssetLoader};
use crate::{
    asset::{
        self, AssetCacheDependency, AssetCacheInsert, AssetCacheLoad, AssetCacheRegistry,
        AssetCacheStorage, AssetHandle, AssetHandleContext, AssetInserter, DynAssetHandle,
        GetAssetState, InternalAssetState,
    },
    Context,
};

pub struct AssetCache {
    storage: AssetCacheStorage,

    inserter: AssetCacheInsert,
    loader: AssetCacheLoad,
    registry: AssetCacheRegistry,

    dependency: AssetCacheDependency,

    #[cfg(not(target_arch = "wasm32"))]
    reloader: asset::reload::AssetCacheReload,
}

impl AssetCache {
    pub(crate) fn new(ctx: &Context) -> Self {
        let asset_handle_ctx = AssetHandleContext::new();
        let filesystem_runtime = ctx.filesystem.runtime();
        let render_runtime = ctx.render.runtime();
        let arc_runtime = ctx.arc.runtime();

        let task_executor = ctx.task.runtime();

        let storage = AssetCacheStorage::new();

        let inserter = AssetCacheInsert::new();
        let loader = AssetCacheLoad::new(
            task_executor.clone(),
            filesystem_runtime.clone(),
            render_runtime.clone(),
            arc_runtime.clone(),
        );

        let registry = AssetCacheRegistry::new(asset_handle_ctx.clone());

        let dependency = AssetCacheDependency::new();

        #[cfg(not(target_arch = "wasm32"))]
        let reloader = asset::AssetCacheReload::new(filesystem_runtime.clone());

        Self {
            storage,

            inserter,
            loader,

            registry,

            dependency,

            #[cfg(not(target_arch = "wasm32"))]
            reloader,
        }
    }

    pub(crate) fn poll(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        self.reloader
            .poll_reload(&mut self.loader, &mut self.storage);

        self.storage.clear_just_available();

        self.loader.poll_set_debug_name(&mut self.storage);
        self.loader.poll_get_requests(&mut self.storage);
        self.loader
            .poll_load_requests(&mut self.registry, &mut self.storage);
        self.loader.poll_insert_requests(
            &mut self.registry,
            &mut self.storage,
            &mut self.inserter,
            &mut self.dependency,
        );

        self.loader.poll_loaded(
            &mut self.storage,
            &mut self.dependency,
            #[cfg(not(target_arch = "wasm32"))]
            &mut self.reloader,
        );
        self.loader.poll_queue_loads(&mut self.registry);
    }

    /// Insert an asset and reuse any handles matching the same key
    pub fn insert_asset<T: Asset + 'static, I: AssetInserter + 'static>(
        &mut self,
        key: impl Into<I::Key>,
        asset: T,
    ) -> AssetHandle<T> {
        self.inserter
            .insert_asset::<T, I>(&mut self.registry, &mut self.storage, key.into(), asset)
    }

    /// Insert an asset without checking for cached handles
    pub fn insert_asset_force<T: Asset + 'static>(&mut self, asset: T) -> AssetHandle<T> {
        self.inserter.insert_asset_with_new_handle::<T>(
            &mut self.registry,
            &mut self.storage,
            asset,
        )
    }

    /// Request an asset load
    pub fn load_asset<T: AssetLoader + 'static>(
        &mut self,
        settings: &T::Settings,
    ) -> AssetHandle<T::Asset> {
        self.loader
            .register_load::<T>(&mut self.registry, &mut self.storage, settings)
    }

    pub fn get_asset_cloned<T: Asset + Clone + 'static>(
        &mut self,
        handle: &AssetHandle<T>,
    ) -> Result<T, GetAssetState> {
        self.get_asset(handle).cloned()
    }

    /// Get an asset
    pub fn get_asset<T: Asset + 'static>(
        &mut self,
        handle: &AssetHandle<T>,
    ) -> Result<&T, GetAssetState> {
        if let Some(asset) = self.storage.get_asset(handle) {
            return Ok(asset);
        }

        match self.storage.get_asset_state(&handle.to_dyn()) {
            InternalAssetState::Loading => {
                tracing::info!("waiting for {}", handle);
                Err(GetAssetState::Loading)
            }
            InternalAssetState::Failed => {
                tracing::info!("error in {}", handle);
                Err(GetAssetState::Failed)
            }
            InternalAssetState::Ready => {
                panic!(
                    "could not get asset from storage but status is ready {}",
                    handle
                );
            }
            InternalAssetState::Pending => {
                panic!("trying to get pending asset {}", handle);
            }
        }
    }

    /// Returns wheter a handle is available for reading
    pub fn handle_available<T: Asset>(&mut self, handle: &AssetHandle<T>) -> bool {
        let status = self.storage.get_asset_state(&handle.to_dyn());
        matches!(status, InternalAssetState::Ready)
    }

    pub fn clear_handle<T: Asset>(&mut self, handle: &AssetHandle<T>) {
        self.storage.clear_asset::<T>(handle);
        self.storage
            .set_asset_state(&handle.to_dyn(), InternalAssetState::Pending);
        // TODO: probably more
    }

    // TODO:  is this needed?
    // can check arc strong count
    // might not work since handles can depend on eachother
    // maybe use weak references in certain cases
    pub fn clear_unused_handles(&mut self) {
        todo!()
        // self.storage.clear_unused_handles(&mut self.derived);
    }

    pub fn set_debug_name<T: Asset>(&mut self, handle: &AssetHandle<T>, name: impl Into<String>) {
        self.storage.set_debug_name(&handle.to_dyn(), name);
    }

    pub fn get_debug_name<T: Asset>(&self, handle: &AssetHandle<T>) -> Option<&str> {
        self.storage.get_debug_name(&handle.to_dyn())
    }

    pub fn debug_asset_dependency_graph(&self) {
        self.dependency.debug_dependencies(&self.storage);
    }

    /// Return if the handle became ready this frame
    pub fn handle_just_available<T: Asset>(&self, handle: &AssetHandle<T>) -> bool {
        self.storage.handle_just_available(&handle.to_dyn())
    }

    /// Reload an existing asset while reusing the last path and loader
    #[cfg(not(target_arch = "wasm32"))]
    pub fn reload<T: Asset + 'static>(&mut self, handle: &AssetHandle<T>) {
        self.reloader
            .reload(handle.to_dyn(), &mut self.loader, &mut self.storage);
    }

    fn get_all_handles_sorted(&self) -> Vec<DynAssetHandle> {
        let mut dyn_handles = self.storage.get_all_assets();

        // empty check
        if dyn_handles.is_empty() {
            tracing::warn!("trying to debug assets, but none exist");
        }

        // sort based on handle id
        dyn_handles.sort();

        dyn_handles
    }

    pub fn debug_reloadable(&self) {
        let root_handles = self.get_all_handles_sorted();

        for root_handle in root_handles {
            tracing::info!("");

            // print root
            let debug_name = self
                .storage
                .get_debug_name(&root_handle)
                .unwrap_or("no name");

            tracing::info!("{} {}", root_handle.id(), debug_name);
        }
    }

    pub fn debug_assets(&self, show_dependencies: bool, show_recursive_dependencies: bool) {
        let root_handles = self.get_all_handles_sorted();

        for root_handle in root_handles {
            tracing::info!("");

            // print root
            let debug_name = self
                .storage
                .get_debug_name(&root_handle)
                .unwrap_or("no name");

            tracing::info!("{} {}", root_handle.id(), debug_name);

            let mut stack = Vec::new();

            if show_dependencies {
                if let Some(dependencies) = self.dependency.dependencies(&root_handle) {
                    let mut dependencies: Vec<_> = dependencies.iter().cloned().collect();
                    dependencies.sort();

                    // Dependencies are accessed LIFO and therefore the iteration order is reversed
                    // So do .rev() to undo this
                    for (i, dependency) in dependencies.iter().enumerate().rev() {
                        let is_last = i == dependencies.len() - 1;
                        let depth = 1; // hardcoded depth for root children
                        let prefix = String::new(); // no prefix from root
                        stack.push((dependency.clone(), prefix, depth, is_last));
                    }
                }
            }

            while let Some((dyn_handle, prefix, depth, is_last)) = stack.pop() {
                let debug_name = self
                    .storage
                    .get_debug_name(&dyn_handle)
                    .unwrap_or("no name");
                let id = dyn_handle.id();
                let connector = if is_last { "└── " } else { "├── " };

                tracing::info!("{}{}{} {}", prefix, connector, id, debug_name);

                if show_recursive_dependencies {
                    if let Some(dependencies) = self.dependency.dependencies(&dyn_handle) {
                        let mut dependencies: Vec<_> = dependencies.iter().cloned().collect();
                        dependencies.sort();

                        // Dependencies are accessed LIFO and therefore the iteration order is reversed
                        // So do .rev() to undo this
                        for (i, dependency) in dependencies.iter().enumerate().rev() {
                            // note this is_last refers to the currently popped/parent dependency
                            // if its not last you need a pipe going past the comming sub dependencies to the next dependency
                            let prefix = if is_last {
                                format!("{}    ", prefix).to_string()
                            } else {
                                format!("{}│   ", prefix).to_string()
                            };
                            let depth = depth + 1;
                            let is_last = i == dependencies.len() - 1;
                            stack.push((dependency.clone(), prefix, depth, is_last));
                        }
                    }
                }
            }
        }
    }
}
