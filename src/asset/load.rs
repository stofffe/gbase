#[cfg(not(target_arch = "wasm32"))]
use crate::asset::{AssetCacheReload, ReloadContext};

use crate::{
    asset::{
        derive::AssetCacheDerived, Asset, AssetCacheStorage, AssetHandle, AssetHandleContext,
        DynAsset, DynAssetHandle,
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
        load_ctx: LoadContext,
        settings: Self::Settings,
    ) -> impl Future<Output = Result<Self::Asset, Self::Error>> + Send;

    #[cfg(target_arch = "wasm32")]
    fn load(
        load_ctx: LoadContext,
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
}

pub trait DynLoadResponse: Send {
    fn insert_into_storage(
        self: Box<Self>,
        storage: &mut AssetCacheStorage,
        loader: &mut AssetCacheLoad,
        derived: &mut AssetCacheDerived,
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
    ) {
        match self.result {
            LoadAssetResult::Success(asset) => {
                storage.insert(self.handle.clone(), asset);

                loader.status.remove(&self.handle.as_any());
                loader.just_loaded.insert(self.handle.as_any());

                derived.invalidate_derived_assets_depending_on_handle(self.handle.as_any());
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

// impl<T: AssetLoader> TypedAssetLoader<T> {
//     pub fn new() -> Self {
//         Self {}
//     }
//
//     pub fn handle_just_loaded(&self, handle: AssetHandle<T>) -> bool {
//         self.just_loaded.contains(&handle.as_any())
//     }
//
//     // check if any files completed loading and update cache and invalidate render cache
//     pub fn poll_loaded(
//         &mut self,
//         storage: &mut AssetCacheStorage,
//         derived: &mut AssetCacheDerived,
//     ) {
//         self.just_loaded.clear();
//
//         while let Ok(response) = self.response_receiver.try_recv() {
//             if let LoadAssetResult::Success(_) = response.result {
//                 self.just_loaded.insert(response.handle.clone());
//             }
//
//             storage.insert(response.handle.clone(), response.result);
//
//             derived.invalidate_derived_assets_depending_on_handle(response.handle.clone());
//         }
//     }
// }

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

    pub fn handle_just_loaded<T: Asset>(&self, handle: AssetHandle<T>) -> bool {
        self.just_loaded.contains(&handle.as_any())
    }

    // check if any files completed loading and update cache and invalidate render cache
    pub fn poll_loaded(
        &mut self,
        storage: &mut AssetCacheStorage,
        derived: &mut AssetCacheDerived,
    ) {
        self.just_loaded.clear();

        while let Ok(response) = self.response_receiver.try_recv() {
            response.insert_into_storage(storage, self, derived);
        }
    }
}

//
// Load context
//

#[derive(Clone)]
pub struct LoadContext {
    pub(crate) handle: DynAssetHandle,

    pub(crate) asset_handle_ctx: AssetHandleContext,
    pub(crate) filesystem_ctx: filesystem::FileSystemContext,
    pub(crate) task_ctx: task::TaskContext,

    pub(crate) load_response_sender: async_channel::Sender<Box<dyn DynLoadResponse>>,

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) reload_ctx: ReloadContext,
}

impl LoadContext {
    pub fn new(
        handle: DynAssetHandle,

        asset_handle_ctx: AssetHandleContext,
        filesystem_ctx: filesystem::FileSystemContext,
        task_ctx: task::TaskContext,

        loader: &AssetCacheLoad,

        #[cfg(not(target_arch = "wasm32"))] reloader: &AssetCacheReload,
    ) -> Self {
        let load_response_sender = loader.response_sender.clone();

        #[cfg(not(target_arch = "wasm32"))]
        let reload_ctx = ReloadContext::new(reloader);

        Self {
            handle,

            asset_handle_ctx,
            filesystem_ctx,
            task_ctx,

            load_response_sender,

            #[cfg(not(target_arch = "wasm32"))]
            reload_ctx,
        }
    }

    pub fn clone_with_new_handle(&self, handle: DynAssetHandle) -> Self {
        Self {
            handle,
            asset_handle_ctx: self.asset_handle_ctx.clone(),
            filesystem_ctx: self.filesystem_ctx.clone(),
            task_ctx: self.task_ctx.clone(),
            load_response_sender: self.load_response_sender.clone(),

            #[cfg(not(target_arch = "wasm32"))]
            reload_ctx: self.reload_ctx.clone(),
        }
    }

    pub fn insert_asset<T: Asset>(&self, value: T) -> AssetHandle<T> {
        let handle = AssetHandle::<T>::new(&self.asset_handle_ctx);
        self.load_response_sender
            .try_send(Box::new(LoadResponse {
                handle: handle.clone(),
                result: LoadAssetResult::Success(value),
            }))
            .expect("could not send asset handle");
        handle
    }

    /// Request load with new handle
    pub fn load_asset<T: AssetLoader + 'static>(
        &self,
        settings: T::Settings,
    ) -> AssetHandle<T::Asset> {
        let handle = AssetHandle::new(&self.asset_handle_ctx);

        self.load_asset_with_handle::<T>(handle.clone(), settings);

        handle
    }

    /// Request load with existing handle
    // TODO: does not set status to loading (maybe fine?)
    pub fn load_asset_with_handle<T: AssetLoader + 'static>(
        &self,
        handle: AssetHandle<T::Asset>,
        settings: T::Settings,
    ) {
        #[cfg(not(target_arch = "wasm32"))]
        self.reload_ctx
            .register_reload_fns::<T>(self.clone(), handle.clone(), settings.clone());

        self.load_asset_func::<T>(handle.clone(), settings);
    }

    pub fn load_asset_func<T: AssetLoader + 'static>(
        &self,
        handle: AssetHandle<T::Asset>,
        settings: T::Settings,
    ) {
        let load_ctx = self.clone_with_new_handle(handle.as_any());
        let load_response_sender = self.load_response_sender.clone();

        load_response_sender
            .try_send(Box::new(LoadResponse {
                handle: handle.clone(),
                result: LoadAssetResult::Loading,
            }))
            .expect("could not send asset loading response");

        self.task_ctx.spawn_task(Box::pin(async move {
            let data = T::load(load_ctx, settings).await;

            match data {
                Ok(asset) => {
                    load_response_sender
                        .send(Box::new(LoadResponse {
                            handle: handle.clone(),
                            result: LoadAssetResult::Success(asset),
                        }))
                        .await
                        .expect("could not send load success response");
                }
                Err(err) => {
                    tracing::warn!("could not load asset {}", err);
                    load_response_sender
                        .send(Box::new(LoadResponse {
                            handle: handle.clone(),
                            result: LoadAssetResult::Error,
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
        #[cfg(not(target_arch = "wasm32"))]
        self.reload_ctx
            .register_watch(self.handle.clone(), path.as_ref().to_path_buf())
            .await;

        self.filesystem_ctx.load_asset_bytes(&path).await
    }

    pub async fn load_string(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<String, filesystem::LoadFileError> {
        #[cfg(not(target_arch = "wasm32"))]
        self.reload_ctx
            .register_watch(self.handle.clone(), path.as_ref().to_path_buf())
            .await;

        self.filesystem_ctx.load_asset_string(path).await
    }
}
