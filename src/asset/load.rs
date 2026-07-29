#[cfg(not(target_arch = "wasm32"))]
use crate::asset::{AssetCacheReload, ReloadContext};

use crate::{
    asset::{
        dependency, derive::AssetCacheDerived, Asset, AssetCacheDependency, AssetCacheStorage,
        AssetHandle, AssetHandleContext, DynAsset, DynAssetHandle,
    },
    filesystem, task,
};
use rustc_hash::{FxHashMap, FxHashSet};
use std::future::Future;
use std::{error, path::Path};

//
// Types
//

pub trait AssetSettings: Send + Clone {}
impl<T: Send + Clone> AssetSettings for T {} // TODO: maybe do this for Asset and derived asset

pub trait AssetError: error::Error + Send {}
impl<T: error::Error + Send> AssetError for T {} // TODO: maybe do this for Asset and derived asset

pub trait AssetLoader: Send {
    type Asset: Asset;
    type Settings: AssetSettings;
    type Error: AssetError;

    #[cfg(not(target_arch = "wasm32"))]
    fn load(
        load_ctx: &mut LoadContext,
        settings: Self::Settings,
    ) -> impl Future<Output = Result<Self::Asset, Self::Error>> + Send;

    #[cfg(target_arch = "wasm32")]
    fn load(
        load_ctx: &mut LoadContext,
        settings: Self::Settings,
    ) -> impl Future<Output = Result<Self::Asset, Self::Error>>;
}

pub type DynAssetLoadFn = Box<dyn Fn() + Send>;

pub enum LoadAssetResult<T: Asset> {
    Loading,
    Success(T),
    Error,
}

pub enum DynLoadAssetResult {
    Loading,
    Success(DynAsset),
    Error,
}

pub struct LoadResponse<T: Asset> {
    pub(crate) handle: AssetHandle<T>,
    pub(crate) result: LoadAssetResult<T>,
    pub(crate) dependencies: FxHashSet<DynAssetHandle>,
}

pub trait DynLoadResponse: Send {
    fn insert_into_storage(
        self: Box<Self>,
        storage: &mut AssetCacheStorage,
        loader: &mut AssetCacheLoad,
        derived: &mut AssetCacheDerived,
        dependency: &mut AssetCacheDependency,
    );
    fn handle(&self) -> DynAssetHandle;
    fn success(&self) -> bool;
}

impl<T: Asset> DynLoadResponse for LoadResponse<T> {
    fn insert_into_storage(
        self: Box<Self>,
        storage: &mut AssetCacheStorage,
        loader: &mut AssetCacheLoad,
        derived: &mut AssetCacheDerived,
        dependency: &mut AssetCacheDependency,
    ) {
        match self.result {
            LoadAssetResult::Success(asset) => {
                let handle = self.handle.as_any();

                storage.insert(self.handle.clone(), asset);

                loader.remove_status(&handle);
                loader.just_loaded.insert(handle.clone());

                derived.invalidate_derived_assets_depending_on_handle(handle.clone());

                dependency.add_dependencies(&handle, &self.dependencies);

                tracing::info!(
                    "ASSET {} had {} deps",
                    self.handle.id(),
                    self.dependencies.len()
                );
            }
            LoadAssetResult::Error => {
                loader.set_status(&self.handle, LoadStatus::Failed);
            }
            LoadAssetResult::Loading => {
                tracing::error!("RECEIVED LOAD");
                loader.set_status(&self.handle.as_any(), LoadStatus::Loading);
            }
        }
    }

    fn handle(&self) -> DynAssetHandle {
        self.handle.as_any()
    }

    fn success(&self) -> bool {
        matches!(self.result, LoadAssetResult::Success(_))
    }
}

//
// Load
//

#[derive(Clone)]
pub enum LoadStatus {
    Loading,
    Failed,
}

