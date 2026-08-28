#[cfg(not(target_arch = "wasm32"))]
use crate::asset::AssetCacheReload;
use crate::{
    asset::{
        Asset, AssetCacheConvert, AssetCacheDependency, AssetCacheInsert, AssetCacheRegistry,
        AssetCacheStorage, AssetHandle, AssetHandleContext, AssetInserter, DynAssetHandle,
        InsertAssetKey, LoadStatus,
    },
    filesystem::{self, FileSystemContext},
    task::TaskContext,
};
use rustc_hash::{FxHashMap, FxHashSet};
#[cfg(not(target_arch = "wasm32"))]
use std::fmt::Debug;
use std::{
    any::{Any, TypeId},
    collections::VecDeque,
    future::Future,
    hash::Hash,
    marker::PhantomData,
    path::PathBuf,
};
use std::{error, path::Path};

//
// Types
//

pub trait LoadAssetSettings: Hash + Eq + Clone {}
impl<T: Hash + Eq + Clone> LoadAssetSettings for T {}

pub trait AssetError: error::Error {}
impl<T: error::Error> AssetError for T {}

#[cfg(not(target_arch = "wasm32"))]
pub trait AssetLoader: Send {
    type Asset: Asset;
    type Settings: LoadAssetSettings + Send + Debug;
    type Error: AssetError + Send;

    fn load(
        load_ctx: &mut LoadContext,
        settings: Self::Settings,
    ) -> impl Future<Output = Result<Self::Asset, Self::Error>> + Send;
}

#[cfg(target_arch = "wasm32")]
pub trait AssetLoader {
    type Asset: Asset;
    type Settings: LoadAssetSettings;
    type Error: AssetError;

    fn load(
        load_ctx: &mut LoadContext,
        settings: Self::Settings,
    ) -> impl Future<Output = Result<Self::Asset, Self::Error>>;
}

//
// Response
//

enum LoadAssetResult<T: AssetLoader> {
    Success(T::Asset),
    Error,
}

struct LoadResponse<T: AssetLoader> {
    handle: AssetHandle<T::Asset>,
    result: LoadAssetResult<T>,
    dependencies: FxHashSet<DynAssetHandle>,
    watches: FxHashSet<PathBuf>,
}

#[cfg(not(target_arch = "wasm32"))]
trait DynLoadResponse: Send {
    fn handle_asset_load_response(
        self: Box<Self>,
        storage: &mut AssetCacheStorage,
        loader: &mut AssetCacheLoad,
        registry: &mut AssetCacheRegistry,
        convert: &mut AssetCacheConvert,
        dependency: &mut AssetCacheDependency,
        #[cfg(not(target_arch = "wasm32"))] reloader: &mut AssetCacheReload,
    );
}

#[cfg(target_arch = "wasm32")]
trait DynLoadResponse {
    fn handle_asset_load_response(
        self: Box<Self>,
        storage: &mut AssetCacheStorage,
        loader: &mut AssetCacheLoad,
        registry: &mut AssetCacheRegistry,
        convert: &mut AssetCacheConvert,
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
        convert: &mut AssetCacheConvert,
        dependency: &mut AssetCacheDependency,
        #[cfg(not(target_arch = "wasm32"))] reloader: &mut AssetCacheReload,
    ) {
        match self.result {
            LoadAssetResult::Success(asset) => {
                tracing::info!("load success {}", self.handle);
                let dyn_handle = self.handle.to_dyn();

                // Storage
                storage.insert_asset(self.handle.clone(), asset);

                // Registry
                registry.set_status(dyn_handle.clone(), LoadStatus::Ready);
                registry.set_just_available(dyn_handle.clone());

                // Dependency
                dependency.register_dependencies(&dyn_handle.clone(), &self.dependencies);

                // Derived
                convert.wakeup_waiting_on_handle(registry, &dyn_handle.clone());
                convert.reload_depending_conversions(dependency, registry, &dyn_handle);

                // Reloader
                #[cfg(not(target_arch = "wasm32"))]
                {
                    reloader.register_watches(dyn_handle.clone(), &self.watches);

                    if reloader.is_currently_reloading(&dyn_handle) {
                        reloader.reload_dependents(
                            dependency,
                            loader,
                            convert,
                            registry,
                            &dyn_handle,
                        );
                    }
                }
            }
            LoadAssetResult::Error => {
                tracing::error!("load error");
                let dyn_handle = self.handle.to_dyn();

                // Registry
                registry.set_status(dyn_handle.clone(), LoadStatus::Failed);

                // TODO: do we want this?
                // Dependency
                dependency.register_dependencies(&dyn_handle.clone(), &self.dependencies);
            }
        }
    }
}

