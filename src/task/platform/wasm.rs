use crate::task::{DynTask, Task, TaskExecutorPlatformTrait};
use std::{future::Future, pin::Pin};

pub type TaskExecutorPlatform = WasmTaskExecutor;

pub struct WasmTaskExecutor {
    shutdown_sender: async_channel::Sender<()>,
}

impl TaskExecutorPlatformTrait for WasmTaskExecutor {
    /// Start the task executor
    ///
    /// Native: Spawn a new thread with an executor
    ///
    /// Wasm: Attach background loader to JS scheduler
    fn start(task_receiver: async_channel::Receiver<Box<dyn DynTask>>) -> Self {
        let (shutdown_sender, shutdown_receiver) = async_channel::bounded(1);
        wasm_bindgen_futures::spawn_local(super::task_runner(task_receiver, shutdown_receiver));

        Self { shutdown_sender }
    }

    fn check(&self) {}

    fn shutdown(self) {}
}
