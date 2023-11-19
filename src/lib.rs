mod app;
mod window;

pub use app::*;

/// Holds neccesary state for running the engine
///
/// Sent with each command
pub struct Context {
    window: winit::window::Window,
}