//
// Load Request
//

/// Type erased load request
///
/// Is sent using async channels
#[cfg(not(target_arch = "wasm32"))]
trait DynLoadRequest: Send {
    fn get_or_load_asset(
        self: Box<Self>,
        loader: &mut AssetCacheLoad,
        registry: &mut AssetCacheRegistry,
    );
}

#[cfg(target_arch = "wasm32")]
trait DynLoadRequest {
    fn get_or_load_asset(
        self: Box<Self>,
        loader: &mut AssetCacheLoad,
        registry: &mut AssetCacheRegistry,
    );
}

struct TypedLoadRequest<T: AssetLoader> {
    settings: T::Settings,
    response_sender: async_channel::Sender<AssetHandle<T::Asset>>,
}

impl<T: AssetLoader> TypedLoadRequest<T> {
    fn new(
        settings: T::Settings,
        response_sender: async_channel::Sender<AssetHandle<T::Asset>>,
    ) -> Self {
        Self {
            settings,
            response_sender,
        }
    }
}

impl<T: AssetLoader + 'static> DynLoadRequest for TypedLoadRequest<T> {
    fn get_or_load_asset(
        self: Box<Self>,
        loader: &mut AssetCacheLoad,
        registry: &mut AssetCacheRegistry,
    ) {
        let handle = loader.register_load::<T>(registry, &self.settings);

        self.response_sender
            .try_send(handle)
            .expect("could not send load asset handle response");
    }
}

//
// Insert request
//

/// Type erased insert request
///
/// Is sent using async channels
#[cfg(not(target_arch = "wasm32"))]
pub(crate) trait DynInsertRequest: Send {
    fn insert_asset(
        self: Box<Self>,
        registry: &mut AssetCacheRegistry,
        storage: &mut AssetCacheStorage,
        inserter: &mut AssetCacheInsert,
    );
}

#[cfg(target_arch = "wasm32")]
pub(crate) trait DynInsertRequest {
    fn insert_asset(
        self: Box<Self>,
        registry: &mut AssetCacheRegistry,
        storage: &mut AssetCacheStorage,
        inserter: &mut AssetCacheInsert,
    );
}

struct TypedInsertRequest<T: Asset, I: AssetInserter> {
    key: I::Key,
    asset: T,
    response_sender: async_channel::Sender<AssetHandle<T>>,
}

impl<T: Asset, I: AssetInserter> TypedInsertRequest<T, I> {
    fn new(key: I::Key, asset: T, response_sender: async_channel::Sender<AssetHandle<T>>) -> Self {
        Self {
            key,
            asset,
            response_sender,
        }
    }
}

impl<T: Asset, I: AssetInserter + 'static> DynInsertRequest for TypedInsertRequest<T, I> {
    fn insert_asset(
        self: Box<Self>,
        registry: &mut AssetCacheRegistry,
        storage: &mut AssetCacheStorage,
        inserter: &mut AssetCacheInsert,
    ) {
        tracing::info!("insert nested asset {:?}", self.key);
        let handle = inserter.insert_asset::<T, I>(registry, storage, &self.key, self.asset);
        self.response_sender
            .try_send(handle)
            .expect("could not send insert asset handle response");
    }
}

//
// Generic
//

