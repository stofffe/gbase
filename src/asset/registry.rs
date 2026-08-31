use crate::asset::{
    Asset, AssetConverter, AssetHandle, AssetHandleContext, AssetInserter, AssetLoader,
    DynAssetHandle, ScopedInsertAssetKey,
};
use rustc_hash::{FxHashMap, FxHashSet};
use std::{
    any::{Any, TypeId},
    fmt::Debug,
    marker::PhantomData,
};

//
// Metadata
//

pub struct AssetMetadata {
    status: InternalAssetState,
    debug_name: Option<String>,
}

impl AssetMetadata {
    fn new() -> Self {
        Self {
            status: InternalAssetState::NotRegistered,
            debug_name: None,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) enum InternalAssetState {
    Loading,
    Failed,
    Ready,
    NotRegistered,
}

/// Only returned when an asset is not found
#[derive(Clone, Debug)]
pub enum GetAssetState {
    Loading,
    Failed,
}

//
// Genereic
//

pub struct AssetCacheRegistry {
    asset_handle_ctx: AssetHandleContext,

    typed_convert_registries: FxHashMap<TypeId, Box<dyn DynConvertRegistry>>,
    typed_load_registries: FxHashMap<TypeId, Box<dyn DynLoadRegistry>>,
    typed_insert_registries: FxHashMap<TypeId, Box<dyn DynInsertRegistry>>,

    metadata: FxHashMap<DynAssetHandle, AssetMetadata>,
    // handles that became available this frame
    just_available: FxHashSet<DynAssetHandle>,
}

impl AssetCacheRegistry {
    pub(crate) fn new(asset_handle_ctx: AssetHandleContext) -> Self {
        Self {
            typed_convert_registries: FxHashMap::default(),
            typed_load_registries: FxHashMap::default(),
            typed_insert_registries: FxHashMap::default(),
            just_available: FxHashSet::default(),
            metadata: FxHashMap::default(),
            asset_handle_ctx,
        }
    }

    fn get_or_create_typed_convert_registry_mut<T: AssetConverter + 'static>(
        &mut self,
    ) -> &mut TypedConvertRegistry<T> {
        let entry = self
            .typed_convert_registries
            .entry(TypeId::of::<T>())
            .or_insert_with(|| Box::new(TypedConvertRegistry::<T>::new()));
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
            .or_insert_with(|| Box::new(TypedLoadRegistry::<T>::new()));
        entry
            .as_any_mut()
            .downcast_mut::<TypedLoadRegistry<T>>()
            .expect("could not downcast typed storage cache")
    }

    fn get_or_create_typed_insert_registry_mut<T: Asset + 'static, I: AssetInserter + 'static>(
        &mut self,
    ) -> &mut TypedInsertRegistry<T, I> {
        let entry = self
            .typed_insert_registries
            .entry(TypeId::of::<T>())
            .or_insert_with(|| Box::new(TypedInsertRegistry::<T, I>::new()));
        entry
            .as_any_mut()
            .downcast_mut::<TypedInsertRegistry<T, I>>()
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

        let typed = self.get_or_create_typed_convert_registry_mut::<T>();
        typed
            .handle_to_settings
            .insert(new_handle.to_dyn(), settings.clone());
        typed
            .settings_to_handle
            .insert(settings.clone(), new_handle.to_dyn());

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

        let typed = self.get_or_create_typed_load_registry_mut::<T>();
        typed
            .handle_to_settings
            .insert(new_handle.to_dyn(), settings.clone());
        typed
            .settings_to_handle
            .insert(settings.clone(), new_handle.to_dyn());

        new_handle
    }

    /// Gets an existing or creates a new handle
    ///
    /// Does not queue any conversions
    pub(crate) fn get_or_create_insert_handle<T: Asset + 'static, I: AssetInserter + 'static>(
        &mut self,
        key: I::Key,
        scope: Option<DynAssetHandle>,
    ) -> AssetHandle<T> {
        let insert_asset_key = ScopedInsertAssetKey::new(key, scope);

        let typed = self.get_or_create_typed_insert_registry_mut::<T, I>();
        if let Some(handle) = typed.key_to_handle.get(&insert_asset_key) {
            let typed_handle = handle
                .to_typed()
                .expect("could not convert to typed handle");
            return typed_handle;
        }

        let new_handle = AssetHandle::<T>::new(&self.asset_handle_ctx);
        tracing::info!("create insert handle {}", new_handle);

        let typed = self.get_or_create_typed_insert_registry_mut::<T, I>();
        typed
            .handle_to_key
            .insert(new_handle.to_dyn(), insert_asset_key.clone());
        typed
            .key_to_handle
            .insert(insert_asset_key.clone(), new_handle.to_dyn());

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

    /// Checks if a handle was created using a inserter
    ///
    /// O(n) where n is types of loaders
    pub(crate) fn created_by_inserter(&mut self, dyn_handle: &DynAssetHandle) -> bool {
        for typed_insert in self.typed_insert_registries.values() {
            if typed_insert.contains_handle(dyn_handle) {
                return true;
            }
        }
        false
    }

    //
    // Get settings/keys
    //

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
        self.get_or_create_typed_insert_registry_mut::<T, I>()
            .handle_to_key
            .get(handle)
            .map(|insert_asset_key| insert_asset_key.key().clone())
    }

    //
    // Metadata
    //

    fn get_metadata_mut(&mut self, handle: DynAssetHandle) -> &mut AssetMetadata {
        self.metadata.entry(handle).or_insert(AssetMetadata::new())
    }

    pub(crate) fn set_status(&mut self, handle: DynAssetHandle, status: InternalAssetState) {
        let metadata = self.get_metadata_mut(handle);
        metadata.status = status;
    }

    pub(crate) fn get_status(&mut self, handle: DynAssetHandle) -> InternalAssetState {
        let metadata = self.get_metadata_mut(handle);
        metadata.status.clone()
    }

    pub(crate) fn set_debug_name(&mut self, handle: DynAssetHandle, debug_name: String) {
        let metadata = self.get_metadata_mut(handle);
        metadata.debug_name = Some(debug_name);
    }

    pub(crate) fn get_debug_name(&mut self, handle: DynAssetHandle) -> Option<&str> {
        let metadata = self.get_metadata_mut(handle);
        metadata.debug_name.as_deref()
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

struct TypedInsertRegistry<T: Asset, I: AssetInserter> {
    handle_to_key: FxHashMap<DynAssetHandle, ScopedInsertAssetKey<I>>,
    key_to_handle: FxHashMap<ScopedInsertAssetKey<I>, DynAssetHandle>,

    inserter_type: PhantomData<T>,
}

impl<T: Asset + 'static, I: AssetInserter + 'static> TypedInsertRegistry<T, I> {
    fn new() -> Self {
        Self {
            handle_to_key: FxHashMap::default(),
            key_to_handle: FxHashMap::default(),
            inserter_type: PhantomData,
        }
    }
}

trait DynInsertRegistry {
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn contains_handle(&self, handle: &DynAssetHandle) -> bool;
}

impl<T: Asset + 'static, I: AssetInserter + 'static> DynInsertRegistry
    for TypedInsertRegistry<T, I>
{
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self as &mut dyn Any
    }

    fn contains_handle(&self, handle: &DynAssetHandle) -> bool {
        self.handle_to_key.contains_key(handle)
    }
}
