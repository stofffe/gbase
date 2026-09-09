#[cfg(not(target_arch = "wasm32"))]
use crate::asset::AssetCacheReload;

use crate::{
    arc::{self, ArcHandleRuntime},
    asset::{
        dependency, Asset, AssetCacheDependency, AssetCacheInsert, AssetCacheRegistry,
        AssetCacheStorage, AssetHandle, AssetInserter, DynAssetHandle, InternalAssetState,
    },
    filesystem::{self, FileSystemRuntime},
    render::{self, RenderRuntime},
    task::TaskExecutorRuntime,
    ConditionalSend,
};
use rustc_hash::{FxHashMap, FxHashSet};
use std::{
    any::{Any, TypeId},
    collections::VecDeque,
    future::Future,
    hash::Hash,
    marker::PhantomData,
    path::PathBuf,
};
use std::{error, path::Path};
use std::{fmt::Debug, sync::Arc};

//
// Types
//

pub trait LoadAssetSettings: Debug + Hash + Eq + Clone {}
impl<T: Debug + Hash + Eq + Clone> LoadAssetSettings for T {}

pub trait AssetError: error::Error {}
impl<T: error::Error> AssetError for T {}

pub trait LoadAssetExtraData: Clone {}
impl<T: Clone> LoadAssetExtraData for T {}

pub trait AssetLoader: ConditionalSend {
    type Asset: Asset;
    type Settings: LoadAssetSettings + ConditionalSend;
    type Error: AssetError + ConditionalSend;

