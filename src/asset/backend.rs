use crate::asset::{
    ArcAssetCacheRegistry, AssetCacheDependency, AssetCacheInsert, AssetCacheLoad, AssetCacheStorage, BackendToBackendMessage, BackendToFrontendMessage, DynAssetHandle, FrontendToBackendMessage
};
use crate::Context;
use futures::{select, FutureExt};
use tracing::{Instrument, span};

/// This is the owner of the asset cache state on the background thread
pub struct AssetCacheBackend {
    pub(crate) registry: ArcAssetCacheRegistry,

    pub(crate) storage: AssetCacheStorage,
    pub(crate) inserter: AssetCacheInsert,
    pub(crate) loader: AssetCacheLoad,
    pub(crate) dependency: AssetCacheDependency,

    pub(crate) backend_to_backend_sender: async_channel::Sender<BackendToBackendMessage>,
    pub(crate) backend_to_frontend_sender: async_channel::Sender<BackendToFrontendMessage>,
    pub(crate) frontend_to_backend_receiver: async_channel::Receiver<FrontendToBackendMessage>,
    pub(crate) backend_to_backend_receiver: async_channel::Receiver<BackendToBackendMessage>,

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) reloader: crate::asset::reload::AssetCacheReload,
}

impl AssetCacheBackend {
    pub(crate) fn new(
        ctx: &Context,
        registry: ArcAssetCacheRegistry,

        backend_to_backend_sender: async_channel::Sender<BackendToBackendMessage>,
        backend_to_frontend_sender: async_channel::Sender<BackendToFrontendMessage>,
        frontend_to_backend_receiver: async_channel::Receiver<FrontendToBackendMessage>,
        backend_to_backend_receiver: async_channel::Receiver<BackendToBackendMessage>,
    ) -> Self {
        let filesystem_runtime = ctx.filesystem.runtime();
        let render_runtime = ctx.render.runtime();
        let arc_runtime = ctx.arc.runtime();
        let task_executor = ctx.task.runtime();

        let storage = AssetCacheStorage::new();

        let inserter = AssetCacheInsert::new();
        let loader = AssetCacheLoad::new(
            task_executor.clone(),
            filesystem_runtime.clone(),
            render_runtime.clone(),
            arc_runtime.clone(),
            registry.clone(),
            backend_to_backend_sender.clone(),
        );

        let dependency = AssetCacheDependency::new();

        #[cfg(not(target_arch = "wasm32"))]
        let reloader =
            crate::asset::AssetCacheReload::new(filesystem_runtime.clone(), backend_to_backend_sender.clone());

        Self {
            storage,

            inserter,
            loader,

            registry,

            dependency,

            #[cfg(not(target_arch = "wasm32"))]
            reloader,

        backend_to_backend_sender,
        backend_to_frontend_sender,
        frontend_to_backend_receiver,
        backend_to_backend_receiver,
        }
    }

    fn get_all_handles_sorted(&self) -> Vec<DynAssetHandle> {
        let mut dyn_handles = self.storage.get_all_assets();

        // empty check
        if dyn_handles.is_empty() {
            tracing::warn!("trying to debug assets, but none exist");
        }

        // sort based on handle id
        dyn_handles.sort();

        dyn_handles
    }

    pub fn debug_assets(&self, show_dependencies: bool, show_recursive_dependencies: bool) {
        let root_handles = self.get_all_handles_sorted();

        for root_handle in root_handles {
            tracing::info!("");

            // print root
            let debug_name = self
                .storage
                .get_debug_name(&root_handle)
                .unwrap_or("no name");

            tracing::info!("{} {}", root_handle.id(), debug_name);

            let mut stack = Vec::new();

            if show_dependencies {
                if let Some(dependencies) = self.dependency.dependencies(&root_handle) {
                    let mut dependencies: Vec<_> = dependencies.iter().cloned().collect();
                    dependencies.sort();

                    // Dependencies are accessed LIFO and therefore the iteration order is reversed
                    // So do .rev() to undo this
                    for (i, dependency) in dependencies.iter().enumerate().rev() {
                        let is_last = i == dependencies.len() - 1;
                        let depth = 1; // hardcoded depth for root children
                        let prefix = String::new(); // no prefix from root
                        stack.push((dependency.clone(), prefix, depth, is_last));
                    }
                }
            }

            while let Some((dyn_handle, prefix, depth, is_last)) = stack.pop() {
                let debug_name = self
                    .storage
                    .get_debug_name(&dyn_handle)
                    .unwrap_or("no name");
                let id = dyn_handle.id();
                let connector = if is_last { "└── " } else { "├── " };

                tracing::info!("{}{}{} {}", prefix, connector, id, debug_name);

                if show_recursive_dependencies {
                    if let Some(dependencies) = self.dependency.dependencies(&dyn_handle) {
                        let mut dependencies: Vec<_> = dependencies.iter().cloned().collect();
                        dependencies.sort();

                        // Dependencies are accessed LIFO and therefore the iteration order is reversed
                        // So do .rev() to undo this
                        for (i, dependency) in dependencies.iter().enumerate().rev() {
                            // note this is_last refers to the currently popped/parent dependency
                            // if its not last you need a pipe going past the comming sub dependencies to the next dependency
                            let prefix = if is_last {
                                format!("{}    ", prefix).to_string()
                            } else {
                                format!("{}│   ", prefix).to_string()
                            };
                            let depth = depth + 1;
                            let is_last = i == dependencies.len() - 1;
                            stack.push((dependency.clone(), prefix, depth, is_last));
                        }
                    }
                }
            }
        }
    }
}

