use crate::task::TaskExecutorPlatformTrait;
use futures::{FutureExt, StreamExt};
use std::{any::Any, future::Future, pin::Pin};

pub type Task = Pin<Box<dyn Future<Output = ()> + Send + 'static>>;
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
            // TODO: should probably use better executor
            // pollster::block_on(Self::task_runner(task_receiver));
            // if the thread crashes crash the main program as well
            match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                pollster::block_on(task_runner(task_receiver, shutdown_receiver));
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
        if running_tasks.is_empty() {
            // when no assets are loading, only await new requests
            futures::select! {
                task = task_receiver.recv().fuse() => {
                    let task = task.expect("channel closed");
                    running_tasks.push(task);
                }

                _ = shutdown_receiver.recv().fuse() => {
                    return;
                }
            }
        } else {
            // TODO: should we fuse?

            // when assets are loading, await both assets and new requests
            futures::select! {
                task = task_receiver.recv().fuse() => {
                    let task = task.expect("could not receive task task");
                    running_tasks.push(task);
                }
                result = running_tasks.next().fuse() => {
                    // TODO : maybe expect here
                    if result.is_none() {
                        tracing::info!("finished loading all current load requests");
                    }
                }
                _ = shutdown_receiver.recv().fuse() => {
                    return;
                }
            }
        }
    }
}
