use crate::asset::{
    Asset, AssetConverter, AssetHandle, AssetHandleContext, AssetInserter, AssetLoader,
    DynAssetHandle,
};
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
    typed_insert_registries: FxHashMap<TypeId, Box<dyn DynInsertRegistry>>,

    // the status for handles
    // successfully loaded handles are removed
    status: FxHashMap<DynAssetHandle, LoadStatus>,
    // handles that became available this frame
    just_available: FxHashSet<DynAssetHandle>,

    asset_handle_ctx: AssetHandleContext,
}

impl AssetCacheRegistry {
    pub(crate) fn new(asset_handle_ctx: AssetHandleContext) -> Self {
        Self {
            typed_convert_registries: FxHashMap::default(),
            typed_load_registries: FxHashMap::default(),
            typed_insert_registries: FxHashMap::default(),
            just_available: FxHashSet::default(),
            status: FxHashMap::default(),
            asset_handle_ctx,
        }
    }

    fn get_or_create_typed_convert_registry_mut<T: AssetConverter + 'static>(
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

    fn get_or_create_typed_load_registry_mut<T: AssetLoader + 'static>(
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

    fn get_or_create_typed_insert_registry_mut<T: AssetInserter + 'static>(
        &mut self,
    ) -> &mut TypedInsertRegistry<T> {
        let entry = self
            .typed_insert_registries
            .entry(TypeId::of::<T>())
            .or_insert(Box::new(TypedInsertRegistry::<T>::new()));
        entry
            .as_any_mut()
            .downcast_mut::<TypedInsertRegistry<T>>()
            .expect("could not downcast typed storage cache")
    }

    pub(crate) fn create_empty_handle<T: Asset + 'static>(&self) -> AssetHandle<T> {
        AssetHandle::new(&self.asset_handle_ctx)
    }

    /// Gets an existing or creates a new handle
    ///
    /// Does not queue any conversions
    pub(crate) fn get_or_create_convert_handle<T: AssetConverter + 'static>(
        &mut self,
        settings: &T::Settings,
    ) -> AssetHandle<T::Asset> {
        let typed = self.get_or_create_typed_convert_registry_mut::<T>();
        if let Some(handle) = typed.settings_to_handle.get(settings) {
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
    pub(crate) fn get_or_create_load_handle<T: AssetLoader + 'static>(
        &mut self,
        settings: &T::Settings,
    ) -> AssetHandle<T::Asset> {
        let typed = self.get_or_create_typed_load_registry_mut::<T>();
        if let Some(handle) = typed.settings_to_handle.get(settings) {
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

    /// Gets an existing or creates a new handle
    ///
    /// Does not queue any conversions
    pub(crate) fn get_or_create_insert_handle<T: Asset, I: AssetInserter + 'static>(
        &mut self,
        key: &I::Key,
    ) -> AssetHandle<T> {
        let typed = self.get_or_create_typed_insert_registry_mut::<I>();
        if let Some(handle) = typed.key_to_handle.get(key) {
            let typed_handle = handle
                .to_typed()
                .expect("could not convert to typed handle");
            return typed_handle;
        }

        let new_handle = AssetHandle::<T>::new(&self.asset_handle_ctx);
        tracing::info!("create insert handle {}", new_handle);

        self.register_insert_handle_settings_mapping::<T, I>(new_handle.clone(), key.clone());

        new_handle
    }

    /// Note that this never returns cached handles it only creates new ones
    #[deprecated]
    pub(crate) fn crate_insert_handle<T: Asset + 'static>(&mut self) -> AssetHandle<T> {
        let new_handle = AssetHandle::<T>::new(&self.asset_handle_ctx);
        tracing::warn!(
            "creating insert handle {}, this is not cached in any way",
            new_handle
        );
        new_handle
    }

    /// Checks if a handle was created using a converter
    ///
    /// O(n) where n is types of converters
    pub(crate) fn created_by_converter(&mut self, dyn_handle: &DynAssetHandle) -> bool {
        for typed_convert in self.typed_convert_registries.values() {
            if typed_convert.contains_handle(dyn_handle) {
                return true;
            }
        }
        false
    }

    /// Checks if a handle was created using a loader
    ///
    /// O(n) where n is types of loaders
    pub(crate) fn created_by_loader(&mut self, dyn_handle: &DynAssetHandle) -> bool {
        for typed_load in self.typed_load_registries.values() {
            if typed_load.contains_handle(dyn_handle) {
                return true;
            }
        }
        false
    }

    //
    // Handle -> Settings
    // Settings -> Handle
    //

    pub(crate) fn register_convert_handle_settings_mapping<T: AssetConverter + 'static>(
        &mut self,
        handle: AssetHandle<T::Asset>,
        settings: T::Settings,
    ) {
        // tracing::info!("map convert settings -> handle {}", handle);
        let typed = self.get_or_create_typed_convert_registry_mut::<T>();
        typed
            .handle_to_settings
            .insert(handle.to_dyn(), settings.clone());
        typed.settings_to_handle.insert(settings, handle.to_dyn());
    }

    pub(crate) fn register_load_handle_settings_mapping<T: AssetLoader + 'static>(
        &mut self,
        handle: AssetHandle<T::Asset>,
        settings: T::Settings,
    ) {
        // tracing::info!("map load settings -> handle {}", handle);
        let typed = self.get_or_create_typed_load_registry_mut::<T>();
        typed
            .handle_to_settings
            .insert(handle.to_dyn(), settings.clone());
        typed.settings_to_handle.insert(settings, handle.to_dyn());
    }

    pub(crate) fn register_insert_handle_settings_mapping<T: Asset, I: AssetInserter + 'static>(
        &mut self,
        handle: AssetHandle<T>,
        key: I::Key,
    ) {
        // tracing::info!("map load settings -> handle {}", handle);
        let typed = self.get_or_create_typed_insert_registry_mut::<I>();
        typed.handle_to_key.insert(handle.to_dyn(), key.clone());
        typed.key_to_handle.insert(key, handle.to_dyn());
    }

    pub(crate) fn get_convert_settings_from_handle<T: AssetConverter + 'static>(
        &mut self,
        handle: &DynAssetHandle,
    ) -> Option<T::Settings> {
        self.get_or_create_typed_convert_registry_mut::<T>()
            .handle_to_settings
            .get(handle)
            .cloned()
    }

    pub(crate) fn get_load_settings_from_handle<T: AssetLoader + 'static>(
        &mut self,
        handle: &DynAssetHandle,
    ) -> Option<T::Settings> {
        self.get_or_create_typed_load_registry_mut::<T>()
            .handle_to_settings
            .get(handle)
            .cloned()
    }

    pub(crate) fn get_insert_settings_from_handle<T: Asset, I: AssetInserter + 'static>(
        &mut self,
        handle: &DynAssetHandle,
    ) -> Option<I::Key> {
        self.get_or_create_typed_insert_registry_mut::<I>()
            .handle_to_key
            .get(handle)
            .cloned()
    }

    //
    // Status
    //

    pub(crate) fn set_status(&mut self, handle: DynAssetHandle, status: LoadStatus) {
        self.status.insert(handle, status);
    }

    pub(crate) fn get_status(&mut self, handle: &DynAssetHandle) -> LoadStatus {
        self.status
            .entry(handle.clone())
            .or_insert(LoadStatus::NotRegistered)
            .clone()
    }

    //
    // Just available
    //

    pub(crate) fn set_just_available(&mut self, handle: DynAssetHandle) {
        self.just_available.insert(handle);
    }

    pub(crate) fn handle_just_available(&self, handle: &DynAssetHandle) -> bool {
        self.just_available.contains(handle)
    }

    pub(crate) fn clear_just_available(&mut self) {
        self.just_available.clear();
    }
}

