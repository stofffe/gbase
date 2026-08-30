mod platform;

pub use platform::*;

use futures::{FutureExt, StreamExt};

pub struct TaskContext {
    task_executor: TaskExecutor,
}

impl TaskContext {
    pub fn new() -> Self {
        let task_executor = TaskExecutor::new();

        Self { task_executor }
    }

    pub fn check(&self) {
        self.task_executor.check();
    }

    pub fn runtime(&self) -> TaskExecutorRuntime {
        self.task_executor.runtime()
    }

    pub fn spawn_task(&self, task: Task) {
        self.runtime().spawn_task(task);
    }
}

pub struct TaskExecutor {
    executor: TaskExecutorPlatform,
    runtime: TaskExecutorRuntime,
}

impl TaskExecutor {
    pub fn new() -> Self {
        let (task_sender, task_receiver) = async_channel::unbounded();
        let runtime = TaskExecutorRuntime::new(task_sender);
        let executor = TaskExecutorPlatform::start(task_receiver);
        Self { executor, runtime }
    }

    pub fn spawn_task(&self, task: Task) {
        self.runtime.spawn_task(task);
    }

    pub fn runtime(&self) -> TaskExecutorRuntime {
        self.runtime.clone()
    }

    pub fn check(&self) {
        self.executor.check();
    }
}

#[derive(Clone)]
pub struct TaskExecutorRuntime {
    task_sender: async_channel::Sender<Task>,
}

impl TaskExecutorRuntime {
    pub fn new(task_sender: async_channel::Sender<Task>) -> Self {
        Self { task_sender }
    }

    pub fn spawn_task(&self, task: Task) {
        self.task_sender
            .try_send(task)
            .expect("could not send spawn task request");
    }
}

/// Implementation of an asset loader that runs in the background
///
/// Should be started using `start_background_loader`
pub(super) async fn task_runner(task_receiver: async_channel::Receiver<Task>) {
    let mut running_tasks = futures::stream::FuturesUnordered::new();

    loop {
        if running_tasks.is_empty() {
            // when no assets are loading, only await new requests
            let load_request = task_receiver.recv().await.expect("channel closed");
            running_tasks.push(load_request);
            continue;
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
            }
        }
    }
}
