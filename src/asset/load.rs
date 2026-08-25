#[cfg(not(target_arch = "wasm32"))]
use crate::asset::AssetCacheReload;
use crate::{
    asset::{
        Asset, AssetCacheDependency, AssetCacheDerivedConvert, AssetCacheRegistry,
        AssetCacheStorage, AssetHandle, AssetHandleContext, DynAssetHandle, LoadStatus,
    },
    filesystem::{self, FileSystemContext},
    task::TaskContext,
};
use rustc_hash::{FxHashMap, FxHashSet};
use std::{
    any::{Any, TypeId},
    future::Future,
    hash::Hash,
    marker::PhantomData,
    path::PathBuf,
};
use std::{error, path::Path};

//
// Types
//

pub trait AssetSettings: Hash + Eq + Clone + Send {}
impl<T: Hash + Eq + Clone + Send> AssetSettings for T {} // TODO: maybe do this for Asset and derived asset

pub trait AssetError: error::Error + Send {}
impl<T: error::Error + Send> AssetError for T {} // TODO: maybe do this for Asset and derived asset

pub trait AssetLoader: Any + Send {
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

pub enum LoadAssetResult<T: AssetLoader> {
    Success(T::Asset),
    Error,
}

//
// Response
//

pub struct LoadResponse<T: AssetLoader> {
    pub(crate) handle: AssetHandle<T::Asset>,
    pub(crate) result: LoadAssetResult<T>,
    pub(crate) dependencies: FxHashSet<DynAssetHandle>,
    pub(crate) watches: FxHashSet<PathBuf>,
}

#[cfg(not(target_arch = "wasm32"))]
pub trait DynLoadResponse: Send {
    fn handle_asset_load_response(
        self: Box<Self>,
        storage: &mut AssetCacheStorage,
        loader: &mut AssetCacheLoad,
        registry: &mut AssetCacheRegistry,
        derived_convert: &mut AssetCacheDerivedConvert,
        dependency: &mut AssetCacheDependency,
        #[cfg(not(target_arch = "wasm32"))] reloader: &mut AssetCacheReload,
    );
}

#[cfg(target_arch = "wasm32")]
pub trait DynLoadResponse {
    fn handle_asset_load_response(
        self: Box<Self>,
        storage: &mut AssetCacheStorage,
        loader: &mut AssetCacheLoad,
        registry: &mut AssetCacheRegistry,
        derived_convert: &mut AssetCacheDerivedConvert,
        dependency: &mut AssetCacheDependency,
        #[cfg(not(target_arch = "wasm32"))] reloader: &mut AssetCacheReload,
    );
}

impl<T: AssetLoader> DynLoadResponse for LoadResponse<T> {
    fn handle_asset_load_response(
        self: Box<Self>,
        storage: &mut AssetCacheStorage,
        loader: &mut AssetCacheLoad,
        registry: &mut AssetCacheRegistry,
        derived_convert: &mut AssetCacheDerivedConvert,
        dependency: &mut AssetCacheDependency,
        #[cfg(not(target_arch = "wasm32"))] reloader: &mut AssetCacheReload,
    ) {
        match self.result {
            LoadAssetResult::Success(asset) => {
                tracing::info!("load success {}", self.handle);
                let dyn_handle = self.handle.to_dyn();

                // Storage
                storage.insert(self.handle.clone(), asset);

                // Registry
                registry.remove_status(&dyn_handle);

                // Loader
                loader.just_loaded.insert(dyn_handle.clone());

                // Dependency
                dependency.register_dependencies(&dyn_handle.clone(), &self.dependencies);

                // Derived
                derived_convert.wakeup_waiting_on_handle(registry, &dyn_handle.clone());
                derived_convert.requeu_dependents(dependency, registry, &dyn_handle);

                // Reloader
                #[cfg(not(target_arch = "wasm32"))]
                {
                    reloader.register_loader_type::<T>(dyn_handle.clone());
                    reloader.register_watches(dyn_handle.clone(), &self.watches);
                    // TODO: maybe this should apply to everything related to reloading?
                    if reloader.is_currently_reloading(&dyn_handle) {
                        reloader.reload_dependents(dependency, loader, registry, &dyn_handle);
                    }
                }
            }
            LoadAssetResult::Error => {
                tracing::info!("load error");
                let dyn_handle = self.handle.to_dyn();

                // Loader
                registry.set_status(dyn_handle.clone(), LoadStatus::Failed);
            }
        }
    }
}

//
// Request
//

#[cfg(not(target_arch = "wasm32"))]
pub trait DynHandleRequest: Send {
    fn get_or_load_new_asset(&self, loader: &mut AssetCacheLoad, registry: &mut AssetCacheRegistry);
}

#[cfg(target_arch = "wasm32")]
pub trait DynHandleRequest {
    fn get_or_load_new_asset(&self, loader: &mut AssetCacheLoad, registry: &mut AssetCacheRegistry);
}

pub struct GetHandleRequest<T: AssetLoader> {
    settings: T::Settings,
    sender: async_channel::Sender<AssetHandle<T::Asset>>,
}

impl<T: AssetLoader> GetHandleRequest<T> {
    pub fn new(
        settings: T::Settings,
        sender: async_channel::Sender<AssetHandle<T::Asset>>,
    ) -> Self {
        Self { settings, sender }
    }
}

impl<T: AssetLoader> DynHandleRequest for GetHandleRequest<T> {
    fn get_or_load_new_asset(
        &self,
        loader: &mut AssetCacheLoad,
        registry: &mut AssetCacheRegistry,
    ) {
        let (handle, created_new) = registry.get_or_create_load_handle::<T>(self.settings.clone());

        if created_new {
            loader.load_asset_with_handle::<T>(registry, handle.clone(), self.settings.clone());
        }

        // send the handle back
        self.sender
            .try_send(handle)
            .expect("could not send get asset handle response");
    }
}

//
// Generic
//

pub struct AssetCacheLoad {
    typed_caches: FxHashMap<TypeId, Box<dyn DynAssetLoad>>,
    asset_handle_ctx: AssetHandleContext,
    task_ctx: TaskContext,
    filesystem_ctx: FileSystemContext,

