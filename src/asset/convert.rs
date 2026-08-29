use crate::{
    asset::{
        Asset, AssetCacheDependency, AssetCacheLoad, AssetCacheRegistry, AssetCacheStorage,
        AssetHandle, DynAssetHandle, GetAssetState, InternalAssetState,
    },
    Context,
};
use core::{error, panic};
use rustc_hash::{FxHashMap, FxHashSet};
use std::{
    any::{Any, TypeId},
    collections::VecDeque,
    fmt::Debug,
    hash::Hash,
    marker::PhantomData,
};
use tracing::span;

//
// Types
//

pub trait ConvertAssetSettings: Debug + Hash + Eq + Clone {}
impl<T: Debug + Hash + Eq + Clone> ConvertAssetSettings for T {}

pub trait AssetConverter {
    type Asset: Asset;
    type Settings: ConvertAssetSettings;
    // TODO: is this even being used?
    type Error: error::Error;

    fn convert(
        ctx: &mut Context,
        convert_ctx: &mut ConvertContext<'_>,
        settings: &Self::Settings,
    ) -> ConvertAssetState<Self::Asset>;
}

pub enum ConvertAssetState<T: Asset> {
    Loading,
    Success(T),
    Failed,
}

//
// Request
//

struct ConvertRequest<T: AssetConverter> {
    handle: AssetHandle<T::Asset>,
    settings: T::Settings,
}

impl<T: AssetConverter> ConvertRequest<T> {
    fn new(handle: AssetHandle<T::Asset>, settings: T::Settings) -> Self {
        Self { handle, settings }
    }
}

trait DynConvertRequest {
    fn handle(&self) -> DynAssetHandle;
    fn request(self: Box<Self>, convert: &mut AssetCacheConvert, registry: &mut AssetCacheRegistry);
}

impl<T: AssetConverter + 'static> DynConvertRequest for ConvertRequest<T> {
    fn request(
        self: Box<Self>,
        convert: &mut AssetCacheConvert,
        registry: &mut AssetCacheRegistry,
    ) {
        let handle = convert.register_conversion::<T>(registry, &self.settings);
        convert.queue_conversion(registry, handle.to_dyn());
    }

    fn handle(&self) -> DynAssetHandle {
        self.handle.to_dyn()
    }
}

//
// Generic
//

pub(crate) struct AssetCacheConvert {
    typed_convert: FxHashMap<TypeId, Box<dyn DynAssetConvert>>,

    // queue
    queue: VecDeque<DynAssetHandle>,
    queued: FxHashSet<DynAssetHandle>,

    // waiting
    waiting_for: FxHashMap<DynAssetHandle, FxHashSet<DynAssetHandle>>,

    handle_to_converter_type: FxHashMap<DynAssetHandle, TypeId>,
}

impl AssetCacheConvert {
    pub(crate) fn new() -> Self {
        Self {
            typed_convert: FxHashMap::default(),

            queue: VecDeque::default(),
            queued: FxHashSet::default(),

            waiting_for: FxHashMap::default(),

            handle_to_converter_type: FxHashMap::default(),
        }
    }

