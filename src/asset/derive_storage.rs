use crate::{
    asset::{AssetHandleContext, DerivedHandle},
    render::ArcHandle,
    Context,
};
use rustc_hash::FxHashMap;
use std::any::{Any, TypeId};

//
// Types
//

pub trait DerivedAsset: Any {} // TODO: is this even needed? or maybe rename

//
// Storage
//

pub struct AssetCacheDerivedStorage {
    typed_caches: FxHashMap<TypeId, Box<dyn DynDerivedStorage>>,

    asset_handle_ctx: AssetHandleContext,
}

impl AssetCacheDerivedStorage {
    pub fn new(asset_handle_ctx: AssetHandleContext) -> Self {
        let typed_caches = FxHashMap::default();
        Self {
            asset_handle_ctx,
            typed_caches,
        }
    }

    /// Get typed cache assuming it exists
    pub fn get_typed_cache_ref<T: DerivedAsset + 'static>(
        &self,
    ) -> Option<&TypedDerivedStorage<T>> {
        self.typed_caches.get(&TypeId::of::<T>()).map(|a| {
            a.as_any()
                .downcast_ref::<TypedDerivedStorage<T>>()
                .expect("could not downcast typed storage cache")
        })
    }

    /// Get mutable typed cache or create if it doesnt exist
    pub fn get_typed_cache_mut<T: DerivedAsset + 'static>(
        &mut self,
    ) -> &mut TypedDerivedStorage<T> {
        let entry = self
            .typed_caches
            .entry(TypeId::of::<T>())
            .or_insert(Box::new(TypedDerivedStorage::<T>::new(
                self.asset_handle_ctx.clone(),
            )));
        entry
            .as_any_mut()
            .downcast_mut::<TypedDerivedStorage<T>>()
            .expect("could not downcast typed storage cache")
    }

    pub fn get<T: DerivedAsset>(&self, handle: &DerivedHandle<T>) -> Option<ArcHandle<T>> {
        if let Some(typed) = self.get_typed_cache_ref::<T>() {
            typed.get(handle)
        } else {
            None
        }
    }

    pub fn insert<T: DerivedAsset>(
        &mut self,
        ctx: &mut Context,
        handle: DerivedHandle<T>,
        data: T,
    ) {
        self.get_typed_cache_mut::<T>().insert(ctx, handle, data)
    }
}

pub enum GetDerivedResult<T: DerivedAsset> {
    Loading,
    Success(ArcHandle<T>),
    Error,
}

//
// Typed/Dyn storage
//

pub struct TypedDerivedStorage<T: DerivedAsset> {
    asset_handle_ctx: AssetHandleContext,

    // TODO: this should not use archandle, replace with T and make converters create archandle
    cache: FxHashMap<DerivedHandle<T>, ArcHandle<T>>,
}

impl<T: DerivedAsset> TypedDerivedStorage<T> {
    pub fn new(asset_handle_ctx: AssetHandleContext) -> Self {
        Self {
            asset_handle_ctx,
            cache: FxHashMap::default(),
        }
    }

    pub fn get(&self, handle: &DerivedHandle<T>) -> Option<ArcHandle<T>> {
        self.cache.get(handle).cloned()
    }

    pub fn insert(&mut self, ctx: &mut Context, handle: DerivedHandle<T>, data: T) {
        self.cache.insert(handle.clone(), ArcHandle::new(ctx, data));
    }
}

pub trait DynDerivedStorage {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T: DerivedAsset> DynDerivedStorage for TypedDerivedStorage<T> {
    fn as_any(&self) -> &dyn Any {
        self as &dyn Any
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self as &mut dyn Any
    }
}
