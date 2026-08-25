use crate::{
    asset::{
        Asset, AssetCacheDependency, AssetCacheDerivedStorage, AssetCacheLoad, AssetCacheRegistry,
        AssetCacheStorage, AssetHandle, AssetHandleContext, DynAssetHandle, GetAssetResult,
        LoadStatus,
    },
    render::ArcHandle,
    Context,
};
use core::error;
use rustc_hash::{FxHashMap, FxHashSet};
use std::{
    any::{Any, TypeId},
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
    fn request(
        self: Box<Self>,
        derived_convert: &mut AssetCacheDerivedConvert,
        derived_registry: &mut AssetCacheRegistry,
    );
}

impl<T: AssetConverter + 'static> DynConvertRequest for ConvertRequest<T> {
    fn request(
        self: Box<Self>,
        derived_convert: &mut AssetCacheDerivedConvert,
        derived_registry: &mut AssetCacheRegistry,
    ) {
        // register converter
        // TODO: kinda weird since it does nothing
        derived_convert.get_typed_cache_mut::<T>();

        // registry
        derived_registry.register_convert_handle::<T>(self.handle.clone(), self.settings.clone());

        derived_convert
            .handle_to_converter_type
            .insert(self.handle.to_dyn(), TypeId::of::<T>());

        derived_convert.queue_conversion(self.handle.to_dyn());
    }

    fn handle(&self) -> DynAssetHandle {
        self.handle.to_dyn()
    }
}

//
// Generic
//

pub struct AssetCacheDerivedConvert {
    asset_handle_ctx: AssetHandleContext,

    typed: FxHashMap<TypeId, Box<dyn DynDerivedConvert>>,

    // queue
    queue: VecDeque<DynAssetHandle>,
    queued: FxHashSet<DynAssetHandle>,

    // waiting
    waiting_for: FxHashMap<DynAssetHandle, FxHashSet<DynAssetHandle>>,

    handle_to_converter_type: FxHashMap<DynAssetHandle, TypeId>,
}

impl AssetCacheDerivedConvert {
    pub fn new(asset_handle_ctx: AssetHandleContext) -> Self {
        Self {
            asset_handle_ctx,
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
    ) -> Option<&TypedDerivedConvert<T>> {
        self.typed.get(&TypeId::of::<T>()).map(|dyn_convert| {
            dyn_convert
                .as_any()
                .downcast_ref::<TypedDerivedConvert<T>>()
                .expect("could not downcast typed storage cache")
        })
    }

    /// Get mutable typed cache or create if it doesnt exist
    pub fn get_typed_cache_mut<T: AssetConverter + 'static>(
        &mut self,
    ) -> &mut TypedDerivedConvert<T> {
        let entry = self
            .typed
            .entry(TypeId::of::<T>())
            .or_insert(Box::new(TypedDerivedConvert::<T>::new()));
        entry
            .as_any_mut()
            .downcast_mut::<TypedDerivedConvert<T>>()
            .expect("could not downcast typed storage cache")
    }

    pub fn poll_conversions(
        &mut self,
        ctx: &mut Context,
        storage: &mut AssetCacheStorage,
        loader: &mut AssetCacheLoad,
        dependency: &mut AssetCacheDependency,
        derived_storage: &mut AssetCacheDerivedStorage,
        derived_registry: &mut AssetCacheRegistry,
    ) {
        while let Some(dyn_handle) = self.queue.pop_front() {
            self.queued.remove(&dyn_handle);

            let Some(type_id) = self.handle_to_converter_type.get(&dyn_handle) else {
                tracing::warn!("no convertere registered for {}", dyn_handle);
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
                derived_storage,
                derived_registry,
                dyn_handle.clone(),
            );
            tracing::info!("-- done polled conversion {}", dyn_handle);

            // request conversions
            if let Some(request) = request {
                request.request(self, derived_registry);
            }

            match result {
                ConversionPollResult::Waiting(dyn_dependency) => {
                    tracing::info!(
                        "conversion {} waiting for asset handle {}",
                        dyn_handle,
                        dyn_dependency
                    );

                    // TODO: feel like i need to
                    // remove any pending handles
                    self.waiting_for.retain(|_, handles| {
                        handles.remove(&dyn_handle);
                        !handles.is_empty()
                    });

                    // add waiting handle
                    self.waiting_for
                        .entry(dyn_dependency)
                        .or_default()
                        .insert(dyn_handle);
                }
                ConversionPollResult::Failed => {
                    // remove any pending handles
                    self.waiting_for.retain(|_, handles| {
                        handles.remove(&dyn_handle);
                        !handles.is_empty()
                    });
                    tracing::info!("conversion failed {}", dyn_handle.id());
                }
                ConversionPollResult::Success => {
                    // remove any pending handles
                    self.waiting_for.retain(|_, handles| {
                        handles.remove(&dyn_handle);
                        !handles.is_empty()
                    });
                    // tracing::info!("conversion success {}", dyn_derived_handle);
                    // TODO: make available
                    self.wakeup_waiting_on_handle(&dyn_handle);
                    self.requeu_dependents(dependency, &dyn_handle);
                }
            }
        }
    }

