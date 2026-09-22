use std::sync::Arc;

use crate::{
    asset::{
        Asset, AssetCacheFrontendStorage, AssetHandle, AssetMetadata, DynAssetHandle,
        DynGetRequest, DynInsertRequest, DynLoadRequest, DynLoadResponse,
    },
    ConditionalSend,
};

pub(crate) enum FrontendToBackendMessage {
    Load(Box<dyn DynLoadRequest>),
    Insert(Box<dyn DynInsertRequest>),
    SetDebugName {
        handle: DynAssetHandle,
        name: String,
    },
    DebugAssets {
        show_dependencies: bool,
        show_recursive_dependencies: bool,
    },
}

pub(crate) enum BackendToFrontendMessage {
    AssetUpdate(Box<dyn DynFrontendAssetLoaded>),
    AssetMetadataUpdate(MetadataChanged),
}

pub(crate) enum BackendToBackendMessage {
    Get(Box<dyn DynGetRequest>),
    Load(Box<dyn DynLoadRequest>),
    Insert(Box<dyn DynInsertRequest>),
    Response(Box<dyn DynLoadResponse>),
    SetDebugName {
        handle: DynAssetHandle,
        name: String,
    },
    // TODO: this should maybe not live in here
    // more of a path changed message as well
    #[cfg(not(target_arch = "wasm32"))]
    Reload(crate::asset::ReloadHandleRequest),
}

pub(crate) trait DynFrontendAssetLoaded: ConditionalSend {
    fn insert(self: Box<Self>, storage: &mut AssetCacheFrontendStorage);
}

pub(crate) struct TypedFrontendAssetLoaded<T: Asset> {
    handle: AssetHandle<T>,
    asset: Arc<T>,
}

impl<T: Asset> TypedFrontendAssetLoaded<T> {
    pub fn new(handle: AssetHandle<T>, asset: Arc<T>) -> Self {
        Self { handle, asset }
    }
}

impl<T: Asset> DynFrontendAssetLoaded for TypedFrontendAssetLoaded<T> {
    fn insert(self: Box<Self>, storage: &mut AssetCacheFrontendStorage) {
        tracing::info!("SNAPSHOT insert asset {}", self.handle);
        storage.set_asset(self.handle, self.asset);
    }
}

pub(crate) struct MetadataChanged {
    pub(crate) handle: DynAssetHandle,
    pub(crate) metadata: AssetMetadata,
}
