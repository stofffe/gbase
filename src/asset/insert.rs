use crate::{
    asset::{Asset, AssetCacheStorage, AssetHandle, DynAssetHandle, FrontendMessage},
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

    // pub(crate) fn insert_asset_force<T: Asset>(
    //     &mut self,
    //     frontend_message_sender: async_channel::Sender<FrontendMessage>,
    //     storage: &mut AssetCacheStorage,
    //     handle: AssetHandle<T>,
    //     // TODO: why unused, is it because its already used in registry?
    //     scope: Option<DynAssetHandle>,
    //     asset: T,
    // ) -> AssetHandle<T> {
    //     storage.register_asset_if_needed(frontend_message_sender.clone(), handle.clone());
    //
    //     self.insert_asset_with_handle::<T>(frontend_message_sender, storage, handle.clone(), asset);
    //
    //     handle
    // }

    pub(crate) fn insert_asset<T: Asset>(
        &mut self,
        frontend_message_sender: async_channel::Sender<FrontendMessage>,
        storage: &mut AssetCacheStorage,
        handle: AssetHandle<T>,
        // TODO: why unused, is it because its already used in registry?
        scope: Option<DynAssetHandle>,
        asset: T,
    ) -> AssetHandle<T> {
        let is_dependency = scope.is_some();
        storage.register_asset_if_needed(
            frontend_message_sender.clone(),
            handle.clone(),
            is_dependency,
        );

        self.insert_asset_with_handle::<T>(frontend_message_sender, storage, handle.clone(), asset);

        handle
    }

    fn insert_asset_with_handle<T: Asset>(
        &mut self,
        frontend_message_sender: async_channel::Sender<FrontendMessage>,
        storage: &mut AssetCacheStorage,
        handle: AssetHandle<T>,
        asset: T,
    ) {
        storage.insert_asset(frontend_message_sender, handle.clone(), asset);
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
