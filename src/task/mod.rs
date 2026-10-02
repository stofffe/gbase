mod platform;

pub use platform::*;

use crate::{ConditionalSend, Context};
use std::{future::Future, pin::Pin};

//
// Task handle
//

pub enum TaskHandleState<T> {
    Loading,
    Ready(T),
    Taken,
}

pub struct TaskHandle<T> {
    receiver: async_channel::Receiver<T>,
    taken: bool,
}

impl<T> TaskHandle<T> {
    pub fn new(receiver: async_channel::Receiver<T>) -> Self {
        let taken = false;
        Self { receiver, taken }
    }

    pub fn try_take(&mut self) -> TaskHandleState<T> {
        if self.taken {
            return TaskHandleState::Taken;
        }

        match self.receiver.try_recv() {
            Ok(value) => {
                self.taken = true;
                TaskHandleState::Ready(value)
            }
            Err(async_channel::TryRecvError::Empty) => TaskHandleState::Loading,
            Err(async_channel::TryRecvError::Closed) => {
                panic!("trying to receive task response on a closed channel")
            }
        }
    }

    pub async fn take(&self) -> T {
        self.receiver
            .recv()
            .await
            .expect("could not receive from channel")
    }
}

//
// Task
//

// Factory for pinned future returning T which is conditionally sendable
pub trait TaskTrait: FnOnce() -> Pin<Box<dyn Future<Output = ()>>> + ConditionalSend {}
impl<F: FnOnce() -> Pin<Box<dyn Future<Output = ()>>> + ConditionalSend> TaskTrait for F {}

// TODO: do you even need this or should logic in new into spawn function
pub struct Task<T: ConditionalSend> {
    task: Box<dyn TaskTrait>,
    task_response_receiver: async_channel::Receiver<T>,
}

impl<T: ConditionalSend + 'static> Task<T> {
    /// Create a task from a closure that returns future
    ///
    /// The Task will be sent across threads so any captured variables must be Send
    ///
    /// Example usage:
    /// ```rust
    /// Task::new(move || async move {
    ///     some_func().await;
    /// })
    /// ```
    pub fn new<F, Fut>(task: F) -> Self
    where
        F: FnOnce() -> Fut + ConditionalSend + 'static,
        Fut: Future<Output = T> + 'static,
    {
        let (task_response_sender, task_response_receiver) = async_channel::bounded(1);
        Self {
            task: Box::new(move || {
                Box::pin(async move {
                    let result = task().await;
                    task_response_sender
                        .send(result)
                        .await
                        .expect("could not send the task result");
                })
            }),
            task_response_receiver,
        }
    }
}

pub trait DynTask: ConditionalSend {
    fn into_future(self: Box<Self>) -> Pin<Box<dyn Future<Output = ()> + 'static>>;
}

impl<T: ConditionalSend + 'static> DynTask for Task<T> {
    fn into_future(self: Box<Self>) -> Pin<Box<dyn Future<Output = ()> + 'static>> {
        (self.task)()
    }
}

//
// Task context
//

pub struct TaskContext {
    task_executor: TaskExecutor,
}

impl TaskContext {
    pub fn new() -> Self {
        let task_executor = TaskExecutor::new();

        Self { task_executor }
    }

    pub fn spawn_task<T: ConditionalSend + 'static>(&self, task: Task<T>) -> TaskHandle<T> {
        self.runtime().spawn_task(task)
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

//
// Task executor
//

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

    pub fn spawn_task<T: ConditionalSend + 'static>(&self, task: Task<T>) -> TaskHandle<T> {
        self.runtime.spawn_task(task)
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

//
// Task runtime
//

#[derive(Clone)]
pub struct TaskExecutorRuntime {
    task_sender: async_channel::Sender<Box<dyn DynTask>>,
}

impl TaskExecutorRuntime {
    pub fn new(task_sender: async_channel::Sender<Box<dyn DynTask>>) -> Self {
        Self { task_sender }
    }

    pub fn spawn_task<T: ConditionalSend + 'static>(&self, task: Task<T>) -> TaskHandle<T> {
        let receiver = task.task_response_receiver.clone();

        self.task_sender
            .try_send(Box::new(task))
            .expect("could not send spawn task request");

        TaskHandle::new(receiver)
    }
}

//
// Commands
//

pub fn spawn_task<T: ConditionalSend + 'static>(ctx: &mut Context, task: Task<T>) -> TaskHandle<T> {
    ctx.task.spawn_task(task)
}
