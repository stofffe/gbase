#[cfg(not(target_arch = "wasm32"))]
use crate::asset::AssetCacheReload;

use crate::{
    arc::{self, ArcHandleRuntime},
    asset::{
        ArcAssetCacheRegistry, Asset, AssetCacheDependency, AssetCacheInsert, AssetCacheStorage,
        AssetHandle, AssetInserter, BackendMessage, DynAssetHandle, FrontendMessage,
        InternalAssetState,
    },
    filesystem::{self, FileSystemRuntime},
    render::{self, RenderRuntime},
    task::TaskExecutorRuntime,
    ConditionalSend,
};
use rustc_hash::{FxHashMap, FxHashSet};
use std::{
    any::{Any, TypeId},
    future::Future,
    hash::Hash,
    marker::PhantomData,
    path::PathBuf,
    pin::Pin,
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

pub(crate) trait DynLoadResult: ConditionalSend {
    /// Updates asset backend with the loaded asset
    ///
    /// Will return new loads for depending assets
    fn handle_load_result(
        self: Box<Self>,
        frontend_message_sender: async_channel::Sender<FrontendMessage>,
        registry: &ArcAssetCacheRegistry,
        storage: &mut AssetCacheStorage,
        loader: &mut AssetCacheLoad,
        dependency: &mut AssetCacheDependency,
        #[cfg(not(target_arch = "wasm32"))] reloader: &mut AssetCacheReload,
    ) -> Vec<BoxedAssetLoadFuture>;
}

impl<T: AssetLoader> DynLoadResult for LoadResponse<T> {
    fn handle_load_result(
        self: Box<Self>,
        frontend_message_sender: async_channel::Sender<FrontendMessage>,
        registry: &ArcAssetCacheRegistry,
        storage: &mut AssetCacheStorage,
        loader: &mut AssetCacheLoad,
        dependency: &mut AssetCacheDependency,
        #[cfg(not(target_arch = "wasm32"))] reloader: &mut AssetCacheReload,
    ) -> Vec<BoxedAssetLoadFuture> {
        let dyn_handle = self.handle.to_dyn();

        dependency.set_dependencies(&dyn_handle.clone(), &self.dependencies);

        #[cfg(not(target_arch = "wasm32"))]
        reloader.register_watches(dyn_handle.clone(), &self.watches);

        match self.result {
            LoadAssetResult::Success(asset) => {
                tracing::info!("load success {}", self.handle);

                storage.insert_asset(frontend_message_sender, self.handle.clone(), asset);

                loader.reload_depending(registry, dependency, storage, &dyn_handle, None)
            }
            LoadAssetResult::Error => {
                tracing::error!("load error");

                // TODO: should this clear the old asset?
                storage.set_asset_state(&dyn_handle, InternalAssetState::Failed);

                Vec::new()
            }
        }
    }
}

//
// Load Request
//

pub(crate) trait AssetLoadFuture:
    Future<Output = Box<dyn DynLoadResult>> + ConditionalSend
{
}
impl<T: Future<Output = Box<dyn DynLoadResult>> + ConditionalSend> AssetLoadFuture for T {}
pub(crate) type BoxedAssetLoadFuture = Pin<Box<dyn AssetLoadFuture>>;

/// Type erased load request
///
/// Is sent using async channels
pub(crate) trait DynLoadRequest: ConditionalSend {
    /// Loads an asset if needed
    fn load_asset_if_needed(
        self: Box<Self>,
        frontend_message_sender: async_channel::Sender<FrontendMessage>,
        registry: &ArcAssetCacheRegistry,
        loader: &mut AssetCacheLoad,
        storage: &mut AssetCacheStorage,
    ) -> Option<BoxedAssetLoadFuture>;
}

pub(crate) struct TypedLoadRequest<T: AssetLoader> {
    handle: AssetHandle<T::Asset>,
}

impl<T: AssetLoader> TypedLoadRequest<T> {
    pub(crate) fn new(handle: AssetHandle<T::Asset>) -> Self {
        Self { handle }
    }
}

// TODO: clean up?
impl<T: AssetLoader + 'static> DynLoadRequest for TypedLoadRequest<T> {
    fn load_asset_if_needed(
        self: Box<Self>,
        frontend_message_sender: async_channel::Sender<FrontendMessage>,
        registry: &ArcAssetCacheRegistry,
        loader: &mut AssetCacheLoad,
        storage: &mut AssetCacheStorage,
    ) -> Option<BoxedAssetLoadFuture> {
        storage.register_asset_if_needed(frontend_message_sender, self.handle.clone());
        loader.register_load_if_needed::<T>(storage, registry, self.handle.clone())
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
        frontend_message_sender: async_channel::Sender<FrontendMessage>,
        registry: &ArcAssetCacheRegistry,
        storage: &mut AssetCacheStorage,
        inserter: &mut AssetCacheInsert,
        loader: &mut AssetCacheLoad,
        dependency: &mut AssetCacheDependency,
    ) -> Vec<BoxedAssetLoadFuture>;
}

