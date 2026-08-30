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

    pub fn spawn_task(&self, task: Task) {
        self.runtime().spawn_task(task);
    }

    pub fn check(&self) {
        self.task_executor.check();
    }

    pub fn shutdown(self) {
        self.task_executor.shutdown();
    }

    pub fn runtime(&self) -> TaskExecutorRuntime {
        self.task_executor.runtime()
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

    pub fn shutdown(self) {
        self.executor.shutdown();
    }

    pub fn check(&self) {
        self.executor.check();
    }

    pub fn runtime(&self) -> TaskExecutorRuntime {
        self.runtime.clone()
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