    // Handle request
    pub(crate) handle_request_sender: async_channel::Sender<Box<dyn DynHandleRequest>>,
    pub(crate) handle_request_receiver: async_channel::Receiver<Box<dyn DynHandleRequest>>,

    // Load response
    pub(crate) response_sender: async_channel::Sender<Box<dyn DynLoadResponse>>,
    pub(crate) response_receiver: async_channel::Receiver<Box<dyn DynLoadResponse>>,

    // TODO: maybe this should be moved to registry to support it for converted values
    pub(crate) just_loaded: FxHashSet<DynAssetHandle>,
}

impl AssetCacheLoad {
    pub(crate) fn new(
        task_ctx: TaskContext,
        filesystem_ctx: FileSystemContext,
        asset_handle_ctx: AssetHandleContext,
    ) -> Self {
        let typed_caches = FxHashMap::default();

        let (response_sender, response_receiver) = async_channel::unbounded();
        let (handle_request_sender, handle_request_receiver) = async_channel::unbounded();

        let just_loaded = FxHashSet::default();

        Self {
            task_ctx,
            filesystem_ctx,
            asset_handle_ctx,
            typed_caches,

            just_loaded,

            response_sender,
            response_receiver,

            handle_request_sender,
            handle_request_receiver,
        }
    }

    pub fn get_typed_cache_ref<T: AssetLoader + 'static>(&self) -> Option<&TypedAssetLoad<T>> {
        self.typed_caches.get(&TypeId::of::<T>()).map(|a| {
            a.as_any()
                .downcast_ref::<TypedAssetLoad<T>>()
                .expect("could not downcast typed storage cache")
        })
    }