pub struct AssetCacheLoad {
    pub(crate) response_sender: async_channel::Sender<Box<dyn DynLoadResponse>>,
    pub(crate) response_receiver: async_channel::Receiver<Box<dyn DynLoadResponse>>,

    // TODO: maybe these should be derived from cache every frame? O(n)
    // TODO: should failed loads be put here?
    pub(crate) just_loaded: FxHashSet<DynAssetHandle>,
    pub(crate) status: FxHashMap<DynAssetHandle, LoadStatus>,
}

impl AssetCacheLoad {
    pub(crate) fn new() -> Self {
        let (response_sender, response_receiver) = async_channel::unbounded();

        let just_loaded = FxHashSet::default();
        let status = FxHashMap::default();

        Self {
            just_loaded,
            status,

            response_sender,
            response_receiver,
        }
    }

    pub fn set_status<T: Asset>(&mut self, handle: &AssetHandle<T>, status: LoadStatus) {
        self.status.insert(handle.as_any(), status);
    }

    pub fn get_status<T: Asset>(&mut self, handle: &AssetHandle<T>) -> LoadStatus {
        if let Some(status) = self.status.get(&handle.as_any()) {
            status.clone()
        } else {
            LoadStatus::Failed
        }
    }

    pub fn remove_status(&mut self, handle: &DynAssetHandle) {
        self.status.remove(handle);
    }

    pub fn handle_just_loaded<T: Asset>(&self, handle: AssetHandle<T>) -> bool {
        self.just_loaded.contains(&handle.as_any())
    }

    // check if any files completed loading and update cache and invalidate render cache
    pub fn poll_loaded(
        &mut self,
        storage: &mut AssetCacheStorage,
        derived: &mut AssetCacheDerived,
        dependency: &mut AssetCacheDependency,
    ) {
        self.just_loaded.clear();

        while let Ok(response) = self.response_receiver.try_recv() {
            response.insert_into_storage(storage, self, derived, dependency);
        }
    }
}

//
// Load context
//

#[derive(Clone)]
pub struct LoadState {
    pub(crate) handle: DynAssetHandle,
    pub(crate) dependencies: FxHashSet<DynAssetHandle>,
}

impl LoadState {
    pub fn new(handle: DynAssetHandle) -> Self {
        Self {
            handle,
            dependencies: FxHashSet::default(),
        }
    }
}

#[derive(Clone)]
pub struct LoadRuntime {
    pub(crate) asset_handle_ctx: AssetHandleContext,
    pub(crate) filesystem_ctx: filesystem::FileSystemContext,
    pub(crate) task_ctx: task::TaskContext,

    pub(crate) load_response_sender: async_channel::Sender<Box<dyn DynLoadResponse>>,

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) reload_ctx: ReloadContext,
}

impl LoadRuntime {
    pub fn new(
        asset_handle_ctx: AssetHandleContext,
        filesystem_ctx: filesystem::FileSystemContext,
        task_ctx: task::TaskContext,
        loader: &AssetCacheLoad,
        #[cfg(not(target_arch = "wasm32"))] reloader: &AssetCacheReload,
    ) -> Self {
        let load_response_sender = loader.response_sender.clone();
        Self {
            asset_handle_ctx,
            filesystem_ctx,
            task_ctx,
            load_response_sender,

            #[cfg(not(target_arch = "wasm32"))]
            reload_ctx: ReloadContext::new(reloader),
        }
    }
}

#[derive(Clone)]
pub struct LoadContext {
    state: LoadState,
    runtime: LoadRuntime,
}

impl LoadContext {
    pub fn new(state: LoadState, runtime: LoadRuntime) -> Self {
        Self { runtime, state }
    }

    pub fn dependencies(&self) -> &FxHashSet<DynAssetHandle> {
        &self.state.dependencies
    }

    // // TODO: kinda unclear name
    // pub fn new_from_existing(&self, handle: DynAssetHandle) -> Self {
    //     let state = LoadState {
    //         handle,
    //         dependencies: FxHashSet::default(),
    //     };
    //     let runtime = self.runtime.clone();
    //     Self { runtime, state }
    // }

