mod app;
pub mod audio;
pub mod filesystem;
pub mod input;
pub mod render;
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
    input: input::InputContext,
    time: time::TimeContext,
    filesystem: filesystem::FileSystemContext,
    audio: audio::AudioContext,
    render: render::RenderContext,
}