pub(crate) struct TypedInsertRequest<T: Asset> {
    handle: AssetHandle<T>,
    asset: T,
    scope: Option<DynAssetHandle>,
}

impl<T: Asset> TypedInsertRequest<T> {
    pub(crate) fn new(handle: AssetHandle<T>, asset: T, scope: Option<DynAssetHandle>) -> Self {
        Self {
            handle,
            asset,
            scope,
        }
    }
}

impl<T: Asset> DynInsertRequest for TypedInsertRequest<T> {
    fn insert_asset(
        self: Box<Self>,
        frontend_message_sender: async_channel::Sender<FrontendMessage>,
        registry: &ArcAssetCacheRegistry,
        storage: &mut AssetCacheStorage,
        inserter: &mut AssetCacheInsert,
        loader: &mut AssetCacheLoad,
        dependency: &mut AssetCacheDependency,
    ) -> Vec<BoxedAssetLoadFuture> {
        tracing::info!("insert nested asset {:?}", self.handle);

        let handle = inserter.insert_asset::<T>(
            frontend_message_sender,
            storage,
            self.handle,
            self.scope.clone(),
            self.asset,
        );

        // Reload all dependents while ignoring the one making the insertion
        let dyn_handle = handle.to_dyn();
        tracing::error!("reload dependeing ignoring {}", dyn_handle);
        loader.reload_depending(registry, dependency, storage, &dyn_handle, self.scope)
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
    registry: ArcAssetCacheRegistry,
    handle_to_loader_type: FxHashMap<DynAssetHandle, TypeId>,

    // load request
    backend_message_sender: async_channel::Sender<BackendMessage>,
}

impl AssetCacheLoad {
    pub(crate) fn new(
        task_ctx: TaskExecutorRuntime,
        filesystem_runtime: FileSystemRuntime,
        render_runtime: RenderRuntime,
        arc_runtime: ArcHandleRuntime,
        registry: ArcAssetCacheRegistry,
        backend_message_sender: async_channel::Sender<BackendMessage>,
    ) -> Self {
        let typed_load = FxHashMap::default();

        Self {
            task_ctx,
            filesystem_runtime,
            render_runtime,
            arc_runtime,
            registry,

            typed_load,

            handle_to_loader_type: FxHashMap::default(),

            backend_message_sender,
        }
    }

    fn register_loader<T: AssetLoader + 'static>(&mut self) {
        let dyn_load = self.typed_load.entry(TypeId::of::<T>()).or_insert_with(|| {
            Box::new(TypedAssetLoad::<T>::new(
                self.task_ctx.clone(),
                self.filesystem_runtime.clone(),
                self.render_runtime.clone(),
                self.arc_runtime.clone(),
                self.registry.clone(),
                self.backend_message_sender.clone(),
            ))
        });
        let typed_load = dyn_load
            .as_any_mut()
            .downcast_mut::<TypedAssetLoad<T>>()
            .expect("could not downcast typed storage cache");

        let _ = typed_load;
    }

    //
    // Load
    //

