use super::DynAsset;
use crate::asset::{self, Asset, AssetCache, GetAssetResult};
use std::{
    marker::PhantomData,
    sync::{Arc, Mutex},
};

//
// Asset handle
//

#[derive(Debug)]
pub struct AssetHandle<T: Asset + 'static> {
    pub(crate) id: Arc<u64>, // TODO: use strong and weak outside/inside cache
    pub(crate) ty: PhantomData<T>,
}

impl<T: Asset + 'static> AssetHandle<T> {
    pub fn new(asset_handle_ctx: &asset::AssetHandleContext) -> Self {
        let id = asset_handle_ctx.next_id();
        Self {
            id: Arc::new(id),
            ty: PhantomData,
        }
    }

    #[inline]
    pub fn id(&self) -> u64 {
        *self.id
    }

    pub(crate) fn as_any(&self) -> AssetHandle<DynAsset> {
        AssetHandle::<DynAsset> {
            id: self.id.clone(),
            ty: PhantomData,
        }
    }

    pub fn loaded(&self, cache: &mut AssetCache) -> bool {
        cache.handle_successfully_loaded(self.clone())
    }

    pub fn just_loaded(&self, cache: &AssetCache) -> bool {
        cache.handle_just_loaded(self.clone())
    }

    pub fn get<'a>(&self, cache: &'a mut AssetCache) -> GetAssetResult<'a, T> {
        cache.get(self.clone())
    }
}

impl<T: Asset + 'static> PartialOrd for AssetHandle<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: Asset + 'static> Ord for AssetHandle<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.id.cmp(&other.id)
    }
}

impl<T: Asset + 'static> PartialEq for AssetHandle<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<T: Asset + 'static> Eq for AssetHandle<T> {}

impl<T: Asset + 'static> std::hash::Hash for AssetHandle<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl<T: Asset + 'static> Clone for AssetHandle<T> {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            ty: PhantomData,
        }
    }
}

//
// Asset handle context
//

/// Thread safe context for creating new handles
#[derive(Debug, Clone)]
pub struct AssetHandleContext {
    id: Arc<Mutex<u64>>,
}

impl AssetHandleContext {
    pub fn new() -> Self {
        Self {
            id: Arc::new(Mutex::new(0)),
        }
    }

    pub fn next_id(&self) -> u64 {
        let mut id_guard = self.id.lock().expect("could not unlock asset id lock");
        let id = *id_guard;
        *id_guard += 1;
        id
    }
}
