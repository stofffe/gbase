use crate::task::{Task, TaskExecutorPlatformTrait};
use futures::StreamExt;
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
    fn start(task_receiver: async_channel::Receiver<Task>) -> Self {
        let (thread_panic_sender, thread_panic_receiver) = async_channel::bounded(1);
        let (shutdown_sender, shutdown_receiver) = async_channel::bounded(1);

        let thread_handle = std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("could not create tokio runtime");

            // TODO: should probably use better executor
            // pollster::block_on(Self::task_runner(task_receiver));
            // if the thread crashes crash the main program as well
            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                runtime.block_on(task_runner(task_receiver, shutdown_receiver));
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
    task_receiver: async_channel::Receiver<Task>,
    shutdown_receiver: async_channel::Receiver<()>,
) {
    let mut running_tasks = futures::stream::FuturesUnordered::new();

    loop {
        tokio::select! {
            task = task_receiver.recv() => {
                let task = task.expect("could not receive task task");
                running_tasks.push(task);
            }
            task_result = running_tasks.next(), if !running_tasks.is_empty() => {
                task_result.expect("result from running task was none");
            }
            _ = shutdown_receiver.recv() => {
                return;
            }
        }
    }
}
