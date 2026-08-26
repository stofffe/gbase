use crate::{
    asset::{
        Asset, AssetCacheDependency, AssetCacheLoad, AssetCacheRegistry, AssetCacheStorage,
        AssetHandle, AssetHandleContext, DynAssetHandle, GetAssetResult, LoadStatus,
    },
    render::ArcHandle,
    Context,
};
use core::{error, panic};
use rustc_hash::{FxHashMap, FxHashSet};
use std::{
    any::{type_name, Any, TypeId},
    collections::VecDeque,
    hash::Hash,
    marker::PhantomData,
};

//
// Types
//

pub trait DerivedAssetSettings: Hash + Eq + Clone {}
impl<T: Hash + Eq + Clone> DerivedAssetSettings for T {} // TODO: maybe do this for Asset and derived asset

pub trait AssetConverter {
    type Asset: Asset;
    type Settings: DerivedAssetSettings;
    // TODO: is this even being used?
    type Error: error::Error;

    fn convert(
        ctx: &mut Context,
        convert_ctx: &mut ConvertContext<'_>, // TODO: should this be mutable reference?
        settings: &Self::Settings,
    ) -> ConvertAssetStatus<Self::Asset>;
}

pub enum ConvertAssetStatus<T: Asset> {
    Loading,
    Success(T),
    Failed,
}

pub struct ConvertRequest<T: AssetConverter> {
    handle: AssetHandle<T::Asset>,
    settings: T::Settings,
}

impl<T: AssetConverter> ConvertRequest<T> {
    pub fn new(handle: AssetHandle<T::Asset>, settings: T::Settings) -> Self {
        Self { handle, settings }
    }
}

pub trait DynConvertRequest {
    fn handle(&self) -> DynAssetHandle;
    fn request(self: Box<Self>, convert: &mut AssetCacheConvert, registry: &mut AssetCacheRegistry);
}

impl<T: AssetConverter + 'static> DynConvertRequest for ConvertRequest<T> {
    fn request(
        self: Box<Self>,
        convert: &mut AssetCacheConvert,
        registry: &mut AssetCacheRegistry,
    ) {
        let handle = convert.register_conversion::<T>(registry, self.settings.clone());
        convert.queue_conversion(registry, handle.to_dyn());
    }

    fn handle(&self) -> DynAssetHandle {
        self.handle.to_dyn()
    }
}

//
// Generic
//

pub struct AssetCacheConvert {
    typed: FxHashMap<TypeId, Box<dyn DynAssetConvert>>,

    // queue
    queue: VecDeque<DynAssetHandle>,
    queued: FxHashSet<DynAssetHandle>,

    // waiting
    waiting_for: FxHashMap<DynAssetHandle, FxHashSet<DynAssetHandle>>,

    handle_to_converter_type: FxHashMap<DynAssetHandle, TypeId>,
}

impl AssetCacheConvert {
    pub fn new() -> Self {
        Self {
            typed: FxHashMap::default(),

            queue: VecDeque::default(),
            queued: FxHashSet::default(),

            waiting_for: FxHashMap::default(),

            handle_to_converter_type: FxHashMap::default(),
        }
    }

