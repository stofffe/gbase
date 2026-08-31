use crate::asset::{AssetCacheRegistry, AssetHandle};
use rustc_hash::FxHashMap;
use std::any::{type_name, Any, TypeId};

//
// Types
//

#[cfg(not(target_arch = "wasm32"))]
pub trait Asset: Any + Send {}

#[cfg(target_arch = "wasm32")]
pub trait Asset: Any {}

//
// Generic
//

pub(crate) struct AssetCacheStorage {
    typed_storage: FxHashMap<TypeId, Box<dyn DynAssetStorage>>,
}

impl AssetCacheStorage {
    pub(crate) fn new() -> Self {
        let typed_storage = FxHashMap::default();
        Self { typed_storage }
    }

    /// Get typed cache assuming it exists
    fn get_typed_cache_ref<T: Asset + 'static>(&self) -> Option<&TypedAssetStorage<T>> {
        self.typed_storage.get(&TypeId::of::<T>()).map(|a| {
            a.as_any()
                .downcast_ref::<TypedAssetStorage<T>>()
                .expect("could not downcast typed storage cache")
        })
    }

    /// Get mutable typed cache or create if it doesnt exist
    fn get_typed_cache_mut<T: Asset + 'static>(&mut self) -> &mut TypedAssetStorage<T> {
        let entry = self
            .typed_storage
            .entry(TypeId::of::<T>())
            .or_insert_with(|| Box::new(TypedAssetStorage::<T>::new()));
        entry
            .as_any_mut()
            .downcast_mut::<TypedAssetStorage<T>>()
            .expect("could not downcast typed storage cache")
    }

    pub(crate) fn insert_asset<T: Asset>(&mut self, handle: AssetHandle<T>, asset: T) {
        tracing::info!("insert into storage {}", handle);
        self.get_typed_cache_mut::<T>().insert(handle, asset);
    }

    pub(crate) fn get_asset<T: Asset>(&self, handle: &AssetHandle<T>) -> Option<&T> {
        if let Some(typd_cache) = self.get_typed_cache_ref::<T>() {
            typd_cache.get(handle)
        } else {
            None
        }
    }

    pub(crate) fn clear_asset<T: Asset>(&mut self, handle: &AssetHandle<T>) {
        self.get_typed_cache_mut::<T>().cache.remove(handle);
    }
}

//
// Typed/Dyn storage
//

struct TypedAssetStorage<T: Asset> {
    cache: FxHashMap<AssetHandle<T>, T>,
}

impl<T: Asset> TypedAssetStorage<T> {
    fn new() -> Self {
        Self {
            cache: FxHashMap::default(),
        }
    }

    fn get(&self, handle: &AssetHandle<T>) -> Option<&T> {
        self.cache.get(handle)
    }

    fn insert(&mut self, handle: AssetHandle<T>, data: T) {
        self.cache.insert(handle.clone(), data);
    }
}

trait DynAssetStorage {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T: Asset> DynAssetStorage for TypedAssetStorage<T> {
    fn as_any(&self) -> &dyn Any {
        self as &dyn Any
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self as &mut dyn Any
    }
}
