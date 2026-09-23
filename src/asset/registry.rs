use crate::{
    asset::{
        Asset, AssetCacheStorage, AssetHandle, AssetInserter, AssetLoader, DynAssetHandle,
        ScopedInsertAssetKey,
    },
    ConditionalSend,
};
use rustc_hash::FxHashMap;
use std::{
    any::{Any, TypeId},
    marker::PhantomData,
    sync::{Arc, Mutex},
};

//
// Genereic
//

#[derive(Clone)]
pub struct ArcAssetCacheRegistry {
    registry: Arc<Mutex<AssetCacheRegistry>>,
}

impl ArcAssetCacheRegistry {
    pub(crate) fn new() -> Self {
        let registry = AssetCacheRegistry::new();
        Self {
            registry: Arc::new(Mutex::new(registry)),
        }
    }

    pub fn get_empty_handle<T: Asset>(&self) -> AssetHandle<T> {
        self.registry
            .lock()
            .expect("could not lock registry mutex")
            .create_empty_handle()
    }

    // TODO: all usage of this must also register in storage?
    pub fn get_load_handle<T: AssetLoader + 'static>(
        &self,
        settings: &T::Settings,
    ) -> AssetHandle<T::Asset> {
        self.registry
            .lock()
            .expect("could not lock registry mutex")
            .get_or_create_load_handle::<T>(settings)
    }

    pub fn get_load_settings<T: AssetLoader + 'static>(
        &self,
        handle: &AssetHandle<T::Asset>,
    ) -> Option<T::Settings> {
        self.registry
            .lock()
            .expect("could not lock registry mutex")
            .get_load_settings_from_handle::<T>(&handle.to_dyn())
    }

    // TODO: all usage of this must also register in storage?
    pub fn get_insert_handle<T: Asset, I: AssetInserter + 'static>(
        &self,
        key: I::Key,
        scope: Option<DynAssetHandle>,
    ) -> AssetHandle<T> {
        self.registry
            .lock()
            .expect("could not lock registry mutex")
            .get_or_create_insert_handle::<T, I>(key, scope)
    }
}

pub struct AssetCacheRegistry {
    id: u64,

    typed_load_registries: FxHashMap<TypeId, Box<dyn DynLoadRegistry>>,
    typed_insert_registries: FxHashMap<TypeId, Box<dyn DynInsertRegistry>>,
}

impl AssetCacheRegistry {
    pub(crate) fn new() -> Self {
        Self {
            id: 0,
            typed_load_registries: FxHashMap::default(),
            typed_insert_registries: FxHashMap::default(),
        }
    }

    fn next_id(&mut self) -> u64 {
        let id = self.id;
        self.id += 1;
        id
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

    pub(crate) fn create_empty_handle<T: Asset + 'static>(&mut self) -> AssetHandle<T> {
        tracing::info!("create new empty handle");
        AssetHandle::new(self.next_id())
    }

    /// Gets an existing or creates a new handle
    ///
    /// Does not queue any conversions
    pub(crate) fn get_or_create_load_handle<T: AssetLoader + 'static>(
        &mut self,
        // storage: &mut AssetCacheStorage,
        settings: &T::Settings,
    ) -> AssetHandle<T::Asset> {
        let typed = self.get_or_create_typed_load_registry_mut::<T>();
        if let Some(handle) = typed.settings_to_handle.get(settings) {
            let typed_handle = handle
                .to_typed()
                .expect("could not convert to typed handle");
            return typed_handle;
        }

        let new_handle = AssetHandle::<T::Asset>::new(self.next_id());
        tracing::info!("create load handle {}", new_handle);

        let typed = self.get_or_create_typed_load_registry_mut::<T>();
        typed
            .handle_to_settings
            .insert(new_handle.to_dyn(), settings.clone());
        typed
            .settings_to_handle
            .insert(settings.clone(), new_handle.to_dyn());

        // create empty asset entry in storage
        // TODO:
        // storage.register_asset(new_handle.clone());

        new_handle
    }

    /// Gets an existing or creates a new handle
    ///
    /// Does not queue any conversions
    pub(crate) fn get_or_create_insert_handle<T: Asset + 'static, I: AssetInserter + 'static>(
        &mut self,
        // storage: &mut AssetCacheStorage,
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

        let new_handle = AssetHandle::<T>::new(self.next_id());
        tracing::info!("create insert handle {}", new_handle);

        let typed = self.get_or_create_typed_insert_registry_mut::<T, I>();
        typed
            .handle_to_key
            .insert(new_handle.to_dyn(), insert_asset_key.clone());
        typed
            .key_to_handle
            .insert(insert_asset_key.clone(), new_handle.to_dyn());

        // create empty asset entry in storage
        // TODO:
        // storage.register_asset(new_handle.clone());

        new_handle
    }

    //
    // Get settings/keys
    //

    pub(crate) fn get_load_settings_from_handle<T: AssetLoader + 'static>(
        &mut self,
        handle: &DynAssetHandle,
    ) -> Option<T::Settings> {
        self.get_or_create_typed_load_registry_mut::<T>()
            .handle_to_settings
            .get(handle)
            .cloned()
    }

    pub(crate) fn get_insert_key_from_handle<T: Asset, I: AssetInserter + 'static>(
        &mut self,
        handle: &DynAssetHandle,
    ) -> Option<I::Key> {
        self.get_or_create_typed_insert_registry_mut::<T, I>()
            .handle_to_key
            .get(handle)
            .map(|insert_asset_key| insert_asset_key.key().clone())
    }
}

//
// Typed/Dyn
//

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

trait DynLoadRegistry: ConditionalSend {
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T: AssetLoader + 'static> DynLoadRegistry for TypedLoadRegistry<T> {
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self as &mut dyn Any
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

trait DynInsertRegistry: ConditionalSend {
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T: Asset + 'static, I: AssetInserter + 'static> DynInsertRegistry
    for TypedInsertRegistry<T, I>
{
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self as &mut dyn Any
    }
}