    pub fn insert_asset<T: Asset>(&self, value: T) -> AssetHandle<T> {
        let handle = AssetHandle::<T>::new(&self.runtime.asset_handle_ctx);
        self.runtime
            .load_response_sender
            .try_send(Box::new(LoadResponse {
                handle: handle.clone(),
                result: LoadAssetResult::Success(value),
                dependencies: FxHashSet::default(),
            }))
            .expect("could not send asset handle");
        handle
    }

    /// Request load with new handle
    pub fn load_asset<T: AssetLoader + 'static>(
        &mut self,
        settings: T::Settings,
    ) -> AssetHandle<T::Asset> {
        let handle = AssetHandle::new(&self.runtime.asset_handle_ctx);

        self.load_asset_with_handle::<T>(handle.clone(), settings);

        handle
    }

    /// Request load with existing handle
    pub fn load_asset_with_handle<T: AssetLoader + 'static>(
        &mut self,
        handle: AssetHandle<T::Asset>,
        settings: T::Settings,
    ) {
        self.load_asset_func::<T>(handle.clone(), settings);
    }

    pub fn load_asset_func<T: AssetLoader + 'static>(
        &mut self,
        handle: AssetHandle<T::Asset>,
        settings: T::Settings,
    ) {
        let new_asset_state = LoadState::new(handle.as_any());
        let mut new_load_ctx = LoadContext::new(new_asset_state, self.runtime.clone());

        // register for reloading
        #[cfg(not(target_arch = "wasm32"))]
        self.runtime.reload_ctx.register_reload_fns::<T>(
            new_load_ctx.clone(),
            handle.clone(),
            settings.clone(),
        );

        // set asset to loading
        new_load_ctx
            .runtime
            .load_response_sender
            .try_send(Box::new(LoadResponse {
                handle: handle.clone(),
                result: LoadAssetResult::Loading,
                dependencies: FxHashSet::default(),
            }))
            .expect("could not send asset loading response");

        // add dependency to parent
        self.state.dependencies.insert(handle.as_any());

        // spawn load
        self.runtime.task_ctx.spawn_task(Box::pin(async move {
            let data = T::load(&mut new_load_ctx, settings).await;

            match data {
                Ok(asset) => {
                    new_load_ctx
                        .runtime
                        .load_response_sender
                        .send(Box::new(LoadResponse {
                            handle: handle.clone(),
                            result: LoadAssetResult::Success(asset),
                            dependencies: new_load_ctx.state.dependencies,
                        }))
                        .await
                        .expect("could not send load success response");
                }
                Err(err) => {
                    tracing::warn!("could not load asset {}", err);
                    new_load_ctx
                        .runtime
                        .load_response_sender
                        .send(Box::new(LoadResponse {
                            handle: handle.clone(),
                            result: LoadAssetResult::Error,
                            dependencies: new_load_ctx.state.dependencies,
                        }))
                        .await
                        .expect("could not send load error response");
                }
            }
        }));
    }

    pub async fn load_bytes(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<Vec<u8>, filesystem::LoadFileError> {
        let result = self.runtime.filesystem_ctx.load_asset_bytes(&path).await;

        if result.is_ok() {
            #[cfg(not(target_arch = "wasm32"))]
            self.runtime
                .reload_ctx
                .register_watch(self.state.handle.clone(), path.as_ref().to_path_buf())
                .await;
        }

        result
    }

    pub async fn load_string(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<String, filesystem::LoadFileError> {
        let result = self.runtime.filesystem_ctx.load_asset_string(&path).await;

        if result.is_ok() {
            #[cfg(not(target_arch = "wasm32"))]
            self.runtime
                .reload_ctx
                .register_watch(self.state.handle.clone(), path.as_ref().to_path_buf())
                .await;
        }

        result
    }
}
