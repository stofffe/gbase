#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(not(target_arch = "wasm32"))]
pub use native::*;

#[cfg(target_arch = "wasm32")]
mod wasm;
#[cfg(target_arch = "wasm32")]
pub use wasm::*;

use crate::task::DynTask;
use futures_concurrency::future::Race;

/// Trait for keeping platform implementations in check
///
/// Not actually used in the Context
pub trait TaskExecutorPlatformTrait {
    fn start(task_receiver: async_channel::Receiver<Box<dyn DynTask>>) -> Self;
    fn check(&self);
    fn shutdown(self);
}

/// Implementation of an asset loader that runs in the background
///
/// Should be started using `start_background_loader`
pub(crate) async fn task_runner(
    task_receiver: async_channel::Receiver<Box<dyn DynTask>>,
    shutdown_receiver: async_channel::Receiver<()>,
) {
    let task_executor = async_executor::LocalExecutor::new();

    loop {
        let selected_futures = (
            async {
                task_executor.tick().await;
                false
            },
            async {
                let task = task_receiver
                    .recv()
                    .await
                    .expect("could not receive from task channel");
                task_executor.spawn(task.into_future()).detach();
                false
            },
            async {
                shutdown_receiver
                    .recv()
                    .await
                    .expect("could not receive from shutdown channel");
                true
            },
        );

        let should_shutdown = selected_futures.race().await;

        if should_shutdown {
            break;
        }
    }
}
