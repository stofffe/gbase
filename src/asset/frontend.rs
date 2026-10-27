use std::sync::Arc;

use super::{Asset, AssetLoader};
use crate::{
    asset::{
        ArcAssetCacheRegistry, AssetCacheBackend, AssetCacheSnapshot, AssetHandle, AssetInserter,
        AssetMetadata, BackendMessage, DebugAssetsSettings, DynAssetHandle, GetAssetState,
        InternalAssetState, TypedInsertRequest, TypedLoadRequest,
    },
    ConditionalSend, Context,
};
use rustc_hash::FxHashSet;

pub(crate) enum FrontendMessage {
    AssetUpdate(Box<dyn DynFrontendAssetLoaded>),
    AssetMetadataUpdate(MetadataChanged),
}

pub(crate) trait DynFrontendAssetLoaded: ConditionalSend {
    fn insert_snapshot(self: Box<Self>, storage: &mut AssetCacheSnapshot);
    fn handle(&self) -> DynAssetHandle;
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
    fn insert_snapshot(self: Box<Self>, storage: &mut AssetCacheSnapshot) {
        storage.set_asset(self.handle, self.asset);
    }

    fn handle(&self) -> DynAssetHandle {
        self.handle.to_dyn()
    }
}

pub(crate) struct MetadataChanged {
    pub(crate) handle: DynAssetHandle,
    pub(crate) metadata: AssetMetadata,
}

/// Holds snapshots of the backend asset system
///
/// Can send request messages to backend asset system
///
/// Exposed to the user
pub struct AssetCache {
    registry: ArcAssetCacheRegistry,
    snapshot: AssetCacheSnapshot,

    just_available: FxHashSet<DynAssetHandle>,

    backend_message_sender: async_channel::Sender<BackendMessage>,
    frontend_message_receiver: async_channel::Receiver<FrontendMessage>,
}

impl AssetCache {
    pub(crate) fn new(ctx: &Context) -> Self {
        let registry = ArcAssetCacheRegistry::new();
        let backend = AssetCacheBackend::new(ctx, registry.clone());
        let (backend_message_sender, frontend_message_receiver) = backend.run(ctx);
        let snapshot = AssetCacheSnapshot::new();
        Self {
            registry,
            backend_message_sender,
            frontend_message_receiver,
            snapshot,
            just_available: FxHashSet::default(),
        }
    }

    pub(crate) fn poll(&mut self) {
        self.just_available.clear();

        while let Ok(frontend_message_message) = self.frontend_message_receiver.try_recv() {
            match frontend_message_message {
                FrontendMessage::AssetUpdate(asset_update) => {
                    self.just_available.insert(asset_update.handle());
                    tracing::info!("SNAPSHOT asset {}", asset_update.handle());
                    asset_update.insert_snapshot(&mut self.snapshot);
                }
                FrontendMessage::AssetMetadataUpdate(metadata_update) => {
                    tracing::info!("SNAPSHOT metadata {}", metadata_update.handle);
                    self.snapshot
                        .set_metadata(metadata_update.handle, metadata_update.metadata);
                }
            }
        }
    }

    /// Insert an asset and reuse any handles matching the same key
    pub fn insert_asset<T: Asset + 'static, I: AssetInserter + 'static>(
        &mut self,
        key: impl Into<I::Key>,
        asset: T,
    ) -> AssetHandle<T> {
        let handle = self.registry.get_insert_handle::<T, I>(key.into(), None);

        tracing::info!("send insert asset request {}", handle);
        self.backend_message_sender
            .try_send(BackendMessage::Insert(Box::new(
                TypedInsertRequest::<T>::new(handle.clone(), asset, None),
            )))
            .expect("could not send insert request");

        handle
    }

    /// Insert an asset without checking for cached handles
    pub fn insert_asset_force<T: Asset + 'static>(&mut self, asset: T) -> AssetHandle<T> {
        let handle = self.registry.get_empty_handle::<T>();

        tracing::info!("send insert asset request {}", handle);
        self.backend_message_sender
            .try_send(BackendMessage::Insert(Box::new(
                TypedInsertRequest::<T>::new(handle.clone(), asset, None),
            )))
            .expect("could not send insert request");