    /// Get typed cache assuming it exists
    pub fn get_typed_cache_ref<T: AssetConverter + 'static>(
        &self,
    ) -> Option<&TypedAssetConvert<T>> {
        self.typed.get(&TypeId::of::<T>()).map(|dyn_convert| {
            dyn_convert
                .as_any()
                .downcast_ref::<TypedAssetConvert<T>>()
                .expect("could not downcast typed storage cache")
        })
    }

    /// Get mutable typed cache or create if it doesnt exist
    pub fn get_typed_cache_mut<T: AssetConverter + 'static>(
        &mut self,
    ) -> &mut TypedAssetConvert<T> {
        let entry = self
            .typed
            .entry(TypeId::of::<T>())
            .or_insert(Box::new(TypedAssetConvert::<T>::new()));
        entry
            .as_any_mut()
            .downcast_mut::<TypedAssetConvert<T>>()
            .expect("could not downcast typed storage cache")
    }

    pub fn poll_conversions(
        &mut self,
        ctx: &mut Context,
        storage: &mut AssetCacheStorage,
        loader: &mut AssetCacheLoad,
        dependency: &mut AssetCacheDependency,
        registry: &mut AssetCacheRegistry,
    ) {
        while let Some(dyn_handle) = self.queue.pop_front() {
            self.queued.remove(&dyn_handle);

            let Some(type_id) = self.handle_to_converter_type.get(&dyn_handle) else {
                tracing::warn!("no converter registered for {}", dyn_handle);
                continue;
            };

            let Some(typed_converter) = self.typed.get_mut(type_id) else {
                panic!("could not get typed converter");
            };

            tracing::info!("-- start polled conversion {}", dyn_handle);
            let (result, request) = typed_converter.convert(
                ctx,
                storage,
                loader,
                dependency,
                registry,
                dyn_handle.clone(),
            );
            tracing::info!("-- done polled conversion {}", dyn_handle);

            // request conversions
            if let Some(request) = request {
                // panic!(
                //     "request nested conversion through return {}",
                //     request.handle()
                // );
                tracing::info!("send request for converison of {}", request.handle());
                request.request(self, registry);
            }

            match result {
                ConversionPollResult::Waiting(loading_handle) => {
                    tracing::info!(
                        "conversion {} waiting for asset handle {}",
                        dyn_handle,
                        loading_handle
                    );

                    // TODO: feel like i need to
                    // remove any pending handles
                    self.waiting_for.retain(|_, handles| {
                        handles.remove(&dyn_handle);
                        !handles.is_empty()
                    });

                    // add waiting handle
                    self.waiting_for
                        .entry(loading_handle)
                        .or_default()
                        .insert(dyn_handle);
                }
                ConversionPollResult::Failed(failing_handle) => {
                    tracing::info!("conversion failed {}", dyn_handle);

                    // TODO: feel like i need to
                    // remove any pending handles
                    self.waiting_for.retain(|_, handles| {
                        handles.remove(&dyn_handle);
                        !handles.is_empty()
                    });

                    self.waiting_for
                        .entry(failing_handle)
                        .or_default()
                        .insert(dyn_handle);
                }
                ConversionPollResult::Success => {
                    // remove any pending handles
                    self.waiting_for.retain(|_, handles| {
                        handles.remove(&dyn_handle);
                        !handles.is_empty()
                    });
                    self.wakeup_waiting_on_handle(registry, &dyn_handle);
                    self.reload_depending_conversions(dependency, registry, &dyn_handle);
                    registry.set_status(dyn_handle, LoadStatus::Ready);
                }
            }
        }
    }

    // setup conversion state
    pub fn register_conversion<T: AssetConverter + 'static>(
        &mut self,
        registry: &mut AssetCacheRegistry,
        settings: T::Settings,
    ) -> AssetHandle<T::Asset> {
        let handle = registry.get_or_create_convert_handle::<T>(settings.clone());

        if let LoadStatus::NotRegistered = registry.get_status(&handle.to_dyn()) {
            tracing::info!("register conversion {}", handle);

            self.handle_to_converter_type
                .insert(handle.to_dyn(), TypeId::of::<T>());

            self.get_typed_cache_mut::<T>();

            self.queue_conversion(registry, handle.to_dyn());
        }

        handle
    }

    // Queue a conversion of a derived handle
    //
    // Assumes all state is set up from queue_conversion function
    pub fn queue_conversion(&mut self, registry: &mut AssetCacheRegistry, handle: DynAssetHandle) {
        // // TODO: this must check if its convertable?
        // match registry.get_status(&handle) {
        //     LoadStatus::NotRegistered => {
        //         panic!("{} has no status", handle)
        //     }
        //     _ => tracing::info!("{} has status", handle),
        // }

        tracing::info!("queue conversion of {}", handle);
        if self.queued.insert(handle.clone()) {
            registry.set_status(handle.clone(), LoadStatus::Loading);
            self.queue.push_back(handle);
        }
    }

    /// Wake up all derived assets waiting for this handle to be ready
    pub fn wakeup_waiting_on_handle(
        &mut self,
        registry: &mut AssetCacheRegistry,
        dependency: &DynAssetHandle,
    ) {
        tracing::info!(
            "wake all waiting on {}: {:?}",
            dependency,
            self.waiting_for.get(dependency)
        );

        let Some(waiting_handles) = self.waiting_for.remove(dependency) else {
            return;
        };

        for handle in waiting_handles {
            tracing::info!("-> wake up {}", handle);
            self.queue_conversion(registry, handle);
        }
    }

    // similar to reload
    pub fn reload_depending_conversions(
        &mut self,
        dependency: &mut AssetCacheDependency,
        registry: &mut AssetCacheRegistry,
        handle: &DynAssetHandle,
    ) {
        if let Some(dependents) = dependency.dependents(handle) {
            tracing::info!("requeue dependents due to {}, len {:?}", handle, dependents);

            for dependent in dependents.iter() {
                self.queue_conversion(registry, dependent.clone());
            }
        }
    }
}

