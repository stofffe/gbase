#![deny(elided_lifetimes_in_paths)]
#![allow(clippy::new_without_default)]

mod app;
pub mod audio;
pub mod collision;
pub mod filesystem;
pub mod input;
pub mod random;
pub mod render;
pub mod time;
pub mod window;

#[cfg(feature = "hot_reload")]
pub mod hot_reload;

// exports
pub use app::*;

// re-exports
// TODO bytemuck and encase macros not exported
pub use bytemuck;
pub use encase;

pub use glam;
pub use log;
pub use notify;
pub use wgpu;
pub use winit;

pub use tracing;

// #[cfg(not(target_arch = "wasm32"))]
// pub use env_logger;
#[cfg(not(target_arch = "wasm32"))]
pub use pollster;

/// Holds neccesary state for running the engine
///
/// Sent with each command
pub struct Context {
    pub(crate) input: input::InputContext,
    pub(crate) time: time::TimeContext,
    #[allow(dead_code)]
    pub(crate) filesystem: filesystem::FileSystemContext,
    pub(crate) audio: audio::AudioContext,
    pub(crate) render: render::RenderContext,
    pub(crate) random: random::RandomContext,

    #[cfg(feature = "hot_reload")]
    pub(crate) hot_reload: hot_reload::HotReloadContext,
}