pub(crate) struct AssetCacheLoad {
    typed_load: FxHashMap<TypeId, Box<dyn DynAssetLoad>>,
    asset_handle_ctx: AssetHandleContext,
    task_ctx: TaskContext,
    filesystem_ctx: FileSystemContext,

    queue: VecDeque<DynAssetHandle>,
    queued: FxHashSet<DynAssetHandle>,
    handle_to_loader_type: FxHashMap<DynAssetHandle, TypeId>,

    // load request
    load_request_sender: async_channel::Sender<Box<dyn DynLoadRequest>>,
    load_request_receiver: async_channel::Receiver<Box<dyn DynLoadRequest>>,

    // insert request
    insert_request_sender: async_channel::Sender<Box<dyn DynInsertRequest>>,
    insert_request_receiver: async_channel::Receiver<Box<dyn DynInsertRequest>>,

    // load response
    response_sender: async_channel::Sender<Box<dyn DynLoadResponse>>,
    response_receiver: async_channel::Receiver<Box<dyn DynLoadResponse>>,
}

impl AssetCacheLoad {
    pub(crate) fn new(
        task_ctx: TaskContext,
        filesystem_ctx: FileSystemContext,
        asset_handle_ctx: AssetHandleContext,
    ) -> Self {
        let typed_load = FxHashMap::default();

        let (response_sender, response_receiver) = async_channel::unbounded();
        let (load_request_sender, load_request_receiver) = async_channel::unbounded();
        let (insert_request_sender, insert_request_receiver) = async_channel::unbounded();

        Self {
            task_ctx,
            filesystem_ctx,
            asset_handle_ctx,
            typed_load,

            queue: VecDeque::default(),
            queued: FxHashSet::default(),
            handle_to_loader_type: FxHashMap::default(),

            response_sender,
            response_receiver,

            load_request_sender,
            load_request_receiver,

            insert_request_sender,
            insert_request_receiver,
        }
    }

    /// Get mutable typed cache or create if it doesnt exist
    fn get_typed_cache_mut<T: AssetLoader + 'static>(&mut self) -> &mut TypedAssetLoad<T> {
        let entry =
            self.typed_load
                .entry(TypeId::of::<T>())
                .or_insert(Box::new(TypedAssetLoad::<T>::new(
                    self.task_ctx.clone(),
                    self.filesystem_ctx.clone(),
                    self.asset_handle_ctx.clone(),
                    self.load_request_sender.clone(),
                    self.insert_request_sender.clone(),
                    self.response_sender.clone(),
                )));
        entry
            .as_any_mut()
            .downcast_mut::<TypedAssetLoad<T>>()
            .expect("could not downcast typed storage cache")
    }

    // check if any files completed loading and update cache and invalidate render cache
    pub(crate) fn poll_loaded(
        &mut self,
        storage: &mut AssetCacheStorage,
        registry: &mut AssetCacheRegistry,
        convert: &mut AssetCacheConvert,
        dependency: &mut AssetCacheDependency,
        #[cfg(not(target_arch = "wasm32"))] reloader: &mut AssetCacheReload,
    ) {
        while let Ok(response) = self.response_receiver.try_recv() {
            response.handle_asset_load_response(
                storage,
                self,
                registry,
                convert,
                dependency,
                #[cfg(not(target_arch = "wasm32"))]
                reloader,
            );
        }
    }

    // check for request of nested loads
    pub(crate) fn poll_load_requests(&mut self, registry: &mut AssetCacheRegistry) {
        while let Ok(request) = self.load_request_receiver.try_recv() {
            request.get_or_load_asset(self, registry);
        }
    }

    // check for request of nested inserts
    pub(crate) fn poll_insert_requests(
        &mut self,
        registry: &mut AssetCacheRegistry,
        storage: &mut AssetCacheStorage,
        inserter: &mut AssetCacheInsert,
    ) {
        while let Ok(request) = self.insert_request_receiver.try_recv() {
            request.insert_asset(registry, storage, inserter);
        }
    }

    pub(crate) fn poll_queue_loads(&mut self, registry: &mut AssetCacheRegistry) {
        while let Some(dyn_handle) = self.queue.pop_front() {
            self.queued.remove(&dyn_handle);

            let Some(type_id) = self.handle_to_loader_type.get(&dyn_handle) else {
                tracing::warn!("no loader registered for {}", dyn_handle);
                continue;
            };

            let Some(typed_load) = self.typed_load.get_mut(type_id) else {
                panic!("could not get typed converter");
            };

            typed_load.load(registry, dyn_handle);
        }
    }

    //
    // Load
    //

    pub(crate) fn register_load<T: AssetLoader + 'static>(
        &mut self,
        registry: &mut AssetCacheRegistry,
        settings: &T::Settings,
    ) -> AssetHandle<T::Asset> {
        let handle = registry.get_or_create_load_handle::<T>(settings);

        if let LoadStatus::NotRegistered = registry.get_status(&handle.to_dyn()) {
            tracing::info!("register load {}", handle);

            self.handle_to_loader_type
                .insert(handle.to_dyn(), TypeId::of::<T>());

            self.get_typed_cache_mut::<T>();

            self.queue_load(registry, handle.to_dyn());
        } else {
            tracing::info!(
                "already has status {:?}",
                registry.get_status(&handle.to_dyn())
            )
        }

        handle
    }

    pub(crate) fn queue_load(&mut self, registry: &mut AssetCacheRegistry, handle: DynAssetHandle) {
        tracing::info!("queue load for {}", handle);
        if self.queued.insert(handle.clone()) {
            registry.set_status(handle.clone(), LoadStatus::Loading);
            self.queue.push_back(handle);
        }
    }
}

