#![deny(elided_lifetimes_in_paths)]
#![allow(clippy::new_without_default)]

mod app;
pub mod asset;
pub mod audio;
pub mod collision; // TODO: move to utils?
pub mod egui_ui;
pub mod filesystem;
pub mod input;
pub mod random;
pub mod render;
pub mod time;

#[cfg(all(feature = "hot_reload", target_arch = "wasm32"))]
compile_error!("The 'hot_reload' feature is not supported on wasm32");
#[cfg(all(feature = "trace_tracy", target_arch = "wasm32"))]
compile_error!("The 'trace_tracy' feature is not supported on wasm32");

#[cfg(all(feature = "hot_reload", not(target_arch = "wasm32")))]
pub mod hot_reload;

// exports
pub use app::*;

// re-exports
// TODO bytemuck and encase macros not exported
pub use bytemuck;
pub use encase;

pub use egui;
pub use futures_channel;
pub use glam;
pub use rustc_hash;
pub use tracing;
pub use wgpu;
pub use winit;

#[cfg(not(target_arch = "wasm32"))]
pub use pollster;

/// Holds neccesary state for running the engine
///
/// Sent with each command
pub struct Context {
    pub(crate) input: input::InputContext,
    pub(crate) time: time::TimeContext,
    pub filesystem: filesystem::FileSystemContext,
    pub(crate) audio: audio::AudioContext,
    pub render: render::RenderContext,
    pub(crate) random: random::RandomContext,

    pub egui: egui_ui::EguiContext,

    #[cfg(feature = "hot_reload")]
    pub(crate) hot_reload: hot_reload::HotReloadContext,
}
