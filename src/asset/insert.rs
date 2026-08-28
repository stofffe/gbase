use crate::asset::{Asset, AssetCacheRegistry, AssetCacheStorage, AssetHandle};
use std::{fmt::Debug, hash::Hash};

//
// Types
//

pub trait InsertAssetKey: Hash + Eq + Clone {}
impl<T: Hash + Eq + Clone> InsertAssetKey for T {}

#[cfg(not(target_arch = "wasm32"))]
pub trait AssetInserter: Send {
    type Key: InsertAssetKey + Send + Debug;
}

#[cfg(target_arch = "wasm32")]
pub trait AssetInserter {
    type Key: InsertAssetKey;
}

//
// Generic
//

pub struct AssetCacheInsert {}

impl AssetCacheInsert {
    pub(crate) fn new() -> Self {
        Self {}
    }

    pub(crate) fn insert_asset<T: Asset, I: AssetInserter + 'static>(
        &mut self,
        registry: &mut AssetCacheRegistry,
        storage: &mut AssetCacheStorage,
        key: &I::Key,
        asset: T,
    ) -> AssetHandle<T> {
        let handle = registry.get_or_create_insert_handle::<T, I>(key);

        storage.insert_asset(handle.clone(), asset);

        handle
    }

    pub(crate) fn insert_asset_force<T: Asset>(
        &mut self,
        registry: &mut AssetCacheRegistry,
        storage: &mut AssetCacheStorage,
        asset: T,
    ) -> AssetHandle<T> {
        let handle = registry.create_empty_handle::<T>();

        storage.insert_asset(handle.clone(), asset);

        handle
    }
}
