use futures::{FutureExt, StreamExt};
use rustc_hash::{FxHashMap, FxHashSet};

#[cfg(not(target_arch = "wasm32"))]
use crate::asset::{AssetCacheReload, ReloadFnHandleRequest, WatchHandleRequest};
use crate::{
    asset::{
        derive::AssetCacheDerived, Asset, AssetCacheStorage, AssetHandle, AssetHandleContext,
        AssetLoader, DynAsset, DynAssetHandle,
    },
    filesystem,
};

use std::path::{Path, PathBuf};
use std::{future::Future, pin::Pin};

pub enum LoadAssetResult {
    Loading,
    Success(DynAsset),
    Error,
}

//
// Load
//

#[cfg(target_arch = "wasm32")]
pub type LoadRequest = Pin<Box<dyn Future<Output = ()> + 'static>>;

#[cfg(not(target_arch = "wasm32"))]
pub type LoadRequest = Pin<Box<dyn Future<Output = ()> + Send + 'static>>;

pub struct LoadResponse {
    pub(crate) handle: DynAssetHandle,
    pub(crate) result: LoadAssetResult,
}

pub struct AssetCacheLoad {
    pub(crate) request_sender: async_channel::Sender<LoadRequest>,
    pub(crate) request_receiver: async_channel::Receiver<LoadRequest>,

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
        let (request_sender, request_receiver) = async_channel::unbounded();
        let (response_sender, response_receiver) = async_channel::unbounded();

        let just_loaded = FxHashSet::default();

        Self {
            just_loaded,

            request_sender,
            request_receiver,

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

            // invalidate render cache
            derived.invalidate_render_cache_for_handle(response.handle.clone());
        }
    }

    pub(crate) fn request_load_func<T: AssetLoader + 'static>(
        load_ctx: LoadContext,

        handle: AssetHandle<T::Asset>,
        settings: T::Settings,
    ) {
        let load_ctx_clone = load_ctx.clone();
        let sender = load_ctx.load_response_sender.clone();
        let dyn_handle = handle.as_any().clone();

        // TODO: is this correct?
        // set currently loading
        sender
            .try_send(LoadResponse {
                handle: dyn_handle.clone(),
                result: LoadAssetResult::Loading,
            })
            .expect("could not send load success response");

        // request load
        load_ctx
            .load_request_sender
            .try_send(Box::pin(async move {
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
            }))
            .expect("could not send request to unbounded channel");
    }

    /// Start the background loader
    ///
    /// Native: Spawn a new thread with an executor
    ///
    /// Wasm: Attach background loader to JS scheduler
    pub fn start_background_loader(&self) {
        let request_receiver_copy = self.request_receiver.clone();

        #[cfg(not(target_arch = "wasm32"))]
        std::thread::spawn(move || {
            // TODO: should probably use better executor
            pollster::block_on(Self::background_loader(request_receiver_copy));
        });

        #[cfg(target_arch = "wasm32")]
        wasm_bindgen_futures::spawn_local(Self::background_loader(request_receiver_copy));
    }

    /// Implementation of an asset loader that runs in the background
    ///
    /// Should be started using `start_background_loader`
    async fn background_loader(requests: async_channel::Receiver<LoadRequest>) {
        let mut running = futures::stream::FuturesUnordered::new();

        loop {
            if running.is_empty() {
                // when no assets are loading, only await new requests
                let load_request = requests.recv().await.expect("channel closed");
                running.push(load_request);
                continue;
            } else {
                // when assets are loading, await both assets and new requests
                futures::select! {
                    load_request = requests.recv().fuse() => {
                        let request = load_request.expect("could not");
                        running.push(request);
                    }
                    load_result = running.next().fuse() => {
                        if load_result.is_none() {
                            tracing::info!("finished loading all current load requests");
                        }
                    }
                }
            }
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

    /// channel for loading additional assets
    pub(crate) load_request_sender: async_channel::Sender<LoadRequest>,

    /// channel for sending load result
    pub(crate) load_response_sender: async_channel::Sender<LoadResponse>,

    /// channel for registering handle for reload watching
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) watch_handle_sender: async_channel::Sender<WatchHandleRequest>,

    // channel for registering reload fns
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) reload_handle_sender: async_channel::Sender<ReloadFnHandleRequest>,
}

