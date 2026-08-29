use crate::asset::{
    Asset, AssetCacheRegistry, AssetCacheStorage, AssetHandle, DynAssetHandle, LoadStatus,
};
use std::{fmt::Debug, hash::Hash};

//
// Types
//

pub trait InsertAssetKey: Debug + Hash + Eq + Clone {}
impl<T: Debug + Hash + Eq + Clone> InsertAssetKey for T {}

#[cfg(not(target_arch = "wasm32"))]
pub trait AssetInserter: Send {
    type Key: InsertAssetKey + Send;
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
        key: I::Key,
        asset: T,
    ) -> AssetHandle<T> {
        let handle = registry.get_or_create_insert_handle::<T, I>(key, None);

        self.insert_asset_with_handle::<T>(registry, storage, handle.clone(), asset);

        handle
    }

    pub(crate) fn insert_asset_scoped<T: Asset, I: AssetInserter + 'static>(
        &mut self,
        registry: &mut AssetCacheRegistry,
        storage: &mut AssetCacheStorage,
        key: I::Key,
        scope: DynAssetHandle,
        asset: T,
    ) -> AssetHandle<T> {
        let handle = registry.get_or_create_insert_handle::<T, I>(key, Some(scope));

        self.insert_asset_with_handle::<T>(registry, storage, handle.clone(), asset);

        handle
    }

    pub(crate) fn insert_asset_with_new_handle<T: Asset>(
        &mut self,
        registry: &mut AssetCacheRegistry,
        storage: &mut AssetCacheStorage,
        asset: T,
    ) -> AssetHandle<T> {
        let handle = registry.create_empty_handle::<T>();

        self.insert_asset_with_handle(registry, storage, handle.clone(), asset);

        handle
    }

    fn insert_asset_with_handle<T: Asset>(
        &mut self,
        registry: &mut AssetCacheRegistry,
        storage: &mut AssetCacheStorage,
        handle: AssetHandle<T>,
        asset: T,
    ) {
        storage.insert_asset(handle.clone(), asset);
        registry.set_status(handle.to_dyn(), LoadStatus::Ready);
    }
}

//
// Asset key
//

pub(crate) struct ScopedInsertAssetKey<I: AssetInserter> {
    key: I::Key,
    scope: Option<DynAssetHandle>,
}

impl<I: AssetInserter> ScopedInsertAssetKey<I> {
    pub(crate) fn new(key: I::Key, scope: Option<DynAssetHandle>) -> Self {
        Self { key, scope }
    }

    pub(crate) fn key(&self) -> &I::Key {
        &self.key
    }

    pub(crate) fn scope(&self) -> &Option<DynAssetHandle> {
        &self.scope
    }
}

impl<T: AssetInserter> Hash for ScopedInsertAssetKey<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.key.hash(state);
        self.scope.hash(state);
    }
}
impl<T: AssetInserter> Clone for ScopedInsertAssetKey<T> {
    fn clone(&self) -> Self {
        Self {
            key: self.key.clone(),
            scope: self.scope.clone(),
        }
    }
}
impl<T: AssetInserter> Eq for ScopedInsertAssetKey<T> {}
impl<T: AssetInserter> PartialEq for ScopedInsertAssetKey<T> {
    fn eq(&self, other: &Self) -> bool {
        self.key == other.key && self.scope == other.scope
    }
}
