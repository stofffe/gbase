#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(not(target_arch = "wasm32"))]
pub use native::*;

#[cfg(target_arch = "wasm32")]
mod wasm;
#[cfg(target_arch = "wasm32")]
pub use wasm::*;

#[derive(thiserror::Error, Debug)]
pub enum LoadFileError {
    #[error("file not found")]
    FileNotFound,
    #[error("invalid path")]
    InvalidPath,
    #[error("other error: {0}")]
    Other(Box<dyn std::error::Error + Send + Sync>),

    #[error("internal")]
    Placeholder,
}

#[derive(thiserror::Error, Debug)]
pub enum WriteFileError {
    #[error("invalid path")]
    InvalidPath,
    #[error("other error: {0}")]
    Other(Box<dyn std::error::Error + Send + Sync>),
}
