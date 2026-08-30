#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(not(target_arch = "wasm32"))]
pub use native::*;

#[cfg(target_arch = "wasm32")]
mod wasm;
#[cfg(target_arch = "wasm32")]
pub use wasm::*;

use crate::task::Task;

/// Trait for keeping platform implementations in check
///
/// Not actually used in the Context
pub trait TaskExecutorPlatformTrait {
    fn start(task_receiver: async_channel::Receiver<Task>) -> Self;
    fn check(&self);
    fn shutdown(self);
}
