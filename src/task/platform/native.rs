use crate::task::{self, TaskExecutorPlatformTrait};
use std::{any::Any, future::Future, pin::Pin};

pub type Task = Pin<Box<dyn Future<Output = ()> + Send + 'static>>;
pub type TaskExecutorPlatform = NativeTaskExecutor;

pub struct NativeTaskExecutor {
    thread_handle: std::thread::JoinHandle<()>,
    thread_panic_receiver: async_channel::Receiver<Box<dyn Any + Send>>,
}

impl TaskExecutorPlatformTrait for NativeTaskExecutor {
    /// Start the task executor
    ///
    /// Native: Spawn a new thread with an executor
    ///
    /// Wasm: Attach background loader to JS scheduler
    fn start(task_receiver: async_channel::Receiver<Task>) -> Self {
        let (thread_panic_sender, thread_panic_receiver) = async_channel::bounded(1);

        let thread_handle = std::thread::spawn(move || {
            // TODO: should probably use better executor
            // pollster::block_on(Self::task_runner(task_receiver));
            // if the thread crashes crash the main program as well
            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                pollster::block_on(task::task_runner(task_receiver));
            })) {
                Ok(_) => {}
                Err(err) => thread_panic_sender
                    .try_send(err)
                    .expect("could not send task thread panic message"),
            };
        });

        Self {
            thread_handle,
            thread_panic_receiver,
        }
    }

    fn check(&self) {
        if let Ok(err) = self.thread_panic_receiver.try_recv() {
            tracing::error!("task thread crashed");
            std::panic::resume_unwind(err)
        }
    }

    fn shutdown(self) {
        todo!()
    }
}
