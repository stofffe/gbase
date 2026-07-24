mod executor;

pub use executor::*;

use std::{future::Future, pin::Pin};

#[cfg(target_arch = "wasm32")]
pub type Task = Pin<Box<dyn Future<Output = ()> + 'static>>;

#[cfg(not(target_arch = "wasm32"))]
pub type Task = Pin<Box<dyn Future<Output = ()> + Send + 'static>>;

#[derive(Clone)]
pub struct TaskContext {
    general_executor: TaskExecutor,
}

impl TaskContext {
    pub fn new() -> Self {
        let general_executor = TaskExecutor::new();
        general_executor.start();

        Self { general_executor }
    }

    pub fn spawn_task(&self, task: Task) {
        self.general_executor.spawn(task);
    }
}