    fn load(
        load_ctx: &mut LoadContext,
        settings: Self::Settings,
    ) -> impl Future<Output = Result<Self::Asset, Self::Error>> + ConditionalSend;
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

trait DynLoadResponse: ConditionalSend {
    fn handle_asset_load_response(
        self: Box<Self>,
        storage: &mut AssetCacheStorage,
        loader: &mut AssetCacheLoad,
        dependency: &mut AssetCacheDependency,
        #[cfg(not(target_arch = "wasm32"))] reloader: &mut AssetCacheReload,
    );
}

impl<T: AssetLoader> DynLoadResponse for LoadResponse<T> {
    fn handle_asset_load_response(
        self: Box<Self>,
        storage: &mut AssetCacheStorage,
        loader: &mut AssetCacheLoad,
        dependency: &mut AssetCacheDependency,
        #[cfg(not(target_arch = "wasm32"))] reloader: &mut AssetCacheReload,
    ) {
        match self.result {
            LoadAssetResult::Success(asset) => {
                tracing::info!("load success {}", self.handle);
                let dyn_handle = self.handle.to_dyn();

                // Storage
                storage.insert_asset(self.handle.clone(), asset);
                storage.set_just_available(dyn_handle.clone());

                // Dependency
                dependency.set_dependencies(&dyn_handle.clone(), &self.dependencies);

                // Loader
                loader.reload_depending(dependency, storage, &dyn_handle, None);

                // Reloader
                #[cfg(not(target_arch = "wasm32"))]
                {
                    reloader.register_watches(dyn_handle.clone(), &self.watches);

                    // TODO: doesnt seem like this needed anymore due to reload above
                    // keep for now in case problems occur
                    // if reloader.is_currently_reloading(&dyn_handle) {
                    //     reloader.reload_dependents(dependency, loader, storage, &dyn_handle);
                    // }
                }
            }
            LoadAssetResult::Error => {
                tracing::error!("load error");
                let dyn_handle = self.handle.to_dyn();

                // Registry
                storage.set_asset_state(dyn_handle.clone(), InternalAssetState::Failed);

                // TODO: do we want this?
                // Dependency
                dependency.set_dependencies(&dyn_handle.clone(), &self.dependencies);
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
trait DynLoadRequest: ConditionalSend {
    fn get_or_load_asset(
        self: Box<Self>,
        loader: &mut AssetCacheLoad,
        registry: &mut AssetCacheRegistry,
        storage: &mut AssetCacheStorage,
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
        storage: &mut AssetCacheStorage,
    ) {
        let handle = loader.register_load::<T>(registry, storage, &self.settings);

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
pub(crate) trait DynInsertRequest: ConditionalSend {
    fn insert_asset(
        self: Box<Self>,
        registry: &mut AssetCacheRegistry,
        storage: &mut AssetCacheStorage,
        inserter: &mut AssetCacheInsert,
        loader: &mut AssetCacheLoad,
        dependency: &mut AssetCacheDependency,
    );
}

struct TypedInsertRequest<T: Asset, I: AssetInserter> {
    // TODO: make it always scoped
    scope: Option<DynAssetHandle>,
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
            scope: None,
        }
    }

    fn new_scoped(
        key: I::Key,
        asset: T,
        scope: DynAssetHandle,
        response_sender: async_channel::Sender<AssetHandle<T>>,
    ) -> Self {
        Self {
            key,
            asset,
            response_sender,
            scope: Some(scope),
        }
    }
}

impl<T: Asset, I: AssetInserter + 'static> DynInsertRequest for TypedInsertRequest<T, I> {
    fn insert_asset(
        self: Box<Self>,
        registry: &mut AssetCacheRegistry,
        storage: &mut AssetCacheStorage,
        inserter: &mut AssetCacheInsert,
        loader: &mut AssetCacheLoad,
        dependency: &mut AssetCacheDependency,
    ) {
        tracing::info!("insert nested asset {:?}", self.key);

        let handle = match &self.scope {
            Some(scope) => inserter.insert_asset_scoped::<T, I>(
                registry,
                storage,
                self.key,
                scope.clone(),
                self.asset,
            ),
            None => inserter.insert_asset::<T, I>(registry, storage, self.key, self.asset),
        };

        let dyn_handle = handle.to_dyn();
        loader.reload_depending(dependency, storage, &dyn_handle, self.scope);
        storage.set_just_available(dyn_handle);

        self.response_sender
            .try_send(handle)
            .expect("could not send insert asset handle response");
    }
}

//
// Get request
//

pub(crate) trait DynGetRequest: ConditionalSend {
    fn get_asset(self: Box<Self>, storage: &mut AssetCacheStorage);
}

pub(crate) struct TypedGetRequest<T: Asset> {
    handle: AssetHandle<T>,
    response_sender: async_channel::Sender<Arc<T>>,
}
impl<T: Asset> TypedGetRequest<T> {
    fn new(handle: AssetHandle<T>, response_sender: async_channel::Sender<Arc<T>>) -> Self {
        Self {
            handle,
            response_sender,
        }
    }
}

impl<T: Asset> DynGetRequest for TypedGetRequest<T> {
    fn get_asset(self: Box<Self>, storage: &mut AssetCacheStorage) {
        if let Some(asset) = storage.get_asset(&self.handle) {
            self.response_sender
                .try_send(asset.clone())
                .expect("could not send get request response");
        } else {
            tracing::info!("could not find {} in storage, request it", self.handle);
            storage.add_get_request(&self.handle, self.response_sender);
        }
    }
}

//
// Generic
//

pub(crate) struct AssetCacheLoad {
    typed_load: FxHashMap<TypeId, Box<dyn DynAssetLoad>>,
    task_ctx: TaskExecutorRuntime,
    filesystem_runtime: FileSystemRuntime,
    render_runtime: RenderRuntime,
    arc_runtime: ArcHandleRuntime,

    queue: VecDeque<DynAssetHandle>,
    queued: FxHashSet<DynAssetHandle>,
    handle_to_loader_type: FxHashMap<DynAssetHandle, TypeId>,

    // load request
    load_request_sender: async_channel::Sender<Box<dyn DynLoadRequest>>,
    load_request_receiver: async_channel::Receiver<Box<dyn DynLoadRequest>>,

    // insert request
    insert_request_sender: async_channel::Sender<Box<dyn DynInsertRequest>>,
    insert_request_receiver: async_channel::Receiver<Box<dyn DynInsertRequest>>,

    // get request
    get_request_sender: async_channel::Sender<Box<dyn DynGetRequest>>,
    get_request_receiver: async_channel::Receiver<Box<dyn DynGetRequest>>,

    // load response
    response_sender: async_channel::Sender<Box<dyn DynLoadResponse>>,
    response_receiver: async_channel::Receiver<Box<dyn DynLoadResponse>>,
}

impl AssetCacheLoad {
    pub(crate) fn new(
        task_ctx: TaskExecutorRuntime,
        filesystem_runtime: FileSystemRuntime,
        render_runtime: RenderRuntime,
        arc_runtime: ArcHandleRuntime,
    ) -> Self {
        let typed_load = FxHashMap::default();

        let (response_sender, response_receiver) = async_channel::unbounded();
        let (load_request_sender, load_request_receiver) = async_channel::unbounded();
        let (insert_request_sender, insert_request_receiver) = async_channel::unbounded();
        let (get_request_sender, get_request_receiver) = async_channel::unbounded();

        Self {
            task_ctx,
            filesystem_runtime,
            render_runtime,
            arc_runtime,

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

            get_request_sender,
            get_request_receiver,
        }
    }

    fn register_loader<T: AssetLoader + 'static>(&mut self) {
        let dyn_load = self.typed_load.entry(TypeId::of::<T>()).or_insert_with(|| {
            Box::new(TypedAssetLoad::<T>::new(
                self.task_ctx.clone(),
                self.filesystem_runtime.clone(),
                self.render_runtime.clone(),
                self.arc_runtime.clone(),
                self.load_request_sender.clone(),
                self.insert_request_sender.clone(),
                self.get_request_sender.clone(),
                self.response_sender.clone(),
            ))
        });
        let typed_load = dyn_load
            .as_any_mut()
            .downcast_mut::<TypedAssetLoad<T>>()
            .expect("could not downcast typed storage cache");

        let _ = typed_load;
    }

    /// Get mutable typed cache or create if it doesnt exist
    fn get_typed_cache_mut<T: AssetLoader + 'static>(&mut self) -> Option<&mut TypedAssetLoad<T>> {
        self.typed_load.get_mut(&TypeId::of::<T>()).map(|dyn_load| {
            dyn_load
                .as_any_mut()
                .downcast_mut::<TypedAssetLoad<T>>()
                .expect("could not downcast typed storage cache")
        })
    }

    // check if any files completed loading and update cache and invalidate render cache
    pub(crate) fn poll_loaded(
        &mut self,
        storage: &mut AssetCacheStorage,
        dependency: &mut AssetCacheDependency,
        #[cfg(not(target_arch = "wasm32"))] reloader: &mut AssetCacheReload,
    ) {
        while let Ok(response) = self.response_receiver.try_recv() {
            response.handle_asset_load_response(
                storage,
                self,
                dependency,
                #[cfg(not(target_arch = "wasm32"))]
                reloader,
            );
        }
    }

    // check for request of nested loads
    pub(crate) fn poll_load_requests(
        &mut self,
        registry: &mut AssetCacheRegistry,
        storage: &mut AssetCacheStorage,
    ) {
        while let Ok(load_request) = self.load_request_receiver.try_recv() {
            load_request.get_or_load_asset(self, registry, storage);
        }
    }

    // check for request of nested gets
    pub(crate) fn poll_get_requests(&mut self, storage: &mut AssetCacheStorage) {
        while let Ok(get_request) = self.get_request_receiver.try_recv() {
            get_request.get_asset(storage);
        }
    }

    // check for request of nested inserts
    pub(crate) fn poll_insert_requests(
        &mut self,
        registry: &mut AssetCacheRegistry,
        storage: &mut AssetCacheStorage,
        inserter: &mut AssetCacheInsert,
        dependency: &mut AssetCacheDependency,
    ) {
        while let Ok(request) = self.insert_request_receiver.try_recv() {
            request.insert_asset(registry, storage, inserter, self, dependency);
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
                panic!("could not get typed loader");
            };

            typed_load.load(registry, dyn_handle);
        }
    }