    /// Get mutable typed cache or create if it doesnt exist
    pub fn get_typed_cache_mut<T: AssetLoader + 'static>(&mut self) -> &mut TypedAssetLoad<T> {
        let entry = self
            .typed_caches
            .entry(TypeId::of::<T>())
            .or_insert(Box::new(TypedAssetLoad::<T>::new(
                self.task_ctx.clone(),
                self.filesystem_ctx.clone(),
                self.asset_handle_ctx.clone(),
                self.handle_request_sender.clone(),
                self.response_sender.clone(),
            )));
        entry
            .as_any_mut()
            .downcast_mut::<TypedAssetLoad<T>>()
            .expect("could not downcast typed storage cache")
    }

    /// Request load with new handle
    pub fn load_asset<T: AssetLoader>(
        &mut self,
        registry: &mut AssetCacheRegistry,
        settings: T::Settings,
    ) -> AssetHandle<T::Asset> {
        let handle = AssetHandle::<T::Asset>::new(&self.asset_handle_ctx);

        self.load_asset_with_handle::<T>(registry, handle.clone(), settings);

        handle
    }

    pub fn load_asset_with_handle<T: AssetLoader>(
        &mut self,
        registry: &mut AssetCacheRegistry,
        handle: AssetHandle<T::Asset>,
        settings: T::Settings,
    ) -> AssetHandle<T::Asset> {
        registry.set_status(handle.to_dyn(), LoadStatus::Loading);

        self.get_typed_cache_mut::<T>()
            .load_asset_with_handle(handle.clone(), settings);

        handle
    }

    pub fn handle_just_loaded<T: Asset>(&self, handle: AssetHandle<T>) -> bool {
        self.just_loaded.contains(&handle.to_dyn())
    }

    // check if any files completed loading and update cache and invalidate render cache
    pub fn poll_loaded(
        &mut self,
        storage: &mut AssetCacheStorage,
        registry: &mut AssetCacheRegistry,
        derived_convert: &mut AssetCacheDerivedConvert,
        dependency: &mut AssetCacheDependency,
        #[cfg(not(target_arch = "wasm32"))] reloader: &mut AssetCacheReload,
    ) {
        self.just_loaded.clear();

        while let Ok(response) = self.response_receiver.try_recv() {
            response.handle_asset_load_response(
                storage,
                self,
                registry,
                derived_convert,
                dependency,
                #[cfg(not(target_arch = "wasm32"))]
                reloader,
            );
        }
    }

    // check for request of new handles
    pub fn poll_handle_requests(&mut self, registry: &mut AssetCacheRegistry) {
        while let Ok(request) = self.handle_request_receiver.try_recv() {
            request.get_or_load_new_asset(self, registry);
        }
    }
}

//
// Typed
//

pub struct TypedAssetLoad<T: AssetLoader> {
    asset_handle_ctx: AssetHandleContext,
    task_ctx: TaskContext,
    filesystem_ctx: FileSystemContext,

    // Handle request
    handle_request_sender: async_channel::Sender<Box<dyn DynHandleRequest>>,

    // Load response
    response_sender: async_channel::Sender<Box<dyn DynLoadResponse>>,

    // TODO: shouldnt be needed?
    ty: PhantomData<T>,
}

impl<T: AssetLoader> TypedAssetLoad<T> {
    pub fn new(
        task_ctx: TaskContext,
        filesystem_ctx: FileSystemContext,
        asset_handle_ctx: AssetHandleContext,

        handle_request_sender: async_channel::Sender<Box<dyn DynHandleRequest>>,

        response_sender: async_channel::Sender<Box<dyn DynLoadResponse>>,
    ) -> Self {
        Self {
            asset_handle_ctx,
            task_ctx,
            filesystem_ctx,
            handle_request_sender,

            response_sender,
            ty: PhantomData,
        }
    }

    fn load_asset_with_handle(&mut self, handle: AssetHandle<T::Asset>, settings: T::Settings) {
        tracing::info!("spawn load {}", handle);

        let new_asset_state = LoadState::new(handle.to_dyn());
        let new_asset_runtime = LoadRuntime {
            asset_handle_ctx: self.asset_handle_ctx.clone(),
            filesystem_ctx: self.filesystem_ctx.clone(),
            handle_request_sender: self.handle_request_sender.clone(),
            load_response_sender: self.response_sender.clone(),
        };
        let mut new_load_ctx = LoadContext::new(new_asset_state, new_asset_runtime);

        // spawn load
        self.task_ctx.spawn_task(Box::pin(async move {
            let data = T::load(&mut new_load_ctx, settings).await;

            match data {
                Ok(asset) => {
                    new_load_ctx
                        .runtime
                        .load_response_sender
                        .send(Box::new(LoadResponse {
                            handle: handle.clone(),
                            result: LoadAssetResult::<T>::Success(asset),
                            dependencies: new_load_ctx.state.dependencies,
                            watches: new_load_ctx.state.watches,
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
                            result: LoadAssetResult::<T>::Error,
                            dependencies: new_load_ctx.state.dependencies,
                            watches: new_load_ctx.state.watches,
                        }))
                        .await
                        .expect("could not send load error response");
                }
            }
        }));
    }
}
//
// Dyn
//