    /// Get mutable typed cache or create if it doesnt exist
    fn get_typed_cache_mut<T: AssetConverter + 'static>(&mut self) -> &mut TypedAssetConvert<T> {
        let entry = self
            .typed_convert
            .entry(TypeId::of::<T>())
            .or_insert(Box::new(TypedAssetConvert::<T>::new()));
        entry
            .as_any_mut()
            .downcast_mut::<TypedAssetConvert<T>>()
            .expect("could not downcast typed storage cache")
    }

    pub(crate) fn poll_conversions(
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

            let Some(typed_converter) = self.typed_convert.get_mut(type_id) else {
                panic!("could not get typed converter");
            };

            let mut convert_runtime = ConvertRuntime::new(storage, loader, registry);

            let _conver_span = span!(tracing::Level::INFO, "converting").entered();
            tracing::info!("start polled conversion {}", dyn_handle);
            let (result, state) =
                typed_converter.convert(ctx, &mut convert_runtime, dyn_handle.clone());
            _conver_span.exit();

            // request conversions
            if let Some(request) = state.conversion_request {
                tracing::info!("send request for converison of {}", request.handle());
                request.request(self, registry);
            }

            // TODO: should this be here?
            dependency.set_dependencies(&dyn_handle, &state.dependencies);

            match result {
                ConversionPollResult::Waiting => {
                    let blocking_handle = state
                        .blocking_handle
                        .expect("blocking handle cant be None if Waiting was returned");

                    tracing::info!(
                        "conversion {} waiting for asset handle {}",
                        dyn_handle,
                        blocking_handle
                    );

                    // TODO: keep?
                    self.clear_waiting_handles(&dyn_handle);

                    // add waiting handle
                    self.waiting_for
                        .entry(blocking_handle)
                        .or_default()
                        .insert(dyn_handle);
                }
                ConversionPollResult::Failed => {
                    tracing::info!("conversion failed {}", dyn_handle);

                    // TODO: keep?
                    self.clear_waiting_handles(&dyn_handle);

                    if let Some(blocking_handle) = state.blocking_handle {
                        self.waiting_for
                            .entry(blocking_handle)
                            .or_default()
                            .insert(dyn_handle);
                    }
                }
                ConversionPollResult::Success => {
                    registry.set_status(dyn_handle.clone(), InternalAssetState::Ready);
                    registry.set_just_available(dyn_handle.clone());

                    // TODO: keep?
                    self.clear_waiting_handles(&dyn_handle);

                    self.wakeup_waiting_on_handle(registry, &dyn_handle);

                    self.reload_depending_conversions(dependency, registry, &dyn_handle);

                    registry.set_status(dyn_handle, InternalAssetState::Ready);
                }
            }
        }
    }

    // TODO: should this even be called?
    fn clear_waiting_handles(&mut self, dyn_handle: &DynAssetHandle) {
        self.waiting_for.retain(|_, handles| {
            handles.remove(dyn_handle);
            !handles.is_empty()
        });
    }

    // setup conversion state
    pub(crate) fn register_conversion<T: AssetConverter + 'static>(
        &mut self,
        registry: &mut AssetCacheRegistry,
        settings: &T::Settings,
    ) -> AssetHandle<T::Asset> {
        let handle = registry.get_or_create_convert_handle::<T>(settings);

        if let InternalAssetState::NotRegistered = registry.get_status(handle.to_dyn()) {
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
    pub(crate) fn queue_conversion(
        &mut self,
        registry: &mut AssetCacheRegistry,
        handle: DynAssetHandle,
    ) {
        tracing::info!("queue conversion of {}", handle);
        if self.queued.insert(handle.clone()) {
            registry.set_status(handle.clone(), InternalAssetState::Loading);
            self.queue.push_back(handle);
        }
    }

    /// Wake up all derived assets waiting for this handle to be ready
    pub(crate) fn wakeup_waiting_on_handle(
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
    pub(crate) fn reload_depending_conversions(
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

struct TypedAssetConvert<T: AssetConverter> {
    ty: PhantomData<T>,
}

impl<T: AssetConverter + 'static> TypedAssetConvert<T> {
    fn new() -> Self {
        Self { ty: PhantomData }
    }

    // top level conversion
    fn convert_asset<'a>(
        &mut self,
        ctx: &mut Context,
        runtime: &'a mut ConvertRuntime<'a>,
        dyn_handle: DynAssetHandle,
    ) -> (ConversionPollResult, ConvertState) {
        // TODO: maybe just store the dyn directly?
        let handle = dyn_handle
            .to_typed::<T::Asset>()
            .expect("could not convert dyn handle to typed");

        let Some(settings) = runtime
            .registry
            .get_convert_settings_from_handle::<T>(&dyn_handle)
        else {
            panic!("could not get settings from handle");
        };

        let mut convert_ctx = ConvertContext::new(runtime);

        let conversion = T::convert(ctx, &mut convert_ctx, &settings);
        let state = convert_ctx.state;

        // TODO: should blocking be applied here?
        // dont think so since nested fn in T::conert should handle this

        match conversion {
            ConvertAssetState::Success(asset) => {
                tracing::info!("conversion of {} success", handle);
                // insert into typed storage
                convert_ctx
                    .runtime
                    .storage
                    .insert_asset::<T::Asset>(handle.clone(), asset);

                (ConversionPollResult::Success, state)
            }
            ConvertAssetState::Loading => {
                tracing::info!("conversion of {} loading", handle);
                (ConversionPollResult::Waiting, state)
            }
            ConvertAssetState::Failed => {
                tracing::info!("conversion of {} failed", handle);
                (ConversionPollResult::Failed, state)
            }
        }
    }
}

//
// Dyn
//

#[derive(Debug)]
enum ConversionPollResult {
    Waiting,
    Failed,
    Success,
}

trait DynAssetConvert {
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn convert<'a>(
        &mut self,
        ctx: &mut Context,
        runtime: &'a mut ConvertRuntime<'a>,
        dyn_handle: DynAssetHandle,
    ) -> (ConversionPollResult, ConvertState);
}

impl<T: AssetConverter + 'static> DynAssetConvert for TypedAssetConvert<T> {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self as &mut dyn Any
    }

    fn convert<'a>(
        &mut self,
        ctx: &mut Context,
        runtime: &'a mut ConvertRuntime<'a>,
        dyn_handle: DynAssetHandle,
    ) -> (ConversionPollResult, ConvertState) {
        self.convert_asset(ctx, runtime, dyn_handle)
    }
}

