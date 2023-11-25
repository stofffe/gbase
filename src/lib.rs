mod app;
pub mod input;
pub mod time;
pub mod window;

// exports
pub use app::*;

// re-exports
pub use glam;
pub use log;
pub use wgpu;
pub use winit;

/// Holds neccesary state for running the engine
///
/// Sent with each command
pub struct Context {
    window: winit::window::Window,
    input: input::InputContext,
    time: time::TimeContext,
}
