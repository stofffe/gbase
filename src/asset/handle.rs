use crate::asset::{self, Asset, AssetCache, GetAssetResult};
use std::{
    any::{type_name, TypeId},
    fmt::Display,
    marker::PhantomData,
    sync::{Arc, Mutex},
};

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct DynAssetHandle {
    id: Arc<u64>,
    type_id: TypeId,
}

impl DynAssetHandle {
    pub fn new<T: Asset>(handle: &AssetHandle<T>) -> Self {
        Self {
            id: handle.id.clone(),
            type_id: TypeId::of::<T>(),
        }
    }

    pub fn id(&self) -> u64 {
        *self.id
    }

    pub fn to_typed<T: Asset + 'static>(&self) -> Option<AssetHandle<T>> {
        if self.type_id != TypeId::of::<T>() {
            return None;
        }

        Some(AssetHandle {
            id: self.id.clone(),
            ty: PhantomData,
        })
    }
}

impl Display for DynAssetHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}: dyn asset handle]", self.id())
    }
}

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

    pub(crate) fn to_dyn(&self) -> DynAssetHandle {
        DynAssetHandle::new(self)
    }

    pub fn loaded(&self, cache: &mut AssetCache) -> bool {
        cache.handle_successfully_loaded(self.clone())
    }

    pub fn just_loaded(&self, cache: &AssetCache) -> bool {
        cache.handle_just_loaded(self.clone())
    }

    pub fn get<'a>(&self, cache: &'a mut AssetCache) -> GetAssetResult<'a, T> {
        cache.get(self)
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

impl<T: Asset + 'static> Display for AssetHandle<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}: {}]", self.id(), type_name::<T>())
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