    // setup conversion state
    pub fn register_conversion<T: AssetConverter + 'static>(
        &mut self,
        derived_registry: &mut AssetCacheRegistry,
        settings: T::Settings,
    ) -> AssetHandle<T::Asset> {
        // use cached value if it exists
        if let Some(handle) = derived_registry
            .get_typed_convert_registry_mut::<T>()
            .settings_to_handle
            .get(&settings)
        {
            return handle.clone();
        };

        let derived_handle = AssetHandle::new(&self.asset_handle_ctx);
        tracing::info!("create new derived handle {}", derived_handle);

        // register converter
        // TODO: kinda weird since it does nothing
        self.get_typed_cache_mut::<T>();

        // registry
        derived_registry.register_convert_handle::<T>(derived_handle.clone(), settings.clone());

        self.handle_to_converter_type
            .insert(derived_handle.to_dyn(), TypeId::of::<T>());

        self.queue_conversion(derived_handle.to_dyn());

        derived_handle
    }

    // Queue a conversion of a derived handle
    //
    // Assumes all state is set up from queue_conversion function
    pub fn queue_conversion(&mut self, handle: DynAssetHandle) {
        // TODO: this must check if its convertable?
        if self.queued.insert(handle.clone()) {
            self.queue.push_back(handle);
        }
    }

    /// Wake up all derived assets waiting for this handle to be ready
    pub fn wakeup_waiting_on_handle(&mut self, dependency: &DynAssetHandle) {
        tracing::info!(
            "wake all waiting on {}: #{}",
            dependency,
            self.waiting_for.get(dependency).map_or(0, |a| a.len())
        );
        let Some(waiting_handles) = self.waiting_for.remove(dependency) else {
            return;
        };

        for derived_handle in waiting_handles {
            tracing::info!("-> wake up {}", derived_handle);
            self.queue_conversion(derived_handle);
        }
    }

    pub fn requeu_dependents(
        &mut self,
        dependency: &mut AssetCacheDependency,
        handle: &DynAssetHandle,
    ) {
        if let Some(dependents) = dependency.dependents(handle) {
            tracing::info!(
                "requeue dependents due to {}, len {}",
                handle,
                dependents.len()
            );
            for dependent in dependents.iter() {
                self.queue_conversion(dependent.clone());
            }
        }
    }
}

//
// Typed
//

pub struct TypedDerivedConvert<T: AssetConverter> {
    ty: PhantomData<T>,
}

impl<T: AssetConverter + 'static> TypedDerivedConvert<T> {
    pub fn new() -> Self {
        Self { ty: PhantomData }
    }
}

pub enum ConversionPollResult {
    Waiting(DynAssetHandle),
    Failed,
    Success,
}

//
// Dyn
//

