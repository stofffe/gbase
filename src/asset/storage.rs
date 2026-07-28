use crate::asset::{AssetCacheDerived, AssetHandle, AssetHandleContext};
use rustc_hash::FxHashMap;
use std::{
    any::{Any, TypeId},
    sync::Arc,
};

//
// Types
//

pub trait Asset: Any + Send + Sync {} // TODO: is this even needed? or maybe rename
                                      //
pub type DynAsset = Box<dyn Asset>;
impl Asset for DynAsset {}

pub type DynAssetHandle = AssetHandle<DynAsset>;

//
// Storage
//

pub struct AssetCacheStorage {
    typed_caches: FxHashMap<TypeId, Box<dyn DynAssetStorage>>,
    asset_handle_ctx: AssetHandleContext,
}

impl AssetCacheStorage {
    pub fn new(asset_handle_ctx: AssetHandleContext) -> Self {
        let typed_caches = FxHashMap::default();
        Self {
            asset_handle_ctx,
            typed_caches,
        }
    }

    /// Get typed cache assuming it exists
    pub fn get_typed_cache_ref<T: Asset + 'static>(&self) -> Option<&TypedAssetStorage<T>> {
        self.typed_caches.get(&TypeId::of::<T>()).map(|a| {
            a.as_any()
                .downcast_ref::<TypedAssetStorage<T>>()
                .expect("could not downcast typed storage cache")
        })
    }

    /// Get mutable typed cache or create if it doesnt exist
    pub fn get_typed_cache_mut<T: Asset + 'static>(&mut self) -> &mut TypedAssetStorage<T> {
        let entry = self
            .typed_caches
            .entry(TypeId::of::<T>())
            .or_insert(Box::new(TypedAssetStorage::<T>::new(
                self.asset_handle_ctx.clone(),
            )));
        entry
            .as_any_mut()
            .downcast_mut::<TypedAssetStorage<T>>()
            .expect("could not downcast typed storage cache")
    }

    pub fn get<T: Asset>(&self, handle: &AssetHandle<T>) -> Option<&T> {
        if let Some(typd_cache) = self.get_typed_cache_ref::<T>() {
            typd_cache.get(handle)
        } else {
            None
        }
    }

    pub(crate) fn clear_handle<T: Asset>(
        &mut self,
        derived: &mut AssetCacheDerived,
        handle: AssetHandle<T>,
    ) {
        self.get_typed_cache_mut::<T>()
            .clear_handle(derived, handle)
    }

    pub(crate) fn clear_unused_handles(&mut self, derived: &mut AssetCacheDerived) {
        for (_, dyn_cache) in self.typed_caches.iter_mut() {
            dyn_cache.clear_unused_handles(derived);
        }
    }

    pub fn handle_successfully_loaded<T: Asset>(&mut self, handle: AssetHandle<T>) -> bool {
        self.get_typed_cache_mut::<T>()
            .handle_successfully_loaded(handle)
    }

    pub fn insert<T: Asset>(&mut self, handle: AssetHandle<T>, data: T) {
        self.get_typed_cache_mut::<T>().insert(handle, data)
    }

    pub fn insert_successful_new_handle<T: Asset>(&mut self, data: T) -> AssetHandle<T> {
        let handle = AssetHandle::<T>::new(&self.asset_handle_ctx);
        self.insert(handle.clone(), data);
        handle
    }

    pub fn insert_successful_existing_handle<T: Asset>(
        &mut self,
        derived: &mut AssetCacheDerived,
        handle: AssetHandle<T>,
        data: T,
    ) -> AssetHandle<T> {
        self.insert(handle.clone(), data);
        derived.invalidate_derived_assets_depending_on_handle(handle.as_any());
        handle
    }
}

pub enum GetAssetResult<'a, T: Asset> {
    Loading,
    Success(&'a T),
    Error,
}

impl<'a, T: Asset> GetAssetResult<'a, T> {
    pub fn unwrap_success(self) -> &'a T {
        match self {
            GetAssetResult::Success(asset) => asset,
            GetAssetResult::Loading => panic!("Asset is still loading"),
            GetAssetResult::Error => panic!("Asset failed to load"),
        }
    }
}

//
// Typed/Dyn storage
//

pub struct TypedAssetStorage<T: Asset> {
    asset_handle_ctx: AssetHandleContext,
    cache: FxHashMap<AssetHandle<T>, T>,
}

impl<T: Asset> TypedAssetStorage<T> {
    pub fn new(asset_handle_ctx: AssetHandleContext) -> Self {
        Self {
            asset_handle_ctx,
            cache: FxHashMap::default(),
        }
    }

    pub(crate) fn clear_handle(&mut self, derived: &mut AssetCacheDerived, handle: AssetHandle<T>) {
        self.cache.remove(&handle);

        // clean other uses of handle
        derived.invalidate_derived_assets_depending_on_handle(handle.as_any());
    }

    pub fn handle_successfully_loaded(&self, handle: AssetHandle<T>) -> bool {
        self.cache.get(&handle).is_some()
    }

    pub fn get(&self, handle: &AssetHandle<T>) -> Option<&T> {
        self.cache.get(handle)
    }

    pub fn insert_successful_new_handle(&mut self, data: T) -> AssetHandle<T> {
        let handle = AssetHandle::<T>::new(&self.asset_handle_ctx);
        self.cache.insert(handle.clone(), data);
        handle
    }

    pub fn insert_successful_existing_handle(
        &mut self,
        derived: &mut AssetCacheDerived,
        handle: AssetHandle<T>,
        data: T,
    ) -> AssetHandle<T> {
        self.cache.insert(handle.clone(), data);
        derived.invalidate_derived_assets_depending_on_handle(handle.as_any());
        handle
    }

    pub fn insert(&mut self, handle: AssetHandle<T>, data: T) {
        self.cache.insert(handle.clone(), data);
    }
}

pub trait DynAssetStorage {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn clear_unused_handles(&mut self, derived: &mut AssetCacheDerived);
}

impl<T: Asset> DynAssetStorage for TypedAssetStorage<T> {
    fn as_any(&self) -> &dyn Any {
        self as &dyn Any
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self as &mut dyn Any
    }

    fn clear_unused_handles(&mut self, derived: &mut AssetCacheDerived) {
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
}