pub(crate) fn start_asset_background_thread(
    ctx: &Context,
    registry: ArcAssetCacheRegistry,
) -> (async_channel::Sender<FrontendToBackendMessage>, async_channel::Receiver<BackendToFrontendMessage>) {

    let (frontend_to_backend_sender, frontend_to_backend_receiver) = async_channel::unbounded();
    let (backend_to_backend_sender, backend_to_backend_receiver) = async_channel::unbounded();
    let (backend_to_frontend_sender, backend_to_frontend_receiver) = async_channel::unbounded();

    let mut cache = AssetCacheBackend::new(
        ctx,
        registry,
        backend_to_backend_sender,
        backend_to_frontend_sender.clone(),
        frontend_to_backend_receiver.clone(),
        backend_to_backend_receiver.clone(),
    );

    // TODO: the code done in the select branches could probably be moved inside backend
    ctx.task.runtime().spawn_task(async move {
        loop {
            select! {
                frontend_to_backend_message = frontend_to_backend_receiver.recv().fuse() => {
                    let frontend_to_backend_message = frontend_to_backend_message.expect("could not get frontend message");
                    match frontend_to_backend_message {
                        FrontendToBackendMessage::Load(load_request) => {
                            load_request.load_asset(
                                cache.backend_to_frontend_sender.clone(),
                                &cache.registry,
                                &mut cache.loader, 
                                &mut cache.storage
                            );
                        }
                        FrontendToBackendMessage::Insert(insert_request) => {
                            insert_request.insert_asset(
                                cache.backend_to_frontend_sender.clone(),
                                &cache.registry,
                                &mut cache.storage,
                                &mut cache.inserter,
                                &mut cache.loader,
                                &mut cache.dependency,
                            );
                        }
                        FrontendToBackendMessage::SetDebugName { handle, name } => {
                            cache.storage.set_debug_name(&handle, name);
                        }
                        FrontendToBackendMessage::DebugAssets { show_dependencies, show_recursive_dependencies } => {
                            cache.debug_assets(show_dependencies, show_recursive_dependencies);
                        }
                    }
                },
                backend_to_backend_message = backend_to_backend_receiver.recv().fuse() => {
                    let backend_to_backend_message = backend_to_backend_message.expect("could not get backend message");
                    match backend_to_backend_message {
                        BackendToBackendMessage::Get(get_request) => {
                            get_request.get_asset(
                                &mut cache.storage
                            );
                        },
                        BackendToBackendMessage::Load(load_request) => {
                            load_request.load_asset(
                                cache.backend_to_frontend_sender.clone(),
                                &cache.registry,
                                &mut cache.loader,
                                &mut cache.storage
                            );
                        },
                        BackendToBackendMessage::Insert(insert_request) => {
                            insert_request.insert_asset(
                                cache.backend_to_frontend_sender.clone(),
                                &cache.registry,
                                &mut cache.storage,
                                &mut cache.inserter,
                                &mut cache.loader,
                                &mut cache.dependency,
                            );
                        },
                        BackendToBackendMessage::Response(response_request) => {
                            response_request.handle_asset_load_response(
                                cache.backend_to_frontend_sender.clone(),
                                &cache.registry,
                                &mut cache.storage,
                                &mut cache.loader,
                                &mut cache.dependency,
                                #[cfg(not(target_arch = "wasm32"))] 
                                &mut cache.reloader,
                            );
                        },
                        BackendToBackendMessage::SetDebugName{handle, name} => {
                            cache.storage.set_debug_name(&handle, name);
                        },
                        #[cfg(not(target_arch = "wasm32"))] 
                        BackendToBackendMessage::Reload(reload_request) => {
                            cache.reloader.handle_path_changed(
                                &reload_request.path,
                                &cache.registry,
                                &mut cache.loader,
                                &mut cache.storage
                            );
                        },
                    }
                }
            }
        }
    }.instrument(tracing::info_span!("backend"))
    );

    (frontend_to_backend_sender, backend_to_frontend_receiver)
}

