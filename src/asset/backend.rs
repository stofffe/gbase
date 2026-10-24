use crate::asset::{
    ArcAssetCacheRegistry, AssetCacheDependency, AssetCacheInsert, AssetCacheLoad,
    AssetCacheStorage, BoxedAssetLoadFuture, DynAssetHandle, DynGetRequest, DynInsertRequest,
    DynLoadRequest, DynLoadResult, FrontendMessage,
};
use crate::task::Task;
use crate::Context;

pub(crate) enum BackendMessage {
    Get(Box<dyn DynGetRequest>),
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
    LoadResult(Box<dyn DynLoadResult>),
    #[cfg(feature = "asset_reload")]
    Reload(crate::asset::ReloadHandleRequest),
}

/// This is the owner of the asset cache state on the background thread
pub struct AssetCacheBackend {
    pub(crate) registry: ArcAssetCacheRegistry,

    pub(crate) storage: AssetCacheStorage,
    pub(crate) inserter: AssetCacheInsert,
    pub(crate) loader: AssetCacheLoad,
    pub(crate) dependency: AssetCacheDependency,

    pub(crate) backend_message_sender: async_channel::Sender<BackendMessage>,
    pub(crate) backend_message_receiver: async_channel::Receiver<BackendMessage>,
    pub(crate) frontend_message_sender: async_channel::Sender<FrontendMessage>,
    pub(crate) frontend_message_receiver: async_channel::Receiver<FrontendMessage>,

    #[cfg(feature = "asset_reload")]
    pub(crate) reloader: crate::asset::reload::AssetCacheReload,
}

impl AssetCacheBackend {
    pub(crate) fn new(ctx: &Context, registry: ArcAssetCacheRegistry) -> Self {
        let (frontend_message_sender, frontend_message_receiver) = async_channel::unbounded();
        let (backend_message_sender, backend_message_receiver) = async_channel::unbounded();

        let storage = AssetCacheStorage::new();

        let inserter = AssetCacheInsert::new();
        let loader = AssetCacheLoad::new(
            ctx.task.runtime(),
            ctx.filesystem.runtime(),
            ctx.render.runtime(),
            ctx.arc.runtime(),
            registry.clone(),
            backend_message_sender.clone(),
        );

        let dependency = AssetCacheDependency::new();

        #[cfg(feature = "asset_reload")]
        let reloader = crate::asset::AssetCacheReload::new(
            ctx.filesystem.runtime(),
            backend_message_sender.clone(),
        );

        Self {
            storage,

            inserter,
            loader,

            registry,

            dependency,

            #[cfg(feature = "asset_reload")]
            reloader,

            backend_message_sender,
            backend_message_receiver,
            frontend_message_sender,
            frontend_message_receiver,
        }
    }

    pub(crate) fn run(
        mut self,
        ctx: &Context,
    ) -> (
        async_channel::Sender<BackendMessage>,
        async_channel::Receiver<FrontendMessage>,
    ) {
        let backend_message_sender = self.backend_message_sender.clone();
        let frontend_message_receiver = self.frontend_message_receiver.clone();

        ctx.task.runtime().spawn_task(Task::new(|| async move {
            let asset_executor = async_executor::LocalExecutor::new();

            fn spawn_load(
                asset_executor: &async_executor::LocalExecutor<'_>,
                backend_message_sender: async_channel::Sender<BackendMessage>,
                load_future: BoxedAssetLoadFuture,
            ) {
                asset_executor
                    .spawn(async move {
                        let result = load_future.await;
                        backend_message_sender
                            .send(BackendMessage::LoadResult(result))
                            .await
                            .expect("could not send load asset result");
                    })
                    .detach();
            }

            asset_executor
                .run(async {
                    while let Ok(backend_message) = self.backend_message_receiver.recv().await {
                        match backend_message {
                            BackendMessage::Load(load_request) => {
                                let load_future = load_request.load_asset_if_needed(
                                    self.frontend_message_sender.clone(),
                                    &self.registry,
                                    &mut self.loader,
                                    &mut self.storage,
                                );
                                if let Some(load_future) = load_future {
                                    spawn_load(
                                        &asset_executor,
                                        self.backend_message_sender.clone(),
                                        load_future,
                                    );
                                }
                            }
                            BackendMessage::Insert(insert_request) => {
                                let load_futures = insert_request.insert_asset(
                                    self.frontend_message_sender.clone(),
                                    &self.registry,
                                    &mut self.storage,
                                    &mut self.inserter,
                                    &mut self.loader,
                                    &mut self.dependency,
                                );
                                for load_future in load_futures {
                                    spawn_load(
                                        &asset_executor,
                                        self.backend_message_sender.clone(),
                                        load_future,
                                    );
                                }
                            }
                            BackendMessage::SetDebugName { handle, name } => {
                                self.storage.set_debug_name(&handle, name);
                            }
                            BackendMessage::DebugAssets {
                                show_dependencies,
                                show_recursive_dependencies,
                            } => {
                                self.debug_assets(show_dependencies, show_recursive_dependencies);
                            }

                            BackendMessage::Get(dyn_get_request) => {
                                dyn_get_request.get_asset(&mut self.storage)
                            }
                            BackendMessage::LoadResult(load_result) => {
                                let load_futures = load_result.handle_load_result(
                                    self.frontend_message_sender.clone(),
                                    &self.registry,
                                    &mut self.storage,
                                    &mut self.loader,
                                    &mut self.dependency,
                                    #[cfg(feature = "asset_reload")]
                                    &mut self.reloader,
                                );
                                for load_future in load_futures {
                                    spawn_load(
                                        &asset_executor,
                                        self.backend_message_sender.clone(),
                                        load_future,
                                    );
                                }
                            }
                            #[cfg(feature = "asset_reload")]
                            BackendMessage::Reload(reload_handle_request) => {
                                let load_futures = self.reloader.handle_path_changed(
                                    &reload_handle_request.path,
                                    &self.registry,
                                    &mut self.loader,
                                    &mut self.storage,
                                );
                                for load_future in load_futures {
                                    spawn_load(
                                        &asset_executor,
                                        self.backend_message_sender.clone(),
                                        load_future,
                                    );
                                }
                            }
                        }
                    }
                })
                .await;
        }));

        (backend_message_sender, frontend_message_receiver)
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

    fn asset_info(&self, handle: &DynAssetHandle) -> String {
        let no_name_replacement = "___";
        let asset_name = self
            .storage
            .get_debug_name(handle)
            .unwrap_or(no_name_replacement);
        let state = self.storage.get_asset_state(handle);
        format!("{}: {:?}", asset_name, state)
    }

    pub fn debug_assets(&self, show_dependencies: bool, show_recursive_dependencies: bool) {
        let root_handles = self.get_all_handles_sorted();

        for root_handle in root_handles {
            tracing::info!("");

            // print root

            let debug_name = self.asset_info(&root_handle);

            tracing::info!("{} {}", root_handle.id(), debug_name);

            // print dependencies

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
                let debug_name = self.asset_info(&dyn_handle);
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