//
// Typed
//

pub struct TypedAssetConvert<T: AssetConverter> {
    ty: PhantomData<T>,
}

impl<T: AssetConverter + 'static> TypedAssetConvert<T> {
    pub fn new() -> Self {
        Self { ty: PhantomData }
    }
}

pub enum ConversionPollResult {
    Waiting(DynAssetHandle),
    Failed(DynAssetHandle),
    Success,
}

//
// Dyn
//

pub trait DynAssetConvert {
    fn convert(
        &mut self,
        ctx: &mut Context,
        storage: &mut AssetCacheStorage,
        loader: &mut AssetCacheLoad,
        dependency: &mut AssetCacheDependency,
        registry: &mut AssetCacheRegistry,
        dyn_handle: DynAssetHandle,
    ) -> (ConversionPollResult, Option<Box<dyn DynConvertRequest>>);
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T: AssetConverter + 'static> DynAssetConvert for TypedAssetConvert<T> {
    fn convert(
        &mut self,
        ctx: &mut Context,
        storage: &mut AssetCacheStorage,
        loader: &mut AssetCacheLoad,
        dependency: &mut AssetCacheDependency,
        registry: &mut AssetCacheRegistry,
        dyn_handle: DynAssetHandle,
    ) -> (ConversionPollResult, Option<Box<dyn DynConvertRequest>>) {
        // TODO: maybe just store the dyn directly?
        let handle = dyn_handle
            .to_typed::<T::Asset>()
            .expect("could not convert dyn handle to typed");

        let Some(settings) = registry
            .get_typed_convert_registry_mut::<T>()
            .handle_to_settings
            .get(&handle.to_dyn())
            .cloned()
        else {
            panic!("could not get settings from handle");
        };

        let mut runtime = ConvertRuntime::new(storage, loader, registry);
        let mut convert_ctx = ConvertContext::new(&mut runtime);

        let conversion = T::convert(ctx, &mut convert_ctx, &settings);

        let blocking_handle = convert_ctx.state.blocking_handle.clone();
        let dependencies = convert_ctx.state.dependencies.clone();

        match conversion {
            ConvertAssetStatus::Loading => (
                ConversionPollResult::Waiting(
                    blocking_handle.expect("should have blocking dependency if in loading status"),
                ),
                convert_ctx.state.conversion_request,
            ),
            ConvertAssetStatus::Success(asset) => {
                // insert into typed storage
                storage.insert::<T::Asset>(handle.clone(), asset);

                // set as just available
                // TODO: should this be set here?
                registry.set_status(handle.to_dyn(), LoadStatus::Ready);
                registry.set_just_available(handle.to_dyn());

                // register deps
                dependency.register_dependencies(&handle.to_dyn(), &dependencies);

                (ConversionPollResult::Success, None)
            }
            ConvertAssetStatus::Failed => (
                ConversionPollResult::Failed(
                    // TODO: temp
                    blocking_handle.expect("should have blocking dependency if in loading status"),
                ),
                None,
            ),
        }
    }

    fn as_any(&self) -> &dyn Any {
        self as &dyn Any
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self as &mut dyn Any
    }
}

//
// Conversion context
//

pub struct ConvertRuntime<'a> {
    // to get assets
    pub(crate) storage: &'a mut AssetCacheStorage,
    // to request new loads if no cached value exist in storage
    pub(crate) loader: &'a mut AssetCacheLoad,
    // to get setting -> derived handle mapping
    pub(crate) registry: &'a mut AssetCacheRegistry,
}

impl<'a> ConvertRuntime<'a> {
    pub fn new(
        storage: &'a mut AssetCacheStorage,
        loader: &'a mut AssetCacheLoad,
        registry: &'a mut AssetCacheRegistry,
    ) -> Self {
        Self {
            storage,
            loader,
            registry,
        }
    }
}

pub struct ConvertState {
    // The dependency that caused the conversion to return waiting
    pub blocking_handle: Option<DynAssetHandle>,

