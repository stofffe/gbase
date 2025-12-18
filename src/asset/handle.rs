use super::DynAsset;
use std::{
    marker::PhantomData,
    sync::{atomic::AtomicU64, Arc},
};

// TODO: remove like arc handles
static NEXT_ID: AtomicU64 = AtomicU64::new(0);

// TODO: make while arc?
// then you can have type_id inside

// TODO: is ty_id needed?
#[derive(Debug)]
pub struct AssetHandle<T: 'static> {
    pub(crate) id: Arc<u64>, // TODO: use strong and weak outside/inside cache
    pub(crate) ty: PhantomData<T>,
}

impl<T: 'static> AssetHandle<T> {
    #![allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let id = NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
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
}

impl<T: 'static> PartialOrd for AssetHandle<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.id.partial_cmp(&other.id)
    }
}

impl<T: 'static> Ord for AssetHandle<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.id.cmp(&other.id)
    }
}

impl<T: 'static> PartialEq for AssetHandle<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl<T: 'static> Eq for AssetHandle<T> {}

impl<T: 'static> std::hash::Hash for AssetHandle<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl<T: 'static> Clone for AssetHandle<T> {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            ty: PhantomData,
        }
    }
}

// impl<T: 'static> Copy for AssetHandle<T> {}