//
// Typed
//

struct TypedAssetLoad<T: AssetLoader> {
    asset_handle_ctx: AssetHandleContext,
    task_ctx: TaskContext,
    filesystem_ctx: FileSystemContext,

    load_request_sender: async_channel::Sender<Box<dyn DynLoadRequest>>,
    insert_request_sender: async_channel::Sender<Box<dyn DynInsertRequest>>,

    // Load response
    response_sender: async_channel::Sender<Box<dyn DynLoadResponse>>,

    // TODO: shouldnt be needed?
    ty: PhantomData<T>,
}

impl<T: AssetLoader + 'static> TypedAssetLoad<T> {
    fn new(
        task_ctx: TaskContext,
        filesystem_ctx: FileSystemContext,
        asset_handle_ctx: AssetHandleContext,

        load_request_sender: async_channel::Sender<Box<dyn DynLoadRequest>>,
        insert_request_sender: async_channel::Sender<Box<dyn DynInsertRequest>>,

        response_sender: async_channel::Sender<Box<dyn DynLoadResponse>>,
    ) -> Self {
        Self {
            asset_handle_ctx,
            task_ctx,
            filesystem_ctx,

            load_request_sender,
            insert_request_sender,

            response_sender,
            ty: PhantomData,
        }
    }

    fn load_asset_with_handle(&mut self, handle: AssetHandle<T::Asset>, settings: T::Settings) {
        tracing::info!("spawn load {}", handle);

        let new_asset_state = LoadState::new(handle.to_dyn());
        let new_asset_runtime = LoadRuntime::new(
            self.filesystem_ctx.clone(),
            self.load_request_sender.clone(),
            self.insert_request_sender.clone(),
            self.response_sender.clone(),
        );

        let mut new_load_ctx = LoadContext::new(new_asset_state, new_asset_runtime);

        // spawn load
        self.task_ctx.spawn_task(Box::pin(async move {
            let data = T::load(&mut new_load_ctx, settings).await;

            match data {
                Ok(asset) => {
                    new_load_ctx
                        .runtime
                        .response_sender
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
                        .response_sender
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

trait DynAssetLoad {
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn load(&mut self, registry: &mut AssetCacheRegistry, dyn_handle: DynAssetHandle);
}

impl<T: AssetLoader + 'static> DynAssetLoad for TypedAssetLoad<T> {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self as &mut dyn Any
    }

    fn load(&mut self, registry: &mut AssetCacheRegistry, dyn_handle: DynAssetHandle) {
        let handle = dyn_handle
            .to_typed::<T::Asset>()
            .expect("could not convert dyn handle to typed");

        let Some(settings) = registry.get_load_settings_from_handle::<T>(&dyn_handle) else {
            panic!("could not get settings from handle");
        };

        // TODO: just move everything in this func here
        self.load_asset_with_handle(handle, settings);
    }
}

//
// Load context
//

#[derive(Clone)]
struct LoadState {
    pub(crate) handle: DynAssetHandle,
    // TODO: not being used rn
    pub(crate) dependencies: FxHashSet<DynAssetHandle>,
    pub(crate) watches: FxHashSet<PathBuf>,
}

impl LoadState {
    fn new(handle: DynAssetHandle) -> Self {
        Self {
            handle,
            dependencies: FxHashSet::default(),
            watches: FxHashSet::default(),
        }
    }
}

#[derive(Clone)]
struct LoadRuntime {
    filesystem_ctx: filesystem::FileSystemContext,

    // async channel for requesting nested loads
    load_request_sender: async_channel::Sender<Box<dyn DynLoadRequest>>,

    // async channel for requesting insertions
    insert_request_sender: async_channel::Sender<Box<dyn DynInsertRequest>>,

    // async channel for returning the result of the load
    // note: not for nested loads
    response_sender: async_channel::Sender<Box<dyn DynLoadResponse>>,
}

impl LoadRuntime {
    fn new(
        filesystem_ctx: filesystem::FileSystemContext,
        load_request_sender: async_channel::Sender<Box<dyn DynLoadRequest>>,
        insert_request_sender: async_channel::Sender<Box<dyn DynInsertRequest>>,
        response_sender: async_channel::Sender<Box<dyn DynLoadResponse>>,
    ) -> Self {
        Self {
            filesystem_ctx,
            response_sender,
            load_request_sender,
            insert_request_sender,
        }
    }
}

#[derive(Clone)]
pub struct LoadContext {
    state: LoadState,
    runtime: LoadRuntime,
}

impl LoadContext {
    fn new(state: LoadState, runtime: LoadRuntime) -> Self {
        Self { runtime, state }
    }

    fn handle(&self) -> DynAssetHandle {
        self.state.handle.clone()
    }

    // TODO: should probably just get from registy and then send succes response
    pub async fn insert_asset<T: Asset, I: AssetInserter + 'static>(
        &mut self,
        key: I::Key,
        asset: T,
    ) -> AssetHandle<T> {
        tracing::info!("ASYNC: request nested load request for {}", self.handle());

        let (sender, receiver) = async_channel::bounded(1);

        self.runtime
            .insert_request_sender
            .send(Box::new(TypedInsertRequest::<T, I>::new(
                key, asset, sender,
            )))
            .await
            .expect("could not send insert request");

        let handle = receiver
            .recv()
            .await
            .expect("could not receive insert request");
        tracing::info!(
            "ASYNC: receive nested insert request for {} got {}",
            self.handle(),
            handle
        );

        self.state.dependencies.insert(handle.to_dyn());

        handle
    }

    pub async fn request_load<T: AssetLoader + 'static>(
        &mut self,
        settings: T::Settings,
    ) -> AssetHandle<T::Asset> {
        tracing::info!("ASYNC: request nested load request for {}", self.handle());

        let (sender, receiver) = async_channel::bounded(1);

        self.runtime
            .load_request_sender
            .send(Box::new(TypedLoadRequest::<T>::new(settings, sender)))
            .await
            .expect("could not send load request");

        let handle = receiver
            .recv()
            .await
            .expect("could not receive load request");
        tracing::info!(
            "ASYNC: receive nested load request for {} got {}",
            self.handle(),
            handle
        );

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
