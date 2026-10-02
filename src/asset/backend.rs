use crate::asset::{
    ArcAssetCacheRegistry, AssetCacheDependency, AssetCacheInsert, AssetCacheLoad, AssetCacheStorage, BackendToBackendMessage, BackendToFrontendMessage, DynAssetHandle, DynLoadResult, FrontendToBackendMessage
};
use crate::Context;
use crate::task::Task;
use futures::StreamExt;
use futures::stream::FuturesUnordered;

/// This is the owner of the asset cache state on the background thread
pub struct AssetCacheBackend {
    pub(crate) registry: ArcAssetCacheRegistry,

    pub(crate) storage: AssetCacheStorage,
    pub(crate) inserter: AssetCacheInsert,
    pub(crate) loader: AssetCacheLoad,
    pub(crate) dependency: AssetCacheDependency,

    pub(crate) backend_to_backend_sender: async_channel::Sender<BackendToBackendMessage>,
    pub(crate) backend_to_backend_receiver: async_channel::Receiver<BackendToBackendMessage>,
    pub(crate) backend_to_frontend_sender: async_channel::Sender<BackendToFrontendMessage>,
    pub(crate) backend_to_frontend_receiver: async_channel::Receiver<BackendToFrontendMessage>,
    pub(crate) frontend_to_backend_sender: async_channel::Sender<FrontendToBackendMessage>,
    pub(crate) frontend_to_backend_receiver: async_channel::Receiver<FrontendToBackendMessage>,

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) reloader: crate::asset::reload::AssetCacheReload,
}

impl AssetCacheBackend {
    pub(crate) fn new(
        ctx: &Context,
        registry: ArcAssetCacheRegistry,
    ) -> Self {
        let (frontend_to_backend_sender, frontend_to_backend_receiver) = async_channel::unbounded();
        let (backend_to_backend_sender, backend_to_backend_receiver) = async_channel::unbounded();
        let (backend_to_frontend_sender, backend_to_frontend_receiver) = async_channel::unbounded();

        let storage = AssetCacheStorage::new();

        let inserter = AssetCacheInsert::new();
        let loader = AssetCacheLoad::new(
            ctx.task.runtime(),
            ctx.filesystem.runtime(),
            ctx.render.runtime(),
            ctx.arc.runtime(),
            registry.clone(),
            backend_to_backend_sender.clone(),
        );

        let dependency = AssetCacheDependency::new();

        #[cfg(not(target_arch = "wasm32"))]
        let reloader = crate::asset::AssetCacheReload::new(
            ctx.filesystem.runtime(),
            backend_to_backend_sender.clone(),
        );

        Self {
            storage,

            inserter,
            loader,

            registry,

            dependency,

            #[cfg(not(target_arch = "wasm32"))]
            reloader,

            backend_to_backend_sender,
            backend_to_backend_receiver,
            backend_to_frontend_sender,
            backend_to_frontend_receiver,
            frontend_to_backend_sender,
            frontend_to_backend_receiver,
        }
    }

    pub(crate) fn run(mut self, ctx: &Context) -> (
        async_channel::Sender<FrontendToBackendMessage>,
        async_channel::Receiver<BackendToFrontendMessage>,
        ) 
    {
        let mut loads = FuturesUnordered::new();

        let frontend_to_backend_sender = self.frontend_to_backend_sender.clone();
        let backend_to_frontend_receiver = self.backend_to_frontend_receiver.clone();

        ctx.task.runtime().spawn_task(Task::new(|| async move {
            loop {
                tokio::select! {
                    // progress current loads
                    load_result = loads.next(), if !loads.is_empty() => {
                        let load_result: Box<dyn DynLoadResult> = load_result.expect("result from running task was none");
                        let load_futures = load_result.handle_load_result(
                            self.backend_to_frontend_sender.clone(), 
                            &self.registry, 
                            &mut self.storage, 
                            &mut self.loader, 
                            &mut self.dependency, 
                            #[cfg(not(target_arch = "wasm32"))]
                            &mut self.reloader
                        );
                        for load_future in load_futures {
                            loads.push(load_future)
                        }
                    },
                    // receive frontend request messages
                    frontend_to_backend_message = self.frontend_to_backend_receiver.recv() => {
                        let frontend_to_backend_message = frontend_to_backend_message.expect("could not get frontend message");
                        match frontend_to_backend_message {
                            FrontendToBackendMessage::Load(load_request) => {
                                let load_future = load_request.load_asset_if_needed(
                                    self.backend_to_frontend_sender.clone(),
                                    &self.registry,
                                    &mut self.loader, 
                                    &mut self.storage
                                );
                                if let Some(load_future) = load_future {
                                    loads.push(load_future);
                                }
                            }
                            FrontendToBackendMessage::Insert(insert_request) => {
                                let load_futures = insert_request.insert_asset(
                                    self.backend_to_frontend_sender.clone(),
                                    &self.registry,
                                    &mut self.storage,
                                    &mut self.inserter,
                                    &mut self.loader,
                                    &mut self.dependency,
                                );
                                for load_future in load_futures {
                                    loads.push(load_future)
                                }
                            }
                            FrontendToBackendMessage::SetDebugName { handle, name } => {
                                self.storage.set_debug_name(&handle, name);
                            }
                            FrontendToBackendMessage::DebugAssets { show_dependencies, show_recursive_dependencies } => {
                                self.debug_assets(show_dependencies, show_recursive_dependencies);
                            }
                        }
                    },
                    // receive internal request messages
                    backend_to_backend_message = self.backend_to_backend_receiver.recv() => {
                        let backend_to_backend_message = backend_to_backend_message.expect("could not get backend message");
                        match backend_to_backend_message {
                            BackendToBackendMessage::Get(get_request) => {
                                get_request.get_asset(
                                    &mut self.storage
                                );
                            },
                            BackendToBackendMessage::Load(load_request) => {
                                let load_future = load_request.load_asset_if_needed(
                                    self.backend_to_frontend_sender.clone(),
                                    &self.registry,
                                    &mut self.loader,
                                    &mut self.storage
                                );
                                if let Some(load_future) = load_future {
                                    loads.push(load_future);
                                }
                            },
                            BackendToBackendMessage::Insert(insert_request) => {
                                let load_futures = insert_request.insert_asset(
                                    self.backend_to_frontend_sender.clone(),
                                    &self.registry,
                                    &mut self.storage,
                                    &mut self.inserter,
                                    &mut self.loader,
                                    &mut self.dependency,
                                );
                                for load_future in load_futures {
                                    loads.push(load_future)
                                }
                            },
                            BackendToBackendMessage::SetDebugName{handle, name} => {
                                self.storage.set_debug_name(&handle, name);
                            },
                            #[cfg(not(target_arch = "wasm32"))] 
                            BackendToBackendMessage::Reload(reload_request) => {
                                let load_futures = self.reloader.handle_path_changed(
                                    &reload_request.path,
                                    &self.registry,
                                    &mut self.loader,
                                    &mut self.storage
                                );
                                for load_future in load_futures {
                                    loads.push(load_future);

                                }
                            },
                        }
                    }
                }
            }
        }));

        (frontend_to_backend_sender, backend_to_frontend_receiver)
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
