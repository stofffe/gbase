use crate::{
    asset::{
        ArcAssetCacheRegistry, Asset, AssetCacheRegistry, AssetCacheStorage, AssetHandle,
        BackendToFrontendMessage, DynAssetHandle,
    },
    ConditionalSend,
};
use std::{fmt::Debug, hash::Hash};

//
// Types
//

pub trait InsertAssetKey: Debug + Hash + Eq + Clone {}
impl<T: Debug + Hash + Eq + Clone> InsertAssetKey for T {}

pub trait AssetInserter: ConditionalSend {
    type Key: InsertAssetKey + ConditionalSend;
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
        backend_to_frontend_sender: async_channel::Sender<BackendToFrontendMessage>,
        registry: &ArcAssetCacheRegistry,
        storage: &mut AssetCacheStorage,
        key: I::Key,
        asset: T,
    ) -> AssetHandle<T> {
        let handle = registry.get_insert_handle::<T, I>(key, None);
        storage.register_asset_if_needed(handle.clone());

        self.insert_asset_with_handle::<T>(
            backend_to_frontend_sender,
            storage,
            handle.clone(),
            asset,
        );

        handle
    }

    pub(crate) fn insert_asset_scoped<T: Asset, I: AssetInserter + 'static>(
        &mut self,
        backend_to_frontend_sender: async_channel::Sender<BackendToFrontendMessage>,
        storage: &mut AssetCacheStorage,
        handle: AssetHandle<T>,
        // TODO: why unused
        scope: Option<DynAssetHandle>,
        asset: T,
    ) -> AssetHandle<T> {
        storage.register_asset_if_needed(handle.clone());

        self.insert_asset_with_handle::<T>(
            backend_to_frontend_sender,
            storage,
            handle.clone(),
            asset,
        );

        handle
    }

    // pub(crate) fn insert_asset_with_new_handle<T: Asset>(
    //     &mut self,
    //     registry: &mut AssetCacheRegistry,
    //     storage: &mut AssetCacheStorage,
    //     asset: T,
    // ) -> AssetHandle<T> {
    //     let handle = registry.create_empty_handle::<T>(storage);
    //
    //     self.insert_asset_with_handle(storage, handle.clone(), asset);
    //
    //     handle
    // }

    fn insert_asset_with_handle<T: Asset>(
        &mut self,
        backend_to_frontend_sender: async_channel::Sender<BackendToFrontendMessage>,
        storage: &mut AssetCacheStorage,
        handle: AssetHandle<T>,
        asset: T,
    ) {
        storage.insert_asset(backend_to_frontend_sender, handle.clone(), asset);
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
