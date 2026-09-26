use crate::task::{Task, TaskExecutorPlatformTrait, TaskTrait};
use futures::StreamExt;
use std::{future::Future, pin::Pin};

pub type TaskExecutorPlatform = WasmTaskExecutor;

pub struct WasmTaskExecutor {}

impl TaskExecutorPlatformTrait for WasmTaskExecutor {
    /// Start the task executor
    ///
    /// Native: Spawn a new thread with an executor
    ///
    /// Wasm: Attach background loader to JS scheduler
    fn start(task_receiver: async_channel::Receiver<Task>) -> Self {
        wasm_bindgen_futures::spawn_local(task_runner(task_receiver));

        Self {}
    }

    fn check(&self) {}

    fn shutdown(self) {}
}

/// Implementation of an asset loader that runs in the background
///
/// Should be started using `start_background_loader`
pub(super) async fn task_runner(task_receiver: async_channel::Receiver<Task>) {
    let mut running_tasks = futures::stream::FuturesUnordered::new();

    loop {
        // when assets are loading, await both assets and new requests
        tokio::select! {
            task = task_receiver.recv() => {
                let task = task.expect("could not receive task task");
                running_tasks.push(task);
            }
            result = running_tasks.next(), if !running_tasks.is_empty() => {
                // TODO : maybe expect here
                if result.is_none() {
                    tracing::info!("finished loading all current load requests");
                }
            }
        }
    }
}
