use crate::asset::{AssetConverter, AssetHandle, AssetHandleContext, AssetLoader, DynAssetHandle};
use rustc_hash::{FxHashMap, FxHashSet};
use std::any::{Any, TypeId};

//
// Genereic
//

#[derive(Clone, Debug)]
pub enum LoadStatus {
    Loading,
    Failed,
    Ready,
    NotRegistered,
}

pub struct AssetCacheRegistry {
    typed_convert_registries: FxHashMap<TypeId, Box<dyn DynConvertRegistry>>,
    typed_load_registries: FxHashMap<TypeId, Box<dyn DynLoadRegistry>>,

    // the status for handles
    // successfully loaded handles are removed
    status: FxHashMap<DynAssetHandle, LoadStatus>,
    // handles that became available this frame
    just_available: FxHashSet<DynAssetHandle>,

    asset_handle_ctx: AssetHandleContext,
}

impl AssetCacheRegistry {
    pub fn new(asset_handle_ctx: AssetHandleContext) -> Self {
        Self {
            typed_convert_registries: FxHashMap::default(),
            typed_load_registries: FxHashMap::default(),
            just_available: FxHashSet::default(),
            status: FxHashMap::default(),
            asset_handle_ctx,
        }
    }

    /// Get typed cache assuming it exists
    pub fn get_typed_convert_registry_ref<T: AssetConverter + 'static>(
        &self,
    ) -> Option<&TypedConvertRegistry<T>> {
        self.typed_convert_registries
            .get(&TypeId::of::<T>())
            .map(|dyn_registry| {
                dyn_registry
                    .as_any()
                    .downcast_ref::<TypedConvertRegistry<T>>()
                    .expect("could not downcast typed storage cache")
            })
    }

    /// Get mutable typed cache or create if it doesnt exist
    pub fn get_typed_convert_registry_mut<T: AssetConverter + 'static>(
        &mut self,
    ) -> &mut TypedConvertRegistry<T> {
        let entry = self
            .typed_convert_registries
            .entry(TypeId::of::<T>())
            .or_insert(Box::new(TypedConvertRegistry::<T>::new()));
        entry
            .as_any_mut()
            .downcast_mut::<TypedConvertRegistry<T>>()
            .expect("could not downcast typed storage cache")
    }

    /// Get typed cache assuming it exists
    pub fn get_typed_load_registry_ref<T: AssetLoader + 'static>(
        &self,
    ) -> Option<&TypedLoadRegistry<T>> {
        self.typed_load_registries
            .get(&TypeId::of::<T>())
            .map(|dyn_registry| {
                dyn_registry
                    .as_any()
                    .downcast_ref::<TypedLoadRegistry<T>>()
                    .expect("could not downcast typed storage cache")
            })
    }

    /// Get mutable typed cache or create if it doesnt exist
    pub fn get_typed_load_registry_mut<T: AssetLoader + 'static>(
        &mut self,
    ) -> &mut TypedLoadRegistry<T> {
        let entry = self
            .typed_load_registries
            .entry(TypeId::of::<T>())
            .or_insert(Box::new(TypedLoadRegistry::<T>::new()));
        entry
            .as_any_mut()
            .downcast_mut::<TypedLoadRegistry<T>>()
            .expect("could not downcast typed storage cache")
    }

    // TODO: gets called twice
    pub fn register_convert_handle_settings_mapping<T: AssetConverter + 'static>(
        &mut self,
        handle: AssetHandle<T::Asset>,
        settings: T::Settings,
    ) {
        // tracing::info!("map convert settings -> handle {}", handle);
        let typed = self.get_typed_convert_registry_mut::<T>();
        typed
            .handle_to_settings
            .insert(handle.to_dyn(), settings.clone());
        typed.settings_to_handle.insert(settings, handle.to_dyn());

        // self.set_status(handle.to_dyn(), LoadStatus::Loading);
    }

    pub fn register_load_handle_settings_mapping<T: AssetLoader + 'static>(
        &mut self,
        handle: AssetHandle<T::Asset>,
        settings: T::Settings,
    ) {
        // tracing::info!("map load settings -> handle {}", handle);
        let typed = self.get_typed_load_registry_mut::<T>();
        typed
            .handle_to_settings
            .insert(handle.to_dyn(), settings.clone());
        typed.settings_to_handle.insert(settings, handle.to_dyn());

        // self.set_status(handle.to_dyn(), LoadStatus::Loading);
    }

    /// Gets an existing or creates a new handle
    ///
    /// Does not queue any conversions
    pub fn get_or_create_convert_handle<T: AssetConverter + 'static>(
        &mut self,
        settings: T::Settings,
    ) -> AssetHandle<T::Asset> {
        let typed = self.get_typed_convert_registry_mut::<T>();
        if let Some(handle) = typed.settings_to_handle.get(&settings) {
            let typed_handle = handle
                .to_typed()
                .expect("could not convert to typed handle");
            return typed_handle;
        }

        let new_handle = AssetHandle::<T::Asset>::new(&self.asset_handle_ctx);
        tracing::info!("create convert handle {}", new_handle);

        self.register_convert_handle_settings_mapping::<T>(new_handle.clone(), settings.clone());

        new_handle
    }

    /// Gets an existing or creates a new handle
    ///
    /// Does not queue any conversions
    pub fn get_or_create_load_handle<T: AssetLoader + 'static>(
        &mut self,
        settings: T::Settings, // TODO: make ref since its not always cloned
    ) -> AssetHandle<T::Asset> {
        let typed = self.get_typed_load_registry_mut::<T>();
        if let Some(handle) = typed.settings_to_handle.get(&settings) {
            let typed_handle = handle
                .to_typed()
                .expect("could not convert to typed handle");
            return typed_handle;
        }

        let new_handle = AssetHandle::<T::Asset>::new(&self.asset_handle_ctx);
        tracing::info!("create load handle {}", new_handle);

        self.register_load_handle_settings_mapping::<T>(new_handle.clone(), settings.clone());

        new_handle
    }

    //
    // Status
    //

    pub fn set_status(&mut self, handle: DynAssetHandle, status: LoadStatus) {
        // tracing::error!("set status {} to {:?}", handle, status);
        self.status.insert(handle, status);
    }

    pub fn get_status(&mut self, handle: &DynAssetHandle) -> LoadStatus {
        self.status
            .entry(handle.clone())
            .or_insert(LoadStatus::NotRegistered)
            .clone()
    }

    // pub fn remove_status(&mut self, handle: &DynAssetHandle) {
    //     self.status.remove(handle);
    // }

    // Just available

    pub fn set_just_available(&mut self, handle: DynAssetHandle) {
        self.just_available.insert(handle);
    }

    pub fn handle_just_loaded(&self, handle: &DynAssetHandle) -> bool {
        self.just_available.contains(handle)
    }

    pub fn clear_just_available(&mut self) {
        self.just_available.clear();
    }
}

