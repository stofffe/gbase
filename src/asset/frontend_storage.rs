use crate::{
    asset::{Asset, AssetHandle, AssetMetadata, DynAssetHandle},
    ConditionalSend,
};
use rustc_hash::{FxHashMap, FxHashSet};
use std::{
    any::{type_name, Any, TypeId},
    sync::Arc,
};

//
// Types
//

/// Only returned when an asset is not found
#[derive(Clone, Debug)]
pub enum GetAssetState {
    Loading,
    Failed,
}

//
// Generic
//

pub(crate) struct AssetCacheFrontendStorage {
    typed_storage: FxHashMap<TypeId, Box<dyn DynAssetFrontendStorage>>,

    metadata: FxHashMap<DynAssetHandle, AssetMetadata>,

    just_available: FxHashSet<DynAssetHandle>,
}

impl AssetCacheFrontendStorage {
    pub(crate) fn new() -> Self {
        Self {
            typed_storage: FxHashMap::default(),
            metadata: FxHashMap::default(),
            just_available: FxHashSet::default(),
        }
    }

    fn register_typed_storage<T: Asset + 'static>(&mut self) {
        let type_id = TypeId::of::<T>();
        self.typed_storage
            .entry(type_id)
            .or_insert_with(|| Box::new(TypedAssetFrontendStorage::<T>::new()));
    }

    /// Get typed cache assuming it exists
    fn get_typed_storage_ref<T: Asset + 'static>(&self) -> Option<&TypedAssetFrontendStorage<T>> {
        self.typed_storage
            .get(&TypeId::of::<T>())
            .map(|dyn_storage| {
                dyn_storage
                    .as_any()
                    .downcast_ref::<TypedAssetFrontendStorage<T>>()
                    .expect("could not downcast typed storage cache")
            })
    }

    /// Get mutable typed cache or create if it doesnt exist
    fn get_typed_storage_mut<T: Asset + 'static>(
        &mut self,
    ) -> Option<&mut TypedAssetFrontendStorage<T>> {
        self.typed_storage
            .get_mut(&TypeId::of::<T>())
            .map(|dyn_storage| {
                dyn_storage
                    .as_any_mut()
                    .downcast_mut::<TypedAssetFrontendStorage<T>>()
                    .expect("could not downcast typed storage cache")
            })
    }

    pub(crate) fn get_asset<T: Asset>(&self, handle: &AssetHandle<T>) -> Option<&Arc<T>> {
        let Some(typed_storage) = self.get_typed_storage_ref::<T>() else {
            tracing::warn!("could not get typed frontend cache {}", type_name::<T>());
            return None;
        };

        typed_storage.cache.get(handle)
    }

    pub(crate) fn set_asset<T: Asset + 'static>(&mut self, handle: AssetHandle<T>, asset: Arc<T>) {
        self.register_typed_storage::<T>();

        let Some(typed_storage) = self.get_typed_storage_mut::<T>() else {
            tracing::warn!("could not get typed frontend cache {}", type_name::<T>());
            return;
        };

        typed_storage.cache.insert(handle, asset);
    }

    pub(crate) fn set_metadata(&mut self, dyn_handle: DynAssetHandle, metadata: AssetMetadata) {
        self.metadata.insert(dyn_handle, metadata);
    }

    pub(crate) fn get_metadata(&self, dyn_handle: &DynAssetHandle) -> Option<&AssetMetadata> {
        self.metadata.get(dyn_handle)
    }
}

//
// Typed/Dyn storage
//

struct TypedAssetFrontendStorage<T: Asset> {
    cache: FxHashMap<AssetHandle<T>, Arc<T>>,
}

impl<T: Asset> TypedAssetFrontendStorage<T> {
    fn new() -> Self {
        Self {
            cache: FxHashMap::default(),
        }
    }
}

trait DynAssetFrontendStorage: ConditionalSend {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T: Asset> DynAssetFrontendStorage for TypedAssetFrontendStorage<T> {
    fn as_any(&self) -> &dyn Any {
        self as &dyn Any
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self as &mut dyn Any
    }
}