    // TODO: maybe this should only be called when reloading is enabled?
    pub(crate) fn reload_depending(
        &mut self,
        registry: &ArcAssetCacheRegistry,
        dependency: &mut AssetCacheDependency,
        storage: &mut AssetCacheStorage,
        dyn_handle: &DynAssetHandle,
        ignore_handle: Option<DynAssetHandle>,
    ) -> Vec<BoxedAssetLoadFuture> {
        let mut load_futures = Vec::new();
        if let Some(dependents) = dependency.dependents(dyn_handle) {
            tracing::info!("reload {:?} due to {}", dependents, dyn_handle,);

            for dependent in dependents.iter() {
                if let Some(ignore_handle) = &ignore_handle {
                    if ignore_handle == dependent {
                        continue;
                    }
                }
                let load_future = self.queue_load(registry, storage, dependent.clone());
                load_futures.push(load_future);
            }
        }

        load_futures
    }

    pub(crate) fn register_load_if_needed<T: AssetLoader + 'static>(
        &mut self,
        storage: &mut AssetCacheStorage,
        registry: &ArcAssetCacheRegistry,
        handle: AssetHandle<T::Asset>,
    ) -> Option<BoxedAssetLoadFuture> {
        if let InternalAssetState::Cleared = storage.get_asset_state(&handle.to_dyn()) {
            tracing::info!("register load {}", handle);

            self.handle_to_loader_type
                .insert(handle.to_dyn(), TypeId::of::<T>());

            self.register_loader::<T>();

            Some(self.queue_load(registry, storage, handle.to_dyn()))
        } else {
            None
        }
    }

    pub(crate) fn queue_load(
        &mut self,
        registry: &ArcAssetCacheRegistry,
        storage: &mut AssetCacheStorage,
        dyn_handle: DynAssetHandle,
    ) -> BoxedAssetLoadFuture {
        tracing::info!("queue load for {}", dyn_handle);

        let Some(type_id) = self.handle_to_loader_type.get(&dyn_handle) else {
            panic!("no loader registered for {}", dyn_handle);
        };

        let Some(typed_load) = self.typed_load.get_mut(type_id) else {
            panic!("could not get typed loader");
        };

        storage.set_asset_state(&dyn_handle, InternalAssetState::Loading);
        typed_load.load(registry, dyn_handle)
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
    registry: ArcAssetCacheRegistry,

    backend_message_sender: async_channel::Sender<BackendMessage>,

    // TODO: shouldnt be needed?
    ty: PhantomData<T>,
}

impl<T: AssetLoader + 'static> TypedAssetLoad<T> {
    fn new(
        task_runtime: TaskExecutorRuntime,
        filesystem_runtime: FileSystemRuntime,
        render_runtime: RenderRuntime,
        arc_runtime: ArcHandleRuntime,
        registry: ArcAssetCacheRegistry,

        backend_message_sender: async_channel::Sender<BackendMessage>,
    ) -> Self {
        Self {
            task_runtime,
            filesystem_runtime,
            render_runtime,
            arc_runtime,
            registry,

            backend_message_sender,
            ty: PhantomData,
        }
    }

    fn load_asset_with_handle(
        &mut self,
        handle: AssetHandle<T::Asset>,
        settings: T::Settings,
    ) -> BoxedAssetLoadFuture {
        tracing::info!("spawn loader task for {}", handle);

        let new_asset_state = LoadState::new(handle.to_dyn());
        let new_asset_runtime = LoadRuntime::new(
            self.filesystem_runtime.clone(),
            self.render_runtime.clone(),
            self.arc_runtime.clone(),
            self.registry.clone(),
            self.backend_message_sender.clone(),
        );

        let mut new_load_ctx = LoadContext::new(new_asset_state, new_asset_runtime);

        // spawn load
        Box::pin(async move {
            let data = T::load(&mut new_load_ctx, settings).await;

            match data {
                Ok(asset) => {
                    let response: Box<dyn DynLoadResult> = Box::new(LoadResponse {
                        handle: handle.clone(),
                        result: LoadAssetResult::<T>::Success(asset),
                        dependencies: new_load_ctx.state.dependencies,
                        watches: new_load_ctx.state.watches,
                    });
                    response
                }
                Err(err) => {
                    tracing::warn!("could not load asset {}", err);
                    let response: Box<dyn DynLoadResult> = Box::new(LoadResponse {
                        handle: handle.clone(),
                        result: LoadAssetResult::<T>::Error,
                        dependencies: new_load_ctx.state.dependencies,
                        watches: new_load_ctx.state.watches,
                    });
                    response
                }
            }
        })
    }
}
//
// Dyn
//