//
// Conversion context
//

/// Convertsion context related to a specific conversion
pub struct ConvertContext<'runtime> {
    runtime: &'runtime mut ConvertRuntime<'runtime>,
    state: ConvertState,
}

impl<'runtime> ConvertContext<'runtime> {
    fn new(runtime: &'runtime mut ConvertRuntime<'runtime>) -> Self {
        let state = ConvertState::new();
        Self { runtime, state }
    }

    pub fn get_asset<T: Asset>(&mut self, handle: &AssetHandle<T>) -> Result<&T, GetAssetState> {
        tracing::info!("conversion get {}", handle);

        // register deps
        self.state.dependencies.insert(handle.to_dyn());

        if let Some(asset) = self.runtime.storage.get_asset(handle) {
            tracing::info!("get {} success", handle);
            return Ok(asset);
        }

        let state = self.runtime.registry.get_status(handle.to_dyn());
        match state {
            InternalAssetState::Loading => {
                self.state.blocking_handle = Some(handle.to_dyn());
                tracing::info!("waiting for {}", handle);
                Err(GetAssetState::Loading)
            }
            InternalAssetState::Failed => {
                self.state.blocking_handle = Some(handle.to_dyn());
                tracing::info!("erron in {}", handle);
                Err(GetAssetState::Failed)
            }
            InternalAssetState::Ready => panic!(
                "could not get asset from storage but status is ready {}",
                handle
            ),
            InternalAssetState::NotRegistered => {
                panic!("trying to get unregistered asset {}", handle)
            }
        }
    }

    pub fn convert_asset<T: AssetConverter + 'static>(
        &mut self,
        settings: &T::Settings,
    ) -> Result<&T::Asset, GetAssetState> {
        // get handle from registry
        let handle = self
            .runtime
            .registry
            .get_or_create_convert_handle::<T>(settings);

        tracing::info!("conversion convert {}", handle);

        // register deps
        self.state.dependencies.insert(handle.to_dyn());

        if let Some(asset) = self.runtime.storage.get_asset(&handle) {
            tracing::info!("get {} success", handle);
            return Ok(asset);
        }

        self.state.blocking_handle = Some(handle.to_dyn());

        match self.runtime.registry.get_status(handle.to_dyn()) {
            InternalAssetState::Loading => {
                tracing::info!("{} is not ready, set blocking", handle);
                Err(GetAssetState::Loading)
            }
            InternalAssetState::Failed => {
                tracing::info!("{} failed, set blocking", handle);
                Err(GetAssetState::Failed)
            }
            InternalAssetState::NotRegistered => {
                // register new converison
                tracing::info!("{} is not registered, request new and set blocking", handle);
                self.state.conversion_request = Some(Box::new(ConvertRequest::<T>::new(
                    handle.clone(),
                    settings.clone(),
                )));
                Err(GetAssetState::Loading)
            }
            InternalAssetState::Ready => {
                panic!("could not get asset from storage but status is ready")
            }
        }
    }
}

struct ConvertRuntime<'a> {
    // to get assets
    storage: &'a mut AssetCacheStorage,
    // to request new loads if no cached value exist in storage
    loader: &'a mut AssetCacheLoad,
    // to get setting -> derived handle mapping
    registry: &'a mut AssetCacheRegistry,
}

impl<'a> ConvertRuntime<'a> {
    fn new(
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

struct ConvertState {
    // The dependency that caused the conversion to return waiting
    blocking_handle: Option<DynAssetHandle>,

    // If wait_for was a nested conversion that resulted in a new handle, store the request here
    conversion_request: Option<Box<dyn DynConvertRequest>>,

    // All dependencies accessed during the conversion
    dependencies: FxHashSet<DynAssetHandle>,
}

impl ConvertState {
    fn new() -> Self {
        Self {
            blocking_handle: None,
            conversion_request: None,
            dependencies: FxHashSet::default(),
        }
    }
}
