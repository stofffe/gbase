mod platform;

pub use platform::*;

use crate::{task::TaskHandleState::Taken, ConditionalSend, Context};
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
// Task trait
//

pub trait TaskTrait: Future + ConditionalSend {}

impl<F> TaskTrait for F where F: Future + ConditionalSend + 'static {}

//
// Task
//

pub struct Task {
    task: Pin<Box<dyn TaskTrait<Output = ()> + 'static>>,
}

impl Future for Task {
    type Output = ();

    fn poll(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        self.get_mut().task.as_mut().poll(cx)
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

    pub fn spawn_task<
        T: ConditionalSend + 'static,
        F: TaskTrait<Output = T> + ConditionalSend + 'static,
    >(
        &self,
        task: F,
    ) -> TaskHandle<T> {
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

//
// Task runtime
//

#[derive(Clone)]
pub struct TaskExecutorRuntime {
    task_sender: async_channel::Sender<Task>,
}

impl TaskExecutorRuntime {
    pub fn new(task_sender: async_channel::Sender<Task>) -> Self {
        Self { task_sender }
    }

    pub fn spawn_task<
        T: ConditionalSend + 'static,
        F: TaskTrait<Output = T> + ConditionalSend + 'static,
    >(
        &self,
        task: F,
    ) -> TaskHandle<T> {
        let (task_response_sender, task_response_receiver) = async_channel::bounded(1);

        let task_with_response = async move {
            let result = task.await;
            // ignore result since droppoin received TaskHandle will result in a SendError
            let _ = task_response_sender.send(result).await;
        };

        self.task_sender
            .try_send(Task {
                task: Box::pin(task_with_response),
            })
            .expect("could not send spawn task request");

        TaskHandle::new(task_response_receiver)
    }
}

//
// Commands
//

pub fn spawn_task<
    T: ConditionalSend + 'static,
    F: TaskTrait<Output = T> + ConditionalSend + 'static,
>(
    ctx: &mut Context,
    task: F,
) -> TaskHandle<T> {
    ctx.task.spawn_task(task)
}
