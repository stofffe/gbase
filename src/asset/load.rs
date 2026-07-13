use futures::{FutureExt, StreamExt};

use crate::{
    asset::{Asset, AssetHandle, AssetLoader, DynAssetHandle, LoadAssetResult},
    filesystem,
};

use std::{future::Future, pin::Pin};
use std::{
    path::Path,
    sync::{Arc, Mutex},
};

//
// Load
//

#[cfg(target_arch = "wasm32")]
type LoadRequest = Pin<Box<dyn Future<Output = ()> + 'static>>;

#[cfg(not(target_arch = "wasm32"))]
type LoadRequest = Pin<Box<dyn Future<Output = ()> + Send + 'static>>;

pub struct LoadResponse {
    pub(crate) handle: DynAssetHandle,
    pub(crate) result: LoadAssetResult,
}

#[derive(Clone)]
pub struct AssetCacheLoad {
    pub(crate) load_ctx: LoadContext,

    pub(crate) request_sender: async_channel::Sender<LoadRequest>,
    pub(crate) request_receiver: async_channel::Receiver<LoadRequest>,

    pub(crate) response_sender: async_channel::Sender<LoadResponse>,
    pub(crate) response_receiver: async_channel::Receiver<LoadResponse>,
}

impl AssetCacheLoad {
    pub(crate) fn new(filesystem_ctx: filesystem::FileSystemContext) -> Self {
        let (request_sender, request_receiver) = async_channel::unbounded();
        let (response_sender, response_receiver) = async_channel::unbounded();

        let asset_handle_ctx = AssetHandleContext::new();
        let load_ctx = LoadContext::new(
            asset_handle_ctx,
            filesystem_ctx,
            request_sender.clone(),
            response_sender.clone(),
        );

        Self {
            load_ctx,

            request_sender,
            request_receiver,

            response_sender,
            response_receiver,
        }
    }

    pub(crate) fn request_load<T: AssetLoader + 'static>(
        load_ctx: LoadContext,

        handle: AssetHandle<T::Asset>,
        path: &Path,
        settings: T::Settings,
    ) -> AssetHandle<T::Asset> {
        let load_ctx_clone = load_ctx.clone();
        let path = path.to_path_buf();
        let sender = load_ctx.response_sender.clone();
        let dyn_handle = handle.as_any().clone();

        load_ctx
            .request_sender
            .try_send(Box::pin(async move {
                let data = T::load(load_ctx_clone.clone(), &path, settings).await;

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

        handle
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

    /// Implementation of background loader
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

#[derive(Debug, Clone)]
pub struct LoadContext {
    pub(crate) asset_handle_ctx: AssetHandleContext,
    pub(crate) filesystem_ctx: filesystem::FileSystemContext,

    pub(crate) request_sender: async_channel::Sender<LoadRequest>,
    pub(crate) response_sender: async_channel::Sender<LoadResponse>,
}

impl LoadContext {
    pub fn new(
        asset_handle_ctx: AssetHandleContext,
        filesystem_ctx: filesystem::FileSystemContext,

        request_sender: async_channel::Sender<LoadRequest>,
        response_sender: async_channel::Sender<LoadResponse>,
    ) -> Self {
        Self {
            asset_handle_ctx,
            filesystem_ctx,

            request_sender,
            response_sender,
        }
    }

    pub fn insert<T: Asset>(&self, value: T) -> AssetHandle<T> {
        let handle = AssetHandle::<T>::new(&self.asset_handle_ctx);
        self.response_sender
            .try_send(LoadResponse {
                handle: handle.as_any(),
                result: LoadAssetResult::Success(Box::new(value)),
            })
            .expect("could not send asset handle");
        handle
    }

    pub fn request_load<T: AssetLoader + 'static>(
        &self,
        handle: AssetHandle<T::Asset>,
        path: &Path,
        settings: T::Settings,
    ) -> AssetHandle<T::Asset> {
        AssetCacheLoad::request_load::<T>(self.clone(), handle, path, settings)
    }

    //
    // Re-export filesytem loading functions
    //

    pub async fn load_bytes(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<Vec<u8>, filesystem::LoadFileError> {
        self.filesystem_ctx.load_asset_bytes(path).await
    }
    pub async fn load_string(
        &self,
        path: impl AsRef<Path>,
    ) -> Result<String, filesystem::LoadFileError> {
        self.filesystem_ctx.load_asset_string(path).await
    }
}

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
