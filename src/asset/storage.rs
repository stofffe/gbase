use crate::asset::{AssetCacheRegistry, AssetHandle};
use rustc_hash::FxHashMap;
use std::any::{Any, TypeId};

//
// Types
//

#[cfg(not(target_arch = "wasm32"))]
pub trait Asset: Any + Send {}

#[cfg(target_arch = "wasm32")]
pub trait Asset: Any {}

pub enum GetAssetResult<'a, T: Asset> {
    Loading,
    Success(&'a T),
    Error,
}

impl<'a, T: Asset> GetAssetResult<'a, T> {
    pub fn unwrap_success(self) -> &'a T {
        match self {
            Self::Success(asset) => asset,
            Self::Loading => panic!("Asset is still loading"),
            Self::Error => panic!("Asset failed to load"),
        }
    }

    pub fn unwrap_success_cloned(self) -> T
    where
        T: Clone,
    {
        match self {
            Self::Success(asset) => asset.clone(),
            Self::Loading => panic!("Asset is still loading"),
            Self::Error => panic!("Asset failed to load"),
        }
    }
}

pub enum GetAssetResultCloned<T: Asset> {
    Loading,
    Success(T),
    Error,
}

impl<T: Asset> GetAssetResultCloned<T> {
    pub fn unwrap_success(self) -> T {
        match self {
            Self::Success(asset) => asset,
            Self::Loading => panic!("Asset is still loading"),
            Self::Error => panic!("Asset failed to load"),
        }
    }
}

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
            .or_insert(Box::new(TypedAssetStorage::<T>::new()));
        entry
            .as_any_mut()
            .downcast_mut::<TypedAssetStorage<T>>()
            .expect("could not downcast typed storage cache")
    }

    pub(crate) fn insert_asset<T: Asset>(
        &mut self,
        registry: &mut AssetCacheRegistry,
        data: T,
    ) -> AssetHandle<T> {
        let handle = registry.crate_insert_handle();
        self.insert_asset_with_handle(handle.clone(), data);
        handle
    }

    pub(crate) fn insert_asset_with_handle<T: Asset>(&mut self, handle: AssetHandle<T>, data: T) {
        self.get_typed_cache_mut::<T>().insert(handle, data)
    }

    pub(crate) fn get_asset<T: Asset>(&self, handle: &AssetHandle<T>) -> Option<&T> {
        if let Some(typd_cache) = self.get_typed_cache_ref::<T>() {
            typd_cache.get(handle)
        } else {
            None
        }
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