//
// Typed/Dyn
//

// Convert

struct TypedConvertRegistry<T: AssetConverter> {
    handle_to_settings: FxHashMap<DynAssetHandle, T::Settings>,
    settings_to_handle: FxHashMap<T::Settings, DynAssetHandle>,
}

impl<T: AssetConverter + 'static> TypedConvertRegistry<T> {
    fn new() -> Self {
        Self {
            handle_to_settings: FxHashMap::default(),
            settings_to_handle: FxHashMap::default(),
        }
    }
}

trait DynConvertRegistry {
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn contains_handle(&self, handle: &DynAssetHandle) -> bool;
}

impl<T: AssetConverter + 'static> DynConvertRegistry for TypedConvertRegistry<T> {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self as &mut dyn Any
    }

    fn contains_handle(&self, handle: &DynAssetHandle) -> bool {
        self.handle_to_settings.contains_key(handle)
    }
}

// Load

struct TypedLoadRegistry<T: AssetLoader> {
    handle_to_settings: FxHashMap<DynAssetHandle, T::Settings>,
    settings_to_handle: FxHashMap<T::Settings, DynAssetHandle>,
}

impl<T: AssetLoader + 'static> TypedLoadRegistry<T> {
    fn new() -> Self {
        Self {
            handle_to_settings: FxHashMap::default(),
            settings_to_handle: FxHashMap::default(),
        }
    }
}

trait DynLoadRegistry {
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn contains_handle(&self, handle: &DynAssetHandle) -> bool;
}

impl<T: AssetLoader + 'static> DynLoadRegistry for TypedLoadRegistry<T> {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self as &mut dyn Any
    }

    fn contains_handle(&self, handle: &DynAssetHandle) -> bool {
        self.handle_to_settings.contains_key(handle)
    }
}

// Insert

struct TypedInsertRegistry<T: AssetInserter> {
    handle_to_key: FxHashMap<DynAssetHandle, T::Key>,
    key_to_handle: FxHashMap<T::Key, DynAssetHandle>,
}

impl<T: AssetInserter + 'static> TypedInsertRegistry<T> {
    fn new() -> Self {
        Self {
            handle_to_key: FxHashMap::default(),
            key_to_handle: FxHashMap::default(),
        }
    }
}

trait DynInsertRegistry {
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn contains_handle(&self, handle: &DynAssetHandle) -> bool;
}

impl<T: AssetInserter + 'static> DynInsertRegistry for TypedInsertRegistry<T> {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self as &mut dyn Any
    }

    fn contains_handle(&self, handle: &DynAssetHandle) -> bool {
        self.handle_to_key.contains_key(handle)
    }
}
