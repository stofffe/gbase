use crate::{
    asset::{AssetHandle, DynAssetHandle},
    ConditionalSend,
};
use rustc_hash::{FxHashMap, FxHashSet};
use std::any::{type_name, Any, TypeId};

//
// Types
//

pub trait Asset: Any + ConditionalSend {}

#[derive(Clone, Debug)]
pub(crate) enum InternalAssetState {
    // TODO: give better name
    Pending,
    Loading,
    Failed,
    Ready,
}

/// Only returned when an asset is not found
#[derive(Clone, Debug)]
pub enum GetAssetState {
    Loading,
    Failed,
}

//
// Generic
//

pub(crate) struct AssetCacheStorage {
    typed_storage: FxHashMap<TypeId, Box<dyn DynAssetStorage>>,
    just_available: FxHashSet<DynAssetHandle>,
}

impl AssetCacheStorage {
    pub(crate) fn new() -> Self {
        Self {
            typed_storage: FxHashMap::default(),
            just_available: FxHashSet::default(),
        }
    }

    /// Get typed cache assuming it exists
    fn get_typed_storage_ref<T: Asset + 'static>(&self) -> Option<&TypedAssetStorage<T>> {
        self.typed_storage
            .get(&TypeId::of::<T>())
            .map(|dyn_storage| {
                dyn_storage
                    .as_any()
                    .downcast_ref::<TypedAssetStorage<T>>()
                    .expect("could not downcast typed storage cache")
            })
    }

    /// Get mutable typed cache or create if it doesnt exist
    fn get_typed_storage_mut<T: Asset + 'static>(&mut self) -> Option<&mut TypedAssetStorage<T>> {
        self.typed_storage
            .get_mut(&TypeId::of::<T>())
            .map(|dyn_storage| {
                dyn_storage
                    .as_any_mut()
                    .downcast_mut::<TypedAssetStorage<T>>()
                    .expect("could not downcast typed storage cache")
            })
    }

    pub(crate) fn register_asset<T: Asset>(&mut self, handle: AssetHandle<T>) {
        // get or create the typed storage
        let dyn_storage = self
            .typed_storage
            .entry(TypeId::of::<T>())
            .or_insert_with(|| Box::new(TypedAssetStorage::<T>::new()));
        let typed_storage = dyn_storage
            .as_any_mut()
            .downcast_mut::<TypedAssetStorage<T>>()
            .expect("could not downcast typed storage cache");

        // insert into storage
        typed_storage.cache.insert(
            handle,
            AssetEntry {
                asset: None,
                state: InternalAssetState::Pending, // TODO: add state for invalid/not started
                debug_name: None,
            },
        );
    }

    pub(crate) fn insert_asset<T: Asset>(&mut self, handle: AssetHandle<T>, asset: T) {
        tracing::info!("insert into storage {}", handle);
        let Some(typed_storage) = self.get_typed_storage_mut::<T>() else {
            panic!("could not get typed cache {}", type_name::<T>());
        };

        typed_storage.cache.insert(
            handle.clone(),
            AssetEntry {
                asset: Some(asset),
                state: InternalAssetState::Ready,
                debug_name: None,
            },
        );
    }

    fn get_asset_entry<T: Asset>(&self, handle: &AssetHandle<T>) -> Option<&AssetEntry<T>> {
        let Some(typed_storage) = self.get_typed_storage_ref::<T>() else {
            panic!("could not get typed cache {}", type_name::<T>());
        };

        typed_storage.cache.get(handle)
    }

    pub(crate) fn get_asset<T: Asset>(&self, handle: &AssetHandle<T>) -> Option<&T> {
        let entry = self
            .get_asset_entry(handle)
            .expect("could not get asset entry");
        entry.asset.as_ref()
    }

    pub(crate) fn get_asset_state<T: Asset>(&self, handle: &AssetHandle<T>) -> InternalAssetState {
        let entry = self
            .get_asset_entry(handle)
            .expect("could not get asset entry");
        entry.state.clone()
    }

    pub(crate) fn set_asset_state(
        &mut self,
        dyn_handle: DynAssetHandle,
        state: InternalAssetState,
    ) {
        tracing::info!("SET STATE {} {:?}", dyn_handle, state);
        let Some(dyn_storage) = self.typed_storage.get_mut(&dyn_handle.asset_type_id()) else {
            panic!("could not get typed storage for {}", dyn_handle);
        };

        dyn_storage.set_asset_state(dyn_handle, state);
    }

    pub(crate) fn clear_asset<T: Asset>(&mut self, handle: &AssetHandle<T>) {
        let Some(typed_storage) = self.get_typed_storage_mut::<T>() else {
            panic!("could not get typed cache {}", type_name::<T>());
        };

        typed_storage.cache.remove(handle);
    }

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
// Typed/Dyn storage
//

struct AssetEntry<T: Asset> {
    asset: Option<T>,
    state: InternalAssetState,
    debug_name: Option<String>,
    // waiting senders
}

struct TypedAssetStorage<T: Asset> {
    cache: FxHashMap<AssetHandle<T>, AssetEntry<T>>,
}

impl<T: Asset> TypedAssetStorage<T> {
    fn new() -> Self {
        Self {
            cache: FxHashMap::default(),
        }
    }
}

trait DynAssetStorage {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn set_asset_state(&mut self, dyn_handle: DynAssetHandle, state: InternalAssetState);
}

impl<T: Asset> DynAssetStorage for TypedAssetStorage<T> {
    fn as_any(&self) -> &dyn Any {
        self as &dyn Any
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self as &mut dyn Any
    }

    fn set_asset_state(&mut self, dyn_handle: DynAssetHandle, state: InternalAssetState) {
        let handle = dyn_handle
            .to_typed::<T>()
            .expect("could not convert dyn handle to typed");

        self.cache
            .get_mut(&handle)
            .expect("could not get asset entry")
            .state = state;
    }
}
