use crate::{
    asset::{AssetHandle, BackendToFrontendMessage, DynAssetHandle, TypedFrontendAssetLoaded},
    ConditionalSend, ConditionalSync,
};
use rustc_hash::{FxHashMap, FxHashSet};
use std::{
    any::{type_name, Any, TypeId},
    sync::Arc,
};

//
// Types
//

pub trait Asset: Any + ConditionalSend + ConditionalSync {}
impl<T: Any + ConditionalSend + ConditionalSync> Asset for T {}

#[derive(Clone, Debug)]
pub(crate) enum InternalAssetState {
    // TODO: give better name
    Pending,
    Loading,
    Failed,
    Ready,
}

pub struct AssetMetadata {
    pub(crate) state: InternalAssetState,
    pub(crate) debug_name: Option<String>,
}

//
// Generic
//

pub(crate) struct AssetCacheStorage {
    typed_storage: FxHashMap<TypeId, Box<dyn DynAssetStorage>>,

    metadata: FxHashMap<DynAssetHandle, AssetMetadata>,

    just_available: FxHashSet<DynAssetHandle>,
}

impl AssetCacheStorage {
    pub(crate) fn new() -> Self {
        Self {
            typed_storage: FxHashMap::default(),
            metadata: FxHashMap::default(),
            just_available: FxHashSet::default(),
        }
    }