    //
    // Load
    //

    // TODO: maybe this should only be called when reloading is enabled?
    pub(crate) fn reload_depending(
        &mut self,
        dependency: &mut AssetCacheDependency,
        storage: &mut AssetCacheStorage,
        dyn_handle: &DynAssetHandle,
        ignore_handle: Option<DynAssetHandle>,
    ) {
        if let Some(dependents) = dependency.dependents(dyn_handle) {
            tracing::info!("reload {:?} due to {}", dependents, dyn_handle,);

            for dependent in dependents.iter() {
                if let Some(ignore_handle) = &ignore_handle {
                    if ignore_handle == dependent {
                        continue;
                    }
                }
                self.queue_load(storage, dependent.clone());
            }
        }
    }

    pub(crate) fn register_load<T: AssetLoader + 'static>(
        &mut self,
        registry: &mut AssetCacheRegistry,
        storage: &mut AssetCacheStorage,
        settings: &T::Settings,
    ) -> AssetHandle<T::Asset> {
        let handle = registry.get_or_create_load_handle::<T>(storage, settings);

        if let InternalAssetState::Pending = storage.get_asset_state(&handle) {
            tracing::info!("register load {}", handle);

            self.handle_to_loader_type
                .insert(handle.to_dyn(), TypeId::of::<T>());

            self.register_loader::<T>();

            self.queue_load(storage, handle.to_dyn());
        }

        handle
    }