        handle
    }

    /// Request an asset load
    pub fn load_asset<T: AssetLoader + 'static>(
        &mut self,
        settings: &T::Settings,
    ) -> AssetHandle<T::Asset> {
        // TODO: register in storage as well?
        let handle = self.registry.get_load_handle::<T>(settings);

        // check snapshot if sending request is needed
        match self
            .snapshot
            .get_metadata(&handle.to_dyn())
            .map(|metadata| metadata.state.clone())
        {
            None | Some(InternalAssetState::Cleared) => {
                tracing::info!("send load asset request {}", handle);
                self.backend_message_sender
                    .try_send(BackendMessage::Load(Box::new(TypedLoadRequest::<T>::new(
                        handle.clone(),
                        None,
                    ))))
                    .expect("could not send load request from frontend");
            }
            _ => (),
        };

        handle
    }

    pub fn get_asset_cloned<T: Asset + Clone + 'static>(
        &mut self,
        handle: &AssetHandle<T>,
    ) -> Result<T, GetAssetState> {
        self.get_asset(handle).cloned()
    }

    /// Get an asset
    pub fn get_asset<T: Asset + 'static>(
        &mut self,
        handle: &AssetHandle<T>,
    ) -> Result<&T, GetAssetState> {
        if let Some(asset) = self.snapshot.get_asset(handle) {
            return Ok(asset);
        }

        let Some(metadata) = self.snapshot.get_metadata(&handle.to_dyn()) else {
            // tracing::warn!(
            //     "trying to get metadata which doesnt exist for {}, returning loading",
            //     handle
            // );
            return Err(GetAssetState::Loading);
        };

        match metadata.state {
            InternalAssetState::Loading => {
                tracing::info!("waiting for {}", handle);
                Err(GetAssetState::Loading)
            }
            InternalAssetState::Failed => {
                tracing::info!("error in {}", handle);
                Err(GetAssetState::Failed)
            }
            InternalAssetState::Cleared => {
                tracing::info!("{} is cleared", handle);
                Err(GetAssetState::Loading)
            }
            InternalAssetState::Ready => {
                panic!(
                    "could not get asset from storage but status is ready {}",
                    handle
                );
            }
        }
    }

    /// Returns wheter a handle is available for reading
    pub fn handle_available<T: Asset>(&mut self, handle: &AssetHandle<T>) -> bool {
        match self.snapshot.get_metadata(&handle.to_dyn()) {
            Some(metadata) => matches!(metadata.state, InternalAssetState::Ready),
            None => false,
        }
    }

    pub fn clear_handle<T: Asset>(&mut self, handle: &AssetHandle<T>) {
        todo!()
        // self.storage.clear_asset::<T>(handle);
        // self.storage
        //     .set_asset_state(&handle.to_dyn(), InternalAssetState::Pending);
        // // TODO: probably more
    }

    // TODO:  is this needed?
    // can check arc strong count
    // might not work since handles can depend on eachother
    // maybe use weak references in certain cases
    pub fn clear_unused_handles(&mut self) {
        todo!()
        // self.storage.clear_unused_handles(&mut self.derived);
    }

    pub fn set_debug_name<T: Asset>(&mut self, handle: &AssetHandle<T>, name: impl Into<String>) {
        let name = name.into();
        tracing::info!("send set debug name request {}: {}", handle, name);
        self.backend_message_sender
            .try_send(BackendMessage::SetDebugName {
                handle: handle.to_dyn(),
                name,
            })
            .expect("could not send set debug name message");
    }

    pub fn get_debug_name<T: Asset>(&self, handle: &AssetHandle<T>) -> Option<&str> {
        self.snapshot
            .get_metadata(&handle.to_dyn())
            .and_then(|metadata| metadata.debug_name.as_deref())
    }

    pub fn debug_asset_dependency_graph(&self) {
        todo!()
        // self.dependency.debug_dependencies(&self.storage);
    }

    /// Return if the handle became ready this frame
    pub fn handle_just_available<T: Asset>(&self, handle: &AssetHandle<T>) -> bool {
        self.just_available.contains(&handle.to_dyn())
    }

    /// Reload an existing asset while reusing the last path and loader
    #[cfg(not(target_arch = "wasm32"))]
    pub fn reload<T: Asset + 'static>(&mut self, handle: &AssetHandle<T>) {
        todo!()
        // self.reloader
        //     .reload(handle.to_dyn(), &mut self.loader, &mut self.storage);
    }

    pub fn debug_assets(&self, settings: DebugAssetsSettings) {
        self.backend_message_sender
            .try_send(BackendMessage::DebugAssets(settings))
            .expect("could not send debug assets message");
    }
}