    /// Get typed cache assuming it exists
    fn get_typed_storage_ref<T: Asset + 'static>(&self) -> Option<&TypedAssetStorage<T>> {
        self.typed_storage
            .get(&TypeId::of::<T>())
            .map(|dyn_storage| {
                dyn_storage
                    .as_any()
                    .downcast_ref::<TypedAssetStorage<T>>()
                    .expect("could not downcast typed storage cache")
            })
    }

    /// Get mutable typed cache or create if it doesnt exist
    fn get_typed_storage_mut<T: Asset + 'static>(&mut self) -> Option<&mut TypedAssetStorage<T>> {
        self.typed_storage
            .get_mut(&TypeId::of::<T>())
            .map(|dyn_storage| {
                dyn_storage
                    .as_any_mut()
                    .downcast_mut::<TypedAssetStorage<T>>()
                    .expect("could not downcast typed storage cache")
            })
    }

    fn get_asset_entry<T: Asset>(&self, handle: &AssetHandle<T>) -> Option<&AssetEntry<T>> {
        let Some(typed_storage) = self.get_typed_storage_ref::<T>() else {
            panic!("could not get typed cache {}", type_name::<T>());
        };

        typed_storage.cache.get(handle)
    }

    fn get_asset_entry_mut<T: Asset>(
        &mut self,
        handle: &AssetHandle<T>,
    ) -> Option<&mut AssetEntry<T>> {
        let Some(typed_storage) = self.get_typed_storage_mut::<T>() else {
            panic!("could not get typed cache {}", type_name::<T>());
        };

        typed_storage.cache.get_mut(handle)
    }

    fn get_asset_metadata(&self, handle: &DynAssetHandle) -> Option<&AssetMetadata> {
        self.metadata.get(handle)
    }

    fn get_asset_metadata_mut(&mut self, handle: &DynAssetHandle) -> Option<&mut AssetMetadata> {
        self.metadata.get_mut(handle)
    }

    pub(crate) fn register_asset_if_needed<T: Asset>(&mut self, handle: AssetHandle<T>) {
        // get or create the typed storage
        let dyn_storage = self
            .typed_storage
            .entry(TypeId::of::<T>())
            .or_insert_with(|| Box::new(TypedAssetStorage::<T>::new()));
        let typed_storage = dyn_storage
            .as_any_mut()
            .downcast_mut::<TypedAssetStorage<T>>()
            .expect("could not downcast typed storage cache");

        // insert metadata
        self.metadata
            .entry(handle.to_dyn())
            .or_insert(AssetMetadata {
                state: InternalAssetState::Pending,
                debug_name: None,
            });

        // insert into storage
        typed_storage.cache.entry(handle).or_insert(AssetEntry {
            asset: None,
            get_requests: Vec::new(),
        });
    }

    pub(crate) fn insert_asset<T: Asset>(
        &mut self,
        backend_to_frontend_sender: async_channel::Sender<BackendToFrontendMessage>,
        handle: AssetHandle<T>,
        asset: T,
    ) {
        tracing::warn!("insert into storage {}", handle);

        let Some(entry) = self.get_asset_entry_mut(&handle) else {
            panic!("could not get asset entry for {}", handle);
        };

        // insert asset
        let arc_asset = Arc::new(asset);
        entry.asset = Some(arc_asset.clone());

        // notify waiting requests
        for get_request in entry.get_requests.drain(..) {
            get_request
                .try_send(arc_asset.clone())
                .expect("could not send get request response");
        }

        // notify frontend
        backend_to_frontend_sender
            .try_send(BackendToFrontendMessage::AssetUpdate(Box::new(
                TypedFrontendAssetLoaded::new(handle.clone(), arc_asset),
            )))
            .expect("could not send asset update message");

        // update metadata
        let Some(metadata) = self.metadata.get_mut(&handle.to_dyn()) else {
            panic!("could not get asset entry for {}", handle);
        };
        metadata.state = InternalAssetState::Ready;
    }

    pub(crate) fn get_asset<T: Asset>(&self, handle: &AssetHandle<T>) -> Option<&Arc<T>> {
        let entry = self
            .get_asset_entry(handle)
            .expect("could not get asset entry");
        entry.asset.as_ref()
    }

    pub(crate) fn clear_asset<T: Asset>(&mut self, handle: &AssetHandle<T>) {
        let Some(typed_storage) = self.get_typed_storage_mut::<T>() else {
            panic!("could not get typed cache {}", type_name::<T>());
        };

        typed_storage.cache.remove(handle);
    }

    pub(crate) fn add_get_request<T: Asset>(
        &mut self,
        handle: &AssetHandle<T>,
        get_request: async_channel::Sender<Arc<T>>,
    ) {
        let Some(entry) = self.get_asset_entry_mut(handle) else {
            panic!("could not get asset entry for {}", handle);
        };

        tracing::info!("add get request for {}", handle);
        entry.get_requests.push(get_request);
    }

    pub(crate) fn get_asset_state(&self, dyn_handle: &DynAssetHandle) -> InternalAssetState {
        let Some(metadata) = self.get_asset_metadata(dyn_handle) else {
            panic!("could not get asset metadata for {}", dyn_handle);
        };
        metadata.state.clone()
    }

    pub(crate) fn set_asset_state(
        &mut self,
        dyn_handle: &DynAssetHandle,
        state: InternalAssetState,
    ) {
        let Some(metadata) = self.get_asset_metadata_mut(dyn_handle) else {
            panic!("could not get asset metadata for {}", dyn_handle);
        };
        metadata.state = state;
    }

    pub(crate) fn set_just_available(&mut self, handle: DynAssetHandle) {
        self.just_available.insert(handle);
    }

    pub(crate) fn handle_just_available(&self, handle: &DynAssetHandle) -> bool {
        self.just_available.contains(handle)
    }

    pub(crate) fn clear_just_available(&mut self) {
        self.just_available.clear();
    }

    pub(crate) fn set_debug_name(&mut self, handle: &DynAssetHandle, name: impl Into<String>) {
        tracing::warn!("set debug name {}", handle);
        let Some(metadata) = self.get_asset_metadata_mut(handle) else {
            panic!("could not get asset entry for {}", handle);
        };

        metadata.debug_name = Some(name.into());
    }

    pub(crate) fn get_debug_name(&self, handle: &DynAssetHandle) -> Option<&str> {
        let Some(metadata) = self.get_asset_metadata(handle) else {
            panic!("could not get asset entry for {}", handle);
        };

        metadata.debug_name.as_deref()
    }

    pub(crate) fn get_all_assets(&self) -> Vec<DynAssetHandle> {
        self.metadata.keys().cloned().collect()
    }
}

//
// Typed/Dyn storage
//

struct AssetEntry<T: Asset> {
    asset: Option<Arc<T>>,
    get_requests: Vec<async_channel::Sender<Arc<T>>>,
}

struct TypedAssetStorage<T: Asset> {
    cache: FxHashMap<AssetHandle<T>, AssetEntry<T>>,
}

impl<T: Asset> TypedAssetStorage<T> {
    fn new() -> Self {
        Self {
            cache: FxHashMap::default(),
        }
    }
}

trait DynAssetStorage: ConditionalSend {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T: Asset> DynAssetStorage for TypedAssetStorage<T> {
    fn as_any(&self) -> &dyn Any {
        self as &dyn Any
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self as &mut dyn Any
    }
}
