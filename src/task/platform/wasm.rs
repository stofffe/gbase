use crate::task::{self, TaskExecutorPlatformTrait};
use std::{future::Future, pin::Pin};

pub type Task = Pin<Box<dyn Future<Output = ()> + 'static>>;
pub type TaskExecutorPlatform = WasmTaskExecutor;

pub struct WasmTaskExecutor {}

impl TaskExecutorPlatformTrait for WasmTaskExecutor {
    /// Start the task executor
    ///
    /// Native: Spawn a new thread with an executor
    ///
    /// Wasm: Attach background loader to JS scheduler
    fn start(task_receiver: async_channel::Receiver<Task>) -> Self {
        wasm_bindgen_futures::spawn_local(task::task_runner(task_receiver));

        Self {}
    }

    fn shutdown() {}
}
