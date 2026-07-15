use std::{any::Any, sync::Arc};

use rustc_hash::FxHashMap;

use crate::asset::{Asset, AssetHandle, AssetHandleContext, DynAssetHandle, LoadAssetResult};

pub enum GetAssetResult<'a, T: Asset> {
    Loading,
    Success(&'a T),
    Failed,
}

impl<'a, T: Asset> GetAssetResult<'a, T> {
    pub fn unwrap_loaded(self) -> &'a T {
        match self {
            GetAssetResult::Success(asset) => asset,
            GetAssetResult::Loading => panic!("Asset is still loading"),
            GetAssetResult::Failed => panic!("Asset failed to load"),
        }
    }
}

pub struct AssetCacheStorage {
    pub cache: FxHashMap<DynAssetHandle, LoadAssetResult>,

    asset_handle_ctx: AssetHandleContext,
}

impl AssetCacheStorage {
    pub fn new(asset_handle_ctx: AssetHandleContext) -> Self {
        Self {
            cache: FxHashMap::default(),
            asset_handle_ctx,
        }
    }

    pub fn cache_size(&self) -> usize {
        self.cache.len()
    }

    pub fn clear_handle<T: Asset>(&mut self, handle: AssetHandle<T>) {
        self.cache.remove(&handle.as_any());
    }

    pub fn clear_unused_handles(&mut self) {
        // TODO: clear all other stuff related to this handle
        self.cache
            .retain(|handle, _| Arc::strong_count(&handle.id) > 1);
    }

    pub fn handle_successfully_loaded<T: Asset>(&self, handle: AssetHandle<T>) -> bool {
        let Some(load_result) = self.cache.get(&handle.as_any()) else {
            tracing::warn!("trying to use invalid handle");
            return false;
        };
        // TODO: should error count as loaded or not?
        match load_result {
            LoadAssetResult::Success(_) => true,
            LoadAssetResult::Loading => false,
            LoadAssetResult::Error => false,
        }
    }

    pub fn get<'a, T: Asset + 'static>(&'a self, handle: AssetHandle<T>) -> GetAssetResult<'a, T> {
        let Some(load_result) = self.cache.get(&handle.as_any()) else {
            tracing::warn!("trying to use invalid handle");
            return GetAssetResult::Failed;
        };

        match load_result {
            LoadAssetResult::Success(asset) => {
                let asset = (asset.as_ref() as &dyn Any)
                    .downcast_ref::<T>()
                    .expect("could not downcast");
                GetAssetResult::Success(asset)
            }
            LoadAssetResult::Loading => GetAssetResult::Loading,
            LoadAssetResult::Error => GetAssetResult::Failed,
        }
    }

    pub fn insert_new_handle<T: Asset + 'static>(&mut self, data: T) -> AssetHandle<T> {
        let handle = AssetHandle::<T>::new(&self.asset_handle_ctx);
        self.insert_existing_handle(data, handle)
    }

    pub fn insert_existing_handle<T: Asset + 'static>(
        &mut self,
        data: T,
        handle: AssetHandle<T>,
    ) -> AssetHandle<T> {
        self.cache
            .insert(handle.as_any(), LoadAssetResult::Success(Box::new(data)));
        handle
    }
}
