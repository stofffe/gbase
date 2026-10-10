use crate::task::{DynTask, TaskExecutorPlatformTrait};
use futures_concurrency::future::Race;
use std::any::Any;

pub type TaskExecutorPlatform = NativeTaskExecutor;

pub struct NativeTaskExecutor {
    thread_handle: std::thread::JoinHandle<()>,
    thread_panic_receiver: async_channel::Receiver<Box<dyn Any + Send>>,
    shutdown_sender: async_channel::Sender<()>,
}

impl TaskExecutorPlatformTrait for NativeTaskExecutor {
    /// Start the task executor
    ///
    /// Native: Spawn a new thread with an executor
    ///
    /// Wasm: Attach background loader to JS scheduler
    fn start(task_receiver: async_channel::Receiver<Box<dyn DynTask>>) -> Self {
        let (thread_panic_sender, thread_panic_receiver) = async_channel::bounded(1);
        let (shutdown_sender, shutdown_receiver) = async_channel::bounded(1);

        let thread_handle = std::thread::spawn(move || {
            // if the thread crashes crash the main program as well
            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                futures_lite::future::block_on(task_runner(task_receiver, shutdown_receiver));
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
            shutdown_sender,
        }
    }

    fn check(&self) {
        if let Ok(err) = self.thread_panic_receiver.try_recv() {
            tracing::error!("task thread crashed");
            std::panic::resume_unwind(err)
        }
    }

    fn shutdown(self) {
        tracing::info!("task thread shutting down");

        self.shutdown_sender
            .send_blocking(())
            .expect("could not send shutdown message to task thread");

        match self.thread_handle.join() {
            Ok(_) => (),
            Err(err) => std::panic::resume_unwind(err),
        }
        tracing::info!("task thread shut down successfully");
    }
}

/// Implementation of an asset loader that runs in the background
///
/// Should be started using `start_background_loader`
pub(super) async fn task_runner(
    task_receiver: async_channel::Receiver<Box<dyn DynTask>>,
    shutdown_receiver: async_channel::Receiver<()>,
) {
    let task_executor = async_executor::LocalExecutor::new();

    loop {
        let selected_futures = (
            async {
                task_executor.tick().await;
                false
            },
            async {
                let task = task_receiver
                    .recv()
                    .await
                    .expect("could not receive from task channel");
                task_executor.spawn(task.into_future()).detach();
                false
            },
            async {
                shutdown_receiver
                    .recv()
                    .await
                    .expect("could not receive from shutdown channel");
                true
            },
        );

        let should_shutdown = selected_futures.race().await;

        if should_shutdown {
            break;
        }
    }
}
