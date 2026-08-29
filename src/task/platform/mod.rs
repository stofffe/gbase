#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(not(target_arch = "wasm32"))]
pub use native::*;

#[cfg(target_arch = "wasm32")]
mod wasm;
#[cfg(target_arch = "wasm32")]
pub use wasm::*;

/// Trait for keeping platform implementations in check
///
/// Not actually used in the Context
pub trait TaskExecutorPlatformTrait {
    fn start(task_receiver: async_channel::Receiver<Task>) -> Self;
    fn shutdown();
}
