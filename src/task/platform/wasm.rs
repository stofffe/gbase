use crate::task::{DynTask, Task, TaskExecutorPlatformTrait};
use futures_concurrency::future::Race;
use std::{future::Future, pin::Pin};

pub type TaskExecutorPlatform = WasmTaskExecutor;

pub struct WasmTaskExecutor {}

impl TaskExecutorPlatformTrait for WasmTaskExecutor {
    /// Start the task executor
    ///
    /// Native: Spawn a new thread with an executor
    ///
    /// Wasm: Attach background loader to JS scheduler
    fn start(task_receiver: async_channel::Receiver<Box<dyn DynTask>>) -> Self {
        wasm_bindgen_futures::spawn_local(task_runner(task_receiver));

        Self {}
    }

    fn check(&self) {}

    fn shutdown(self) {}
}

/// Implementation of an asset loader that runs in the background
///
/// Should be started using `start_background_loader`
pub(super) async fn task_runner(task_receiver: async_channel::Receiver<Box<dyn DynTask>>) {
    let task_executor = async_executor::LocalExecutor::new();

    loop {
        let selected_futures = (
            async {
                let task = task_receiver
                    .recv()
                    .await
                    .expect("could not receive from task channel");
                task_executor.spawn(task.into_future()).detach();
            },
            async {
                task_executor.tick().await;
            },
        );
        selected_futures.race().await;
    }
}