    // If wait_for was a nested conversion that resulted in a new handle, store the request here
    pub conversion_request: Option<Box<dyn DynConvertRequest>>,

    // All dependencies accessed during the conversion
    pub dependencies: FxHashSet<DynAssetHandle>,
}

impl ConvertState {
    pub fn new() -> Self {
        Self {
            blocking_handle: None,
            conversion_request: None,
            dependencies: FxHashSet::default(),
        }
    }
}

/// Convertsion context related to a specific conversion
pub struct ConvertContext<'runtime> {
    pub runtime: &'runtime mut ConvertRuntime<'runtime>,
    pub state: ConvertState,
}

impl<'runtime> ConvertContext<'runtime> {
    pub fn new(runtime: &'runtime mut ConvertRuntime<'runtime>) -> Self {
        let state = ConvertState::new();
        Self { runtime, state }
    }

    pub fn get_asset<T: Asset>(&mut self, handle: &AssetHandle<T>) -> GetAssetResult<'_, T> {
        tracing::info!("conversion get {}", handle);

        // register deps
        self.state.dependencies.insert(handle.to_dyn());

        if let Some(asset) = self.runtime.storage.get(handle) {
            return GetAssetResult::Success(asset);
        }

        match self.runtime.registry.get_status(&handle.to_dyn()) {
            LoadStatus::Loading => {
                self.state.blocking_handle = Some(handle.to_dyn());
                tracing::info!("{} is not ready, set blocking", handle);
                GetAssetResult::Loading
            }
            LoadStatus::Failed => {
                self.state.blocking_handle = Some(handle.to_dyn());
                tracing::info!("{} failed, set blocking", handle);
                GetAssetResult::Error
            }
            LoadStatus::Ready => panic!("could not get asset from storage but status is ready"),
            LoadStatus::NotRegistered => panic!("trying to get unregistered asset"),
        }
    }

    pub fn convert_asset<T: AssetConverter + 'static>(
        &mut self,
        settings: &T::Settings,
    ) -> GetAssetResult<'_, T::Asset> {
        // get handle from registry
        let handle = self
            .runtime
            .registry
            .get_or_create_convert_handle::<T>(settings.clone());

        tracing::info!("conversion convert {}", handle);

        match self.runtime.storage.get(&handle) {
            Some(asset) => GetAssetResult::Success(asset),
            None => {
                match self.runtime.registry.get_status(&handle.to_dyn()) {
                    LoadStatus::Ready => {
                        panic!("could not get asset from storage but status is ready")
                    }
                    LoadStatus::Loading => {
                        tracing::info!("{} is not ready, set blocking", handle);
                        self.state.blocking_handle = Some(handle.to_dyn());
                        GetAssetResult::Loading
                    }
                    LoadStatus::Failed => {
                        tracing::info!("{} failed, set blocking", handle);
                        self.state.blocking_handle = Some(handle.to_dyn());
                        GetAssetResult::Error
                    }
                    // TODO: something is not being sent here
                    LoadStatus::NotRegistered => {
                        // register new converison
                        tracing::info!(
                            "{} is not registered, request new and set blocking",
                            handle
                        );
                        self.state.conversion_request = Some(Box::new(ConvertRequest::<T>::new(
                            handle.clone(),
                            settings.clone(),
                        )));
                        self.state.blocking_handle = Some(handle.to_dyn());
                        // TODO: we have to register the status
                        // self.runtime
                        //     .registry
                        //     .register_convert_handle_settings_mapping::<T>(
                        //         handle.clone(),
                        //         settings.clone(),
                        //     );
                        // self.runtime
                        //     .registry
                        //     .set_status(handle.to_dyn(), LoadStatus::Loading);
                        GetAssetResult::Loading
                    }
                }
            }
        }
    }
}

// NOTE: user facing
pub enum ConvertAssetResult<T: Asset> {
    Loading,
    Success(ArcHandle<T>),
    Failed,
}

impl<T: Asset> ConvertAssetResult<T> {
    /// Unwrap the result as a success
    ///
    /// Panics for other values than
    pub fn unwrap_success(self) -> ArcHandle<T> {
        match self {
            ConvertAssetResult::Loading => {
                panic!("asset conversion loading: unwrap success failed")
            }
            ConvertAssetResult::Failed => panic!("asset conversion failed: unwrap success failed"),
            ConvertAssetResult::Success(arc_handle) => arc_handle,
        }
    }
}
