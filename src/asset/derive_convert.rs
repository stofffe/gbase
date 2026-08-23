use crate::{
    asset::{
        AssetCacheDerivedRegistry, AssetCacheDerivedStorage, AssetCacheLoad, AssetCacheStorage,
        AssetConverter, AssetHandleContext, ConvertAssetStatus, ConvertContext, ConvertRuntime,
        DerivedHandle, DynAssetHandle, DynDependency, DynDerivedHandle,
    },
    Context,
};
use rustc_hash::{FxHashMap, FxHashSet};
use std::{
    any::{Any, TypeId},
    collections::VecDeque,
    marker::PhantomData,
};

//
// Generic
//

pub struct AssetCacheDerivedConvert {
    asset_handle_ctx: AssetHandleContext,

    typed: FxHashMap<TypeId, Box<dyn DynDerivedConvert>>,

    // queue
    queue: VecDeque<DynDerivedHandle>,
    queued: FxHashSet<DynDerivedHandle>,

    // waiting
    waiting_for: FxHashMap<DynDependency, FxHashSet<DynDerivedHandle>>,

    handle_to_converter_type: FxHashMap<DynDerivedHandle, TypeId>,
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
        derived_storage: &mut AssetCacheDerivedStorage,
        derived_registry: &mut AssetCacheDerivedRegistry,
    ) {
        // if !self.waiting_for.is_empty() {
        //     for (handle, waiting) in self.waiting_for.iter() {
        //         for wait in waiting {
        //             tracing::info!("{} wait for {}", wait, handle);
        //         }
        //     }
        // }

        while let Some(dyn_derived_handle) = self.queue.pop_front() {
            self.queued.remove(&dyn_derived_handle);

            let Some(type_id) = self.handle_to_converter_type.get(&dyn_derived_handle) else {
                panic!("no converter registered for {}", dyn_derived_handle.id());
            };

            let Some(typed_converter) = self.typed.get_mut(type_id) else {
                panic!("could not get typed converter");
            };

            match typed_converter.convert(
                ctx,
                storage,
                loader,
                derived_storage,
                derived_registry,
                dyn_derived_handle.clone(),
            ) {
                ConversionPollResult::Waiting(dyn_dependency) => {
                    tracing::info!(
                        "conversion {} waiting for asset handle {}",
                        dyn_derived_handle,
                        dyn_dependency
                    );

                    // TODO: feel like i need to
                    // remove any pending handles
                    self.waiting_for.retain(|_, handles| {
                        handles.remove(&dyn_derived_handle);
                        !handles.is_empty()
                    });

                    // add waiting handle
                    self.waiting_for
                        .entry(dyn_dependency)
                        .or_default()
                        .insert(dyn_derived_handle);
                }
                ConversionPollResult::Failed => {
                    // remove any pending handles
                    self.waiting_for.retain(|_, handles| {
                        handles.remove(&dyn_derived_handle);
                        !handles.is_empty()
                    });
                    tracing::info!("conversion failed {}", dyn_derived_handle.id());
                }
                ConversionPollResult::Success => {
                    // remove any pending handles
                    self.waiting_for.retain(|_, handles| {
                        handles.remove(&dyn_derived_handle);
                        !handles.is_empty()
                    });
                    tracing::info!("conversion success {}", dyn_derived_handle);
                }
            }
        }
    }

    // setup conversion state
    pub fn register_conversion<T: AssetConverter + 'static>(
        &mut self,
        derived_registry: &mut AssetCacheDerivedRegistry,
        settings: T::Settings,
    ) -> DerivedHandle<T::TargetAsset> {
        // use cached value if it exists
        if let Some(handle) = derived_registry
            .get_typed_cache_mut::<T>()
            .settings_to_handle
            .get(&settings)
        {
            return handle.clone();
        };

        let derived_handle = DerivedHandle::new(&self.asset_handle_ctx);

        // register converter
        // TODO: kinda weird since it does nothing
        self.get_typed_cache_mut::<T>();

        // registry
        derived_registry.add_handle_setting_mapping::<T>(derived_handle.clone(), settings.clone());

        self.handle_to_converter_type
            .insert(derived_handle.to_dyn(), TypeId::of::<T>());

        self.queue_conversion(derived_handle.to_dyn());

        derived_handle
    }

    // Queue a conversion of a derived handle
    //
    // Assumes all state is set up from queue_conversion function
    pub fn queue_conversion(&mut self, handle: DynDerivedHandle) {
        if self.queued.insert(handle.clone()) {
            self.queue.push_back(handle);
        }
    }

    /// Wake up all derived assets waiting for this handle to be ready
    pub fn wakeup_waiting_on_handle(&mut self, handle: &DynAssetHandle) {
        tracing::info!(
            "wake all waiting on {}: #{}",
            handle,
            self.waiting_for
                .get(&DynDependency::Asset(handle.clone()))
                .map_or(0, |a| a.len())
        );
        let Some(waiting_handles) = self
            .waiting_for
            .remove(&DynDependency::Asset(handle.clone()))
        else {
            return;
        };

        for derived_handle in waiting_handles {
            tracing::info!("-> wake up {}", derived_handle);
            self.queue_conversion(derived_handle);
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
    Waiting(DynDependency),
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
        derived_storage: &mut AssetCacheDerivedStorage,
        derived_registry: &mut AssetCacheDerivedRegistry,
        dyn_handle: DynDerivedHandle,
    ) -> ConversionPollResult;
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T: AssetConverter + 'static> DynDerivedConvert for TypedDerivedConvert<T> {
    fn convert(
        &mut self,
        ctx: &mut Context,
        storage: &mut AssetCacheStorage,
        loader: &mut AssetCacheLoad,
        derived_storage: &mut AssetCacheDerivedStorage,
        derived_registry: &mut AssetCacheDerivedRegistry,
        dyn_handle: DynDerivedHandle,
    ) -> ConversionPollResult {
        // TODO: maybe just store the dyn directly?
        let handle = dyn_handle
            .to_typed::<T::TargetAsset>()
            .expect("could not convert dyn handle to typed");

        let Some(settings) = derived_registry
            .get_typed_cache_mut::<T>()
            .handle_to_settings
            .get(&handle)
            .cloned()
        else {
            panic!("could not get settings from handle");
        };

        let mut runtime = ConvertRuntime::new(storage, loader, derived_storage, derived_registry); // TODO: this is self refeentaial
        let mut convert_ctx = ConvertContext::new(&mut runtime);

        let conversion = T::convert(ctx, &mut convert_ctx, &settings);
        let state = convert_ctx.state.clone();

        match conversion {
            ConvertAssetStatus::Loading => {
                let blocking_dependency = &state
                    .blocking_dependency
                    .expect("should have blocking dependency if in loading status");
                ConversionPollResult::Waiting(blocking_dependency.clone())
            }
            ConvertAssetStatus::Success(derived_asset) => {
                // insert into typed storage
                derived_storage.insert::<T::TargetAsset>(ctx, handle, derived_asset);

                ConversionPollResult::Success
            }
            ConvertAssetStatus::Failed => ConversionPollResult::Failed,
        }
    }

    fn as_any(&self) -> &dyn Any {
        self as &dyn Any
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self as &mut dyn Any
    }
}
