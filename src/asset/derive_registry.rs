use crate::asset::{AssetConverter, AssetHandleContext, DerivedHandle};
use rustc_hash::FxHashMap;
use std::any::{Any, TypeId};

//
// Genereic
//

pub struct AssetCacheDerivedRegistry {
    typed: FxHashMap<TypeId, Box<dyn DynDerivedRegistry>>,

    asset_handle_ctx: AssetHandleContext,
}
impl AssetCacheDerivedRegistry {
    pub fn new(asset_handle_ctx: AssetHandleContext) -> Self {
        Self {
            typed: FxHashMap::default(),
            asset_handle_ctx,
        }
    }

    /// Get typed cache assuming it exists
    pub fn get_typed_cache_ref<T: AssetConverter + 'static>(
        &self,
    ) -> Option<&TypedDerivedRegistry<T>> {
        self.typed.get(&TypeId::of::<T>()).map(|dyn_registry| {
            dyn_registry
                .as_any()
                .downcast_ref::<TypedDerivedRegistry<T>>()
                .expect("could not downcast typed storage cache")
        })
    }

    /// Get mutable typed cache or create if it doesnt exist
    pub fn get_typed_cache_mut<T: AssetConverter + 'static>(
        &mut self,
    ) -> &mut TypedDerivedRegistry<T> {
        let entry = self
            .typed
            .entry(TypeId::of::<T>())
            .or_insert(Box::new(TypedDerivedRegistry::<T>::new()));
        entry
            .as_any_mut()
            .downcast_mut::<TypedDerivedRegistry<T>>()
            .expect("could not downcast typed storage cache")
    }

    pub fn add_handle_setting_mapping<T: AssetConverter + 'static>(
        &mut self,
        handle: DerivedHandle<T::TargetAsset>,
        settings: T::Settings,
    ) {
        let typed = self.get_typed_cache_mut::<T>();
        typed
            .handle_to_settings
            .insert(handle.clone(), settings.clone());
        typed.settings_to_handle.insert(settings, handle);
    }

    /// Gets an existing or creates a new handle
    ///
    /// Does not queue any conversions
    pub fn get_or_create_handle<T: AssetConverter + 'static>(
        &mut self,
        settings: T::Settings,
    ) -> (DerivedHandle<T::TargetAsset>, bool) {
        let typed = self.get_typed_cache_mut::<T>();
        if let Some(handle) = typed.settings_to_handle.get(&settings) {
            return (handle.clone(), false);
        }

        let new_handle = DerivedHandle::<T::TargetAsset>::new(&self.asset_handle_ctx);

        self.add_handle_setting_mapping::<T>(new_handle.clone(), settings.clone());

        (new_handle, true)
    }
}

//
// Typed
//

pub struct TypedDerivedRegistry<T: AssetConverter> {
    pub handle_to_settings: FxHashMap<DerivedHandle<T::TargetAsset>, T::Settings>,
    pub settings_to_handle: FxHashMap<T::Settings, DerivedHandle<T::TargetAsset>>,
}

impl<T: AssetConverter + 'static> TypedDerivedRegistry<T> {
    pub fn new() -> Self {
        Self {
            handle_to_settings: FxHashMap::default(),
            settings_to_handle: FxHashMap::default(),
        }
    }

    pub fn add(&mut self, handle: DerivedHandle<T::TargetAsset>, settings: T::Settings) {}
}

//
// Dyn
//

pub trait DynDerivedRegistry {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T: AssetConverter + 'static> DynDerivedRegistry for TypedDerivedRegistry<T> {
    fn as_any(&self) -> &dyn Any {
        self as &dyn Any
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self as &mut dyn Any
    }
}
