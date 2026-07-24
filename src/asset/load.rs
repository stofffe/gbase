#[cfg(not(target_arch = "wasm32"))]
use crate::asset::{AssetCacheReload, ReloadContext};
use crate::{
    asset::{
        derive::AssetCacheDerived, Asset, AssetHandle, AssetHandleContext, AssetLoader, DynAsset,
        DynAssetHandle,
    },
    filesystem, task,
};
use rustc_hash::{FxHashMap, FxHashSet};
use std::path::Path;

pub enum LoadAssetResult {
    Loading,
    Success(DynAsset),
    Error,
}

//
// Load
//

pub struct LoadResponse {
    pub(crate) handle: DynAssetHandle,
    pub(crate) result: LoadAssetResult,
}

pub struct AssetCacheLoad {
    pub(crate) response_sender: async_channel::Sender<LoadResponse>,
    pub(crate) response_receiver: async_channel::Receiver<LoadResponse>,

    // TODO: maybe these should be derived from cache every frame? O(n)
    // TODO: should failed loads be put here?
    pub(crate) just_loaded: FxHashSet<DynAssetHandle>,
}

// pbr needs math.h
//

impl AssetCacheLoad {
    pub(crate) fn new() -> Self {
        let (response_sender, response_receiver) = async_channel::unbounded();

        let just_loaded = FxHashSet::default();

        Self {
            just_loaded,

            response_sender,
            response_receiver,
        }
    }

    pub fn handle_just_loaded<T: Asset>(&self, handle: AssetHandle<T>) -> bool {
        self.just_loaded.contains(&handle.as_any())
    }

    // check if any files completed loading and update cache and invalidate render cache
    pub fn poll_loaded(
        &mut self,
        cache: &mut FxHashMap<DynAssetHandle, LoadAssetResult>,
        derived: &mut AssetCacheDerived,
    ) {
        self.just_loaded.clear();

        while let Ok(response) = self.response_receiver.try_recv() {
            if let LoadAssetResult::Success(_) = response.result {
                self.just_loaded.insert(response.handle.clone());
            }

            cache.insert(response.handle.clone(), response.result);

            derived.invalidate_derived_assets_depending_on_handle(response.handle.clone());
        }
    }
}

//
// Load context
//

#[derive(Clone)]
pub struct LoadContext {
    // TODO: should this be here? related to the load_bytes functions
    pub(crate) handle: DynAssetHandle,

    pub(crate) asset_handle_ctx: AssetHandleContext,
    pub(crate) filesystem_ctx: filesystem::FileSystemContext,
    pub(crate) task_ctx: task::TaskContext,

    /// channel for sending load result
    pub(crate) load_response_sender: async_channel::Sender<LoadResponse>,

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

    pub fn insert_asset<T: Asset>(&self, value: T) -> AssetHandle<T> {
        let handle = AssetHandle::<T>::new(&self.asset_handle_ctx);
        self.load_response_sender
            .try_send(LoadResponse {
                handle: handle.as_any(),
                result: LoadAssetResult::Success(Box::new(value)),
            })
            .expect("could not send asset handle");
        handle
    }

    /// Request load with new handle
    // TODO: does not set status to loading (maybe fine?)
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
        let load_ctx_clone = self.clone();
        let sender = self.load_response_sender.clone();
        let dyn_handle = handle.as_any();
        let settings = settings.clone();

        self.task_ctx.spawn_task(Box::pin(async move {
            let data = T::load(load_ctx_clone.clone(), settings).await;

            match data {
                Ok(asset) => {
                    let boxed_asset = Box::new(asset);
                    sender
                        .send(LoadResponse {
                            handle: dyn_handle,
                            result: LoadAssetResult::Success(boxed_asset),
                        })
                        .await
                        .expect("could not send load success response");
                }
                Err(err) => {
                    tracing::warn!("could not load asset {}", err);
                    sender
                        .send(LoadResponse {
                            handle: dyn_handle,
                            result: LoadAssetResult::Error,
                        })
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
