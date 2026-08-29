use crate::task::{self, TaskExecutorPlatformTrait};
use std::{future::Future, pin::Pin};

pub type Task = Pin<Box<dyn Future<Output = ()> + Send + 'static>>;
pub type TaskExecutorPlatform = NativeTaskExecutor;

pub struct NativeTaskExecutor {}

impl TaskExecutorPlatformTrait for NativeTaskExecutor {
    /// Start the task executor
    ///
    /// Native: Spawn a new thread with an executor
    ///
    /// Wasm: Attach background loader to JS scheduler
    fn start(task_receiver: async_channel::Receiver<Task>) -> Self {
        let thread_handle = std::thread::spawn(move || {
            // TODO: should probably use better executor
            // pollster::block_on(Self::task_runner(task_receiver));

            // if the thread crashes crash the main program as well
            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                pollster::block_on(task::task_runner(task_receiver));
            })) {
                Ok(_) => {}
                Err(_) => std::process::abort(),
            };
        });

        Self {}
    }

    fn shutdown() {
        todo!()
    }
}