//
// Typed
//

pub struct TypedConvertRegistry<T: AssetConverter> {
    // TODO: remove pub
    pub handle_to_settings: FxHashMap<DynAssetHandle, T::Settings>,
    settings_to_handle: FxHashMap<T::Settings, DynAssetHandle>,
}

impl<T: AssetConverter + 'static> TypedConvertRegistry<T> {
    pub fn new() -> Self {
        Self {
            handle_to_settings: FxHashMap::default(),
            settings_to_handle: FxHashMap::default(),
        }
    }
}

pub struct TypedLoadRegistry<T: AssetLoader> {
    pub handle_to_settings: FxHashMap<DynAssetHandle, T::Settings>,
    settings_to_handle: FxHashMap<T::Settings, DynAssetHandle>,
}

impl<T: AssetLoader + 'static> TypedLoadRegistry<T> {
    pub fn new() -> Self {
        Self {
            handle_to_settings: FxHashMap::default(),
            settings_to_handle: FxHashMap::default(),
        }
    }
}

//
// Dyn
//

pub trait DynConvertRegistry {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T: AssetConverter + 'static> DynConvertRegistry for TypedConvertRegistry<T> {
    fn as_any(&self) -> &dyn Any {
        self as &dyn Any
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self as &mut dyn Any
    }
}

pub trait DynLoadRegistry {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn contains_handle(&self, handle: &DynAssetHandle) -> bool;
}

impl<T: AssetLoader + 'static> DynLoadRegistry for TypedLoadRegistry<T> {
    fn as_any(&self) -> &dyn Any {
        self as &dyn Any
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self as &mut dyn Any
    }

    fn contains_handle(&self, handle: &DynAssetHandle) -> bool {
        self.handle_to_settings.contains_key(handle)
    }
}