    pub(crate) fn queue_load(&mut self, storage: &mut AssetCacheStorage, handle: DynAssetHandle) {
        tracing::info!("queue load for {}", handle);
        if self.queued.insert(handle.clone()) {
            storage.set_asset_state(handle.clone(), InternalAssetState::Loading);
            self.queue.push_back(handle);
        }
    }
}

//
// Typed
//

struct TypedAssetLoad<T: AssetLoader> {
    task_runtime: TaskExecutorRuntime,
    filesystem_runtime: FileSystemRuntime,
    render_runtime: RenderRuntime,
    arc_runtime: ArcHandleRuntime,

    load_request_sender: async_channel::Sender<Box<dyn DynLoadRequest>>,
    insert_request_sender: async_channel::Sender<Box<dyn DynInsertRequest>>,
    get_request_sender: async_channel::Sender<Box<dyn DynGetRequest>>,

    // Load response
    response_sender: async_channel::Sender<Box<dyn DynLoadResponse>>,

    // TODO: shouldnt be needed?
    ty: PhantomData<T>,
}

impl<T: AssetLoader + 'static> TypedAssetLoad<T> {
    fn new(
        task_runtime: TaskExecutorRuntime,
        filesystem_runtime: FileSystemRuntime,
        render_runtime: RenderRuntime,
        arc_runtime: ArcHandleRuntime,

        load_request_sender: async_channel::Sender<Box<dyn DynLoadRequest>>,
        insert_request_sender: async_channel::Sender<Box<dyn DynInsertRequest>>,
        get_request_sender: async_channel::Sender<Box<dyn DynGetRequest>>,

        response_sender: async_channel::Sender<Box<dyn DynLoadResponse>>,
    ) -> Self {
        Self {
            task_runtime,
            filesystem_runtime,
            render_runtime,
            arc_runtime,

            load_request_sender,
            insert_request_sender,
            get_request_sender,

            response_sender,
            ty: PhantomData,
        }
    }

    fn load_asset_with_handle(&mut self, handle: AssetHandle<T::Asset>, settings: T::Settings) {
        tracing::info!("spawn load {}", handle);

        let new_asset_state = LoadState::new(handle.to_dyn());
        let new_asset_runtime = LoadRuntime::new(
            self.filesystem_runtime.clone(),
            self.render_runtime.clone(),
            self.arc_runtime.clone(),
            self.load_request_sender.clone(),
            self.insert_request_sender.clone(),
            self.get_request_sender.clone(),
            self.response_sender.clone(),
        );

        let mut new_load_ctx = LoadContext::new(new_asset_state, new_asset_runtime);

        // spawn load
        self.task_runtime.spawn_task(Box::pin(async move {
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
    filesystem_runtime: filesystem::FileSystemRuntime,
    render_runtime: render::RenderRuntime,
    arc_runtime: arc::ArcHandleRuntime,

    // async channel requests
    load_request_sender: async_channel::Sender<Box<dyn DynLoadRequest>>,
    insert_request_sender: async_channel::Sender<Box<dyn DynInsertRequest>>,
    get_request_sender: async_channel::Sender<Box<dyn DynGetRequest>>,

    // async channel for returning result
    response_sender: async_channel::Sender<Box<dyn DynLoadResponse>>,
}

impl LoadRuntime {
    fn new(
        filesystem_runtime: filesystem::FileSystemRuntime,
        render_runtime: render::RenderRuntime,
        arc_runtime: arc::ArcHandleRuntime,

        load_request_sender: async_channel::Sender<Box<dyn DynLoadRequest>>,
        insert_request_sender: async_channel::Sender<Box<dyn DynInsertRequest>>,
        get_request_sender: async_channel::Sender<Box<dyn DynGetRequest>>,

        response_sender: async_channel::Sender<Box<dyn DynLoadResponse>>,
    ) -> Self {
        Self {
            filesystem_runtime,
            render_runtime,
            arc_runtime,

            response_sender,
            load_request_sender,
            insert_request_sender,

            get_request_sender,
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

    pub fn handle(&self) -> DynAssetHandle {
        self.state.handle.clone()
    }

    pub fn render_runtime(&self) -> &RenderRuntime {
        &self.runtime.render_runtime
    }

    pub fn arc_runtime(&self) -> &ArcHandleRuntime {
        &self.runtime.arc_runtime
    }

    /// Insert an asset with a specific inserter
    ///
    /// Is local to the currently loading handle to avoid global collisions
    pub async fn insert_asset_scoped<T: Asset, I: AssetInserter + 'static>(
        &mut self,
        key: impl Into<I::Key>,
        asset: T,
    ) -> AssetHandle<T> {
        tracing::info!("ASYNC: request nested load request for {}", self.handle());

        let (sender, receiver) = async_channel::bounded(1);

        self.runtime
            .insert_request_sender
            .send(Box::new(TypedInsertRequest::<T, I>::new_scoped(
                key.into(),
                asset,
                self.state.handle.clone(),
                sender,
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

    pub async fn request_get<T: Asset>(&mut self, handle: AssetHandle<T>) -> Arc<T> {
        tracing::info!("ASYNC: request nested get request for {}", self.handle());

        let (sender, receiver) = async_channel::bounded(1);

        self.runtime
            .get_request_sender
            .send(Box::new(TypedGetRequest::new(handle.clone(), sender)))
            .await
            .expect("could not send get request");

        let asset = receiver
            .recv()
            .await
            .expect("could not receive get request");

        self.state.dependencies.insert(handle.to_dyn());

        asset
    }

    pub async fn load_bytes(
        &mut self,
        path: impl AsRef<Path>,
    ) -> Result<Vec<u8>, filesystem::LoadFileError> {
        let result = self
            .runtime
            .filesystem_runtime
            .load_asset_bytes(&path)
            .await;

        if let Err(err) = &result {
            tracing::error!(
                "could not load nested bytes at {:?}: {}",
                path.as_ref(),
                err
            );
        }

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
        let result = self
            .runtime
            .filesystem_runtime
            .load_asset_string(&path)
            .await;

        if let Err(err) = &result {
            tracing::error!(
                "could not load nested string at {:?}: {}",
                path.as_ref(),
                err
            );
        }

        if result.is_ok() {
            #[cfg(not(target_arch = "wasm32"))]
            self.state.watches.insert(path.as_ref().to_path_buf());
        }

        result
    }
}