pub trait DynDerivedConvert {
    fn convert(
        &mut self,
        ctx: &mut Context,
        storage: &mut AssetCacheStorage,
        loader: &mut AssetCacheLoad,
        dependency: &mut AssetCacheDependency,
        derived_storage: &mut AssetCacheDerivedStorage,
        derived_registry: &mut AssetCacheRegistry,
        dyn_handle: DynAssetHandle,
    ) -> (ConversionPollResult, Option<Box<dyn DynConvertRequest>>);
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T: AssetConverter + 'static> DynDerivedConvert for TypedDerivedConvert<T> {
    fn convert(
        &mut self,
        ctx: &mut Context,
        storage: &mut AssetCacheStorage,
        loader: &mut AssetCacheLoad,
        dependency: &mut AssetCacheDependency,
        derived_storage: &mut AssetCacheDerivedStorage,
        derived_registry: &mut AssetCacheRegistry,
        dyn_handle: DynAssetHandle,
    ) -> (ConversionPollResult, Option<Box<dyn DynConvertRequest>>) {
        // TODO: maybe just store the dyn directly?
        let derived_handle = dyn_handle
            .to_typed::<T::Asset>()
            .expect("could not convert dyn handle to typed");

        let Some(settings) = derived_registry
            .get_typed_convert_registry_mut::<T>()
            .handle_to_settings
            .get(&derived_handle)
            .cloned()
        else {
            panic!("could not get settings from handle");
        };

        let mut runtime = ConvertRuntime::new(storage, loader, derived_storage, derived_registry);
        let mut convert_ctx = ConvertContext::new(&mut runtime);

        let conversion = T::convert(ctx, &mut convert_ctx, &settings);

        let wait_for = convert_ctx.state.wait_for.clone();
        let dependencies = convert_ctx.state.dependencies.clone();

        match conversion {
            ConvertAssetStatus::Loading => (
                ConversionPollResult::Waiting(
                    wait_for.expect("should have blocking dependency if in loading status"),
                ),
                convert_ctx.state.conversion_request,
            ),
            ConvertAssetStatus::Success(derived_asset) => {
                // insert into typed storage
                derived_storage.insert::<T::Asset>(ctx, derived_handle.clone(), derived_asset);

                // register deps
                dependency.register_dependencies(&derived_handle.to_dyn(), &dependencies);

                (ConversionPollResult::Success, None)
            }
            ConvertAssetStatus::Failed => (ConversionPollResult::Failed, None),
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
    // to get asset status
    pub(crate) loader: &'a mut AssetCacheLoad,
    // to get derived assets
    pub(crate) derived_storage: &'a mut AssetCacheDerivedStorage,
    // to get setting -> derived handle mapping
    pub(crate) registry: &'a mut AssetCacheRegistry,
}

impl<'a> ConvertRuntime<'a> {
    pub fn new(
        storage: &'a mut AssetCacheStorage,
        loader: &'a mut AssetCacheLoad,
        derived_storage: &'a mut AssetCacheDerivedStorage,
        registry: &'a mut AssetCacheRegistry,
    ) -> Self {
        Self {
            storage,
            loader,
            derived_storage,
            registry,
        }
    }
}

pub struct ConvertState {
    // The dependency that caused the conversion to return waiting
    pub wait_for: Option<DynAssetHandle>,

    // If wait_for was a nested conversion that resulted in a new handle, store the request here
    pub conversion_request: Option<Box<dyn DynConvertRequest>>,

    // All dependencies accessed during the conversion
    pub dependencies: FxHashSet<DynAssetHandle>,
}

impl ConvertState {
    pub fn new() -> Self {
        Self {
            wait_for: None,
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

    pub fn get_load_asset<T: Asset>(&mut self, handle: &AssetHandle<T>) -> GetAssetResult<'_, T> {
        // register deps
        self.state.dependencies.insert(handle.to_dyn());

        if let Some(asset) = self.runtime.storage.get(handle) {
            return GetAssetResult::Success(asset);
        }

        match self.runtime.loader.get_status(&handle.to_dyn()) {
            LoadStatus::Loading => {
                self.state.wait_for = Some(handle.to_dyn());
                tracing::info!("{} is not ready, set blocking", handle);
                GetAssetResult::Loading
            }
            LoadStatus::Failed => GetAssetResult::Error,
            LoadStatus::Loaded => {
                panic!("could not get asset from storage but its marked as loaded")
            }
        }
    }

    pub fn get_nested_convert<G: AssetConverter + 'static>(
        &mut self,
        ctx: &mut Context,
        settings: &G::Settings,
    ) -> ConvertAssetResult<G::Asset> {
        // get handle from registry
        let (handle, created_new) = self
            .runtime
            .registry
            .get_or_create_convert_handle::<G>(settings.clone());

        // register deps
        self.state.dependencies.insert(handle.to_dyn());

        self.state.wait_for = Some(handle.to_dyn());
        if created_new {
            self.state.conversion_request = Some(Box::new(ConvertRequest::<G>::new(
                handle.clone(),
                settings.clone(),
            )));
        }

        match self.runtime.derived_storage.get(&handle) {
            Some(asset) => ConvertAssetResult::Success(asset),
            None => {
                self.state.wait_for = Some(handle.to_dyn());
                ConvertAssetResult::Loading
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