trait DynAssetLoad: ConditionalSend {
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn load(
        &mut self,
        registry: &ArcAssetCacheRegistry,
        dyn_handle: DynAssetHandle,
    ) -> BoxedAssetLoadFuture;
}

impl<T: AssetLoader + 'static> DynAssetLoad for TypedAssetLoad<T> {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self as &mut dyn Any
    }

    fn load(
        &mut self,
        registry: &ArcAssetCacheRegistry,
        dyn_handle: DynAssetHandle,
    ) -> BoxedAssetLoadFuture {
        let handle = dyn_handle
            .to_typed::<T::Asset>()
            .expect("could not convert dyn handle to typed");

        let Some(settings) = registry.get_load_settings::<T>(&handle) else {
            panic!("could not get settings from handle");
        };

        // TODO: just move everything in this func here
        self.load_asset_with_handle(handle, settings)
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
    registry: ArcAssetCacheRegistry,

    backend_message_sender: async_channel::Sender<BackendMessage>,
}

impl LoadRuntime {
    fn new(
        filesystem_runtime: filesystem::FileSystemRuntime,
        render_runtime: render::RenderRuntime,
        arc_runtime: arc::ArcHandleRuntime,
        registry: ArcAssetCacheRegistry,

        backend_message_sender: async_channel::Sender<BackendMessage>,
    ) -> Self {
        Self {
            filesystem_runtime,
            render_runtime,
            arc_runtime,
            registry,

            backend_message_sender,
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
    pub fn request_insert<T: Asset, I: AssetInserter + 'static>(
        &mut self,
        key: impl Into<I::Key>,
        asset: T,
    ) -> AssetHandle<T> {
        let key = key.into();
        tracing::info!(
            "request load request for {:?} in {}",
            key.clone(),
            self.handle()
        );

        let handle = self
            .runtime
            .registry
            .get_insert_handle::<T, I>(key.clone(), Some(self.handle()));

        self.runtime
            .backend_message_sender
            .try_send(BackendMessage::Insert(Box::new(
                TypedInsertRequest::<T>::new(
                    handle.clone(),
                    asset,
                    Some(self.state.handle.clone()),
                ),
            )))
            .expect("could not send insert request");

        self.state.dependencies.insert(handle.to_dyn());

        handle
    }

    // nested
    pub fn request_load<T: AssetLoader + 'static>(
        &mut self,
        settings: T::Settings,
    ) -> AssetHandle<T::Asset> {
        tracing::info!(
            "request load request for {:?} in {}",
            settings,
            self.handle()
        );

        // TODO: check if you need to load again
        let handle = self.runtime.registry.get_load_handle::<T>(&settings);

        self.runtime
            .backend_message_sender
            .try_send(BackendMessage::Load(Box::new(TypedLoadRequest::<T>::new(
                handle.clone(),
            ))))
            .expect("could not send load request");

        self.state.dependencies.insert(handle.to_dyn());

        handle
    }

    pub async fn request_get<T: Asset>(&mut self, handle: AssetHandle<T>) -> Arc<T> {
        tracing::info!(
            "await start: request get request for {} in {}",
            handle,
            self.handle()
        );

        let (sender, receiver) = async_channel::bounded(1);

        self.runtime
            .backend_message_sender
            .send(BackendMessage::Get(Box::new(TypedGetRequest::new(
                handle.clone(),
                sender,
            ))))
            .await
            .expect("could not send get request");

        let Ok(asset) = receiver.recv().await else {
            panic!("could not receive get request for {}", handle);
        };

        tracing::info!(
            "await done: recieve get request for {} in {}",
            handle,
            self.handle()
        );

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

    pub fn set_debug_name<T: Asset>(
        &mut self,
        handle: AssetHandle<T>,
        debug_name: impl Into<String>,
    ) {
        self.runtime
            .backend_message_sender
            .try_send(BackendMessage::SetDebugName {
                handle: handle.to_dyn(),
                name: debug_name.into(),
            })
            .expect("could not send set debug name request");
    }
}