impl LoadContext {
    pub fn new(
        handle: DynAssetHandle,

        asset_handle_ctx: AssetHandleContext,
        filesystem_ctx: filesystem::FileSystemContext,

        loader: &AssetCacheLoad,

        #[cfg(not(target_arch = "wasm32"))] reloader: &AssetCacheReload,
    ) -> Self {
        Self {
            handle,

            asset_handle_ctx,
            filesystem_ctx,

            load_request_sender: loader.request_sender.clone(),
            load_response_sender: loader.response_sender.clone(),

            #[cfg(not(target_arch = "wasm32"))]
            watch_handle_sender: reloader.watch_sender.clone(),
            #[cfg(not(target_arch = "wasm32"))]
            reload_handle_sender: reloader.reload_fn_sender.clone(),
        }
    }

    pub fn insert<T: Asset>(&self, value: T) -> AssetHandle<T> {
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
    pub fn request_load<T: AssetLoader + 'static>(
        &self,
        settings: T::Settings,
    ) -> AssetHandle<T::Asset> {
        let handle = AssetHandle::new(&self.asset_handle_ctx);

        #[cfg(not(target_arch = "wasm32"))]
        self.register_reload_fns::<T>(handle.clone(), settings.clone());

        AssetCacheLoad::request_load_func::<T>(self.clone(), handle.clone(), settings);

        handle
    }

    /// Request load with existing handle
    pub fn request_load_with_handle<T: AssetLoader + 'static>(
        &self,
        handle: AssetHandle<T::Asset>,
        settings: T::Settings,
    ) {
        #[cfg(not(target_arch = "wasm32"))]
        self.register_reload_fns::<T>(handle.clone(), settings.clone());

        AssetCacheLoad::request_load_func::<T>(self.clone(), handle.clone(), settings);
    }

    pub async fn load_bytes(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<Vec<u8>, filesystem::LoadFileError> {
        #[cfg(not(target_arch = "wasm32"))]
        self.register_watch(path.as_ref().to_path_buf()).await;
        self.filesystem_ctx.load_asset_bytes(&path).await
    }

    pub async fn load_string(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<String, filesystem::LoadFileError> {
        #[cfg(not(target_arch = "wasm32"))]
        self.register_watch(path.as_ref().to_path_buf()).await;
        self.filesystem_ctx.load_asset_string(path).await
    }

    #[cfg(not(target_arch = "wasm32"))]
    async fn register_watch(&self, path: impl Into<PathBuf>) {
        let path = path.into();
        tracing::info!("SEND WATCH HANDLE {}: {}", self.handle.id(), path.display());
        self.watch_handle_sender
            .send(WatchHandleRequest {
                handle: self.handle.clone(),
                path: path.clone(),
            })
            .await
            .expect("could not send");
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn register_reload_fns<T: AssetLoader + 'static>(
        &self,
        handle: AssetHandle<T::Asset>,
        settings: T::Settings,
    ) {
        // async
        let handle_clone = handle.clone();
        let settings_clone = settings.clone();
        let load_ctx_clone = self.clone();
        let load_fn = Box::new(move || {
            let handle_clone = handle_clone.clone();
            let settings_clone = settings_clone.clone();
            let load_ctx_clone = load_ctx_clone.clone();
            AssetCacheLoad::request_load_func::<T>(load_ctx_clone, handle_clone, settings_clone);
        });

        tracing::info!("SEND RELOAD FN {}", handle.id());
        // send over channel
        self.reload_handle_sender
            .try_send(ReloadFnHandleRequest {
                handle: handle.as_any(),
                load_fn,
            })
            .expect("could not send register reload handle request");
    }
}
