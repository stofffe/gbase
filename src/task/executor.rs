use futures::{FutureExt, StreamExt};

use crate::task::Task;

// TODO: make a spawner type thing without the receiver
#[derive(Clone)]
pub struct TaskExecutor {
    task_sender: async_channel::Sender<Task>,
    task_receiver: async_channel::Receiver<Task>,
}

impl TaskExecutor {
    pub fn new() -> Self {
        let (task_sender, task_receiver) = async_channel::unbounded();
        Self {
            task_sender,
            task_receiver,
        }
    }

    pub fn spawn(&self, task: Task) {
        self.task_sender
            .try_send(task)
            .expect("could not send task");
    }

    /// Start the task executor
    ///
    /// Native: Spawn a new thread with an executor
    ///
    /// Wasm: Attach background loader to JS scheduler
    pub(crate) fn start(&self) {
        let task_receiver = self.task_receiver.clone();

        #[cfg(not(target_arch = "wasm32"))]
        std::thread::spawn(move || {
            // TODO: should probably use better executor
            pollster::block_on(Self::task_runner(task_receiver));
        });

        #[cfg(target_arch = "wasm32")]
        wasm_bindgen_futures::spawn_local(Self::task_runner(task_receiver));
    }

    /// Implementation of an asset loader that runs in the background
    ///
    /// Should be started using `start_background_loader`
    async fn task_runner(task_receiver: async_channel::Receiver<Task>) {
        let mut running_tasks = futures::stream::FuturesUnordered::new();

        loop {
            if running_tasks.is_empty() {
                // when no assets are loading, only await new requests
                let load_request = task_receiver.recv().await.expect("channel closed");
                running_tasks.push(load_request);
                continue;
            } else {
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
                }
            }
        }
    }
}