pub trait DynAssetLoad {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T: AssetLoader + 'static> DynAssetLoad for TypedAssetLoad<T> {
    fn as_any(&self) -> &dyn Any {
        self as &dyn Any
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self as &mut dyn Any
    }
}

//
// Load context
//

#[derive(Clone)]
pub struct LoadState {
    pub(crate) handle: DynAssetHandle,
    // TODO: not being used rn
    pub(crate) dependencies: FxHashSet<DynAssetHandle>,
    pub(crate) watches: FxHashSet<PathBuf>,
}

impl LoadState {
    pub fn new(handle: DynAssetHandle) -> Self {
        Self {
            handle,
            dependencies: FxHashSet::default(),
            watches: FxHashSet::default(),
        }
    }
}

#[derive(Clone)]
pub struct LoadRuntime {
    pub(crate) asset_handle_ctx: AssetHandleContext,
    pub(crate) filesystem_ctx: filesystem::FileSystemContext,

    pub(crate) handle_request_sender: async_channel::Sender<Box<dyn DynHandleRequest>>,
    pub(crate) load_response_sender: async_channel::Sender<Box<dyn DynLoadResponse>>,
}

impl LoadRuntime {
    pub fn new(
        asset_handle_ctx: AssetHandleContext,
        filesystem_ctx: filesystem::FileSystemContext,
        loader: &AssetCacheLoad,
    ) -> Self {
        let load_response_sender = loader.response_sender.clone();
        let handle_request_sender = loader.handle_request_sender.clone();
        Self {
            asset_handle_ctx,
            filesystem_ctx,
            load_response_sender,
            handle_request_sender,
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

    pub fn handle(&self) -> DynAssetHandle {
        self.state.handle.clone()
    }

    // TODO: should probably just get from registy and then send succes response
    pub fn insert_asset<T: Asset>(&self, value: T) -> AssetHandle<T> {
        // TODO: should probably request handle with async
        let handle = AssetHandle::<T>::new(&self.runtime.asset_handle_ctx);
        // TODO:
        // self.runtime
        //     .load_response_sender
        //     .try_send(Box::new(LoadResponse {
        //         handle: handle.clone(),
        //         result: LoadAssetResult::<T>::Success(value),
        //         dependencies: FxHashSet::default(),
        //         watches: self.state.watches.clone(),
        //     }))
        //     .expect("could not send asset handle");
        handle
    }

    pub async fn request_load<T: AssetLoader + 'static>(
        &mut self,
        settings: T::Settings,
    ) -> AssetHandle<T::Asset> {
        let (sender, receiver) = async_channel::bounded(1);

        self.runtime
            .handle_request_sender
            .send(Box::new(GetHandleRequest::<T>::new(settings, sender)))
            .await
            .expect("could not send handle request");

        let handle = receiver
            .recv()
            .await
            .expect("could not receive handle request");

        self.state.dependencies.insert(handle.to_dyn());

        handle
    }

    pub async fn load_bytes(
        &mut self,
        path: impl AsRef<Path>,
    ) -> Result<Vec<u8>, filesystem::LoadFileError> {
        let result = self.runtime.filesystem_ctx.load_asset_bytes(&path).await;

        if result.is_ok() {
            #[cfg(not(target_arch = "wasm32"))]
            self.state.watches.insert(path.as_ref().to_path_buf());
        }

        result
    }

    pub async fn load_string(
        &mut self,
        path: impl AsRef<Path>,
    ) -> Result<String, filesystem::LoadFileError> {
        let result = self.runtime.filesystem_ctx.load_asset_string(&path).await;

        if result.is_ok() {
            #[cfg(not(target_arch = "wasm32"))]
            self.state.watches.insert(path.as_ref().to_path_buf());
        }

        result
    }
}
