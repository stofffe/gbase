use crate::asset::{
    Asset, AssetCacheDerived, AssetHandle, AssetHandleContext, DynAssetHandle, LoadAssetResult,
};
use rustc_hash::FxHashMap;
use std::{any::Any, sync::Arc};

pub struct AssetCacheStorage {
    cache: FxHashMap<DynAssetHandle, LoadAssetResult>,

    asset_handle_ctx: AssetHandleContext,
}

impl AssetCacheStorage {
    pub fn new(asset_handle_ctx: AssetHandleContext) -> Self {
        Self {
            cache: FxHashMap::default(),
            asset_handle_ctx,
        }
    }

    pub(crate) fn clear_handle(&mut self, derived: &mut AssetCacheDerived, handle: DynAssetHandle) {
        self.cache.remove(&handle.as_any());
        // clean other uses of handle
        derived.invalidate_derived_assets_depending_on_handle(handle.clone());
    }

    pub(crate) fn clear_unused_handles(&mut self, derived: &mut AssetCacheDerived) {
        let mut handles_to_remove = Vec::new();
        for (handle, _) in self.cache.iter() {
            if Arc::strong_count(&handle.id) <= 1 {
                handles_to_remove.push(handle.clone());
            }
        }

        for handle in handles_to_remove {
            self.clear_handle(derived, handle);
        }
    }

    pub fn handle_successfully_loaded<T: Asset>(&self, handle: AssetHandle<T>) -> bool {
        let Some(load_result) = self.cache.get(&handle.as_any()) else {
            tracing::warn!("trying to use invalid handle");
            return false;
        };
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

    pub fn insert_successful_new_handle<T: Asset + 'static>(&mut self, data: T) -> AssetHandle<T> {
        let handle = AssetHandle::<T>::new(&self.asset_handle_ctx);
        self.cache
            .insert(handle.as_any(), LoadAssetResult::Success(Box::new(data)));
        handle
    }

    pub fn insert_successful_existing_handle<T: Asset + 'static>(
        &mut self,
        derived: &mut AssetCacheDerived,
        handle: AssetHandle<T>,
        data: T,
    ) -> AssetHandle<T> {
        self.cache
            .insert(handle.as_any(), LoadAssetResult::Success(Box::new(data)));
        derived.invalidate_derived_assets_depending_on_handle(handle.as_any());
        handle
    }

    pub fn insert(&mut self, handle: DynAssetHandle, data: LoadAssetResult) -> DynAssetHandle {
        self.cache.insert(handle.clone(), data);
        handle
    }
}

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
