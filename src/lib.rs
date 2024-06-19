#![deny(elided_lifetimes_in_paths)]
#![allow(clippy::new_without_default)]

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
// TODO bytemuck and encase macros not exported
pub use bytemuck;
pub use encase;
pub use glam;
pub use log;
pub use wgpu;
pub use winit;

/// Holds neccesary state for running the engine
///
/// Sent with each command
pub struct Context {
    pub(crate) input: input::InputContext,
    pub(crate) time: time::TimeContext,
    pub(crate) filesystem: filesystem::FileSystemContext,
    pub(crate) audio: audio::AudioContext,
    pub(crate) render: render::RenderContext,
}
