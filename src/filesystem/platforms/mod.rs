#[cfg(not(target_arch = "wasm32"))]
mod native;
#[cfg(not(target_arch = "wasm32"))]
pub use native::*;

#[cfg(target_arch = "wasm32")]
mod wasm;
#[cfg(target_arch = "wasm32")]
pub use wasm::*;

use std::path::{Path, PathBuf};

use crate::filesystem::{LoadFileError, WriteFileError};

pub trait FileSystemPlatformTrait {
    fn new(asset_path: PathBuf, data_path: PathBuf) -> Self;

    fn format_asset_path(&self, path: impl AsRef<Path>) -> PathBuf;
    fn format_data_path(&self, path: impl AsRef<Path>) -> PathBuf;

    //
    // Async
    //

    fn load_bytes(
        &self,
        path: impl AsRef<Path>,
    ) -> impl std::future::Future<Output = Result<Vec<u8>, LoadFileError>>;
    fn load_string(
        &self,
        path: impl AsRef<Path>,
    ) -> impl std::future::Future<Output = Result<String, LoadFileError>>;
    fn write_bytes(
        &self,
        path: impl AsRef<Path>,
        bytes: impl AsRef<[u8]>,
    ) -> impl std::future::Future<Output = Result<(), WriteFileError>>;
    fn write_string(
        &self,
        path: impl AsRef<Path>,
        string: impl AsRef<str>,
    ) -> impl std::future::Future<Output = Result<(), WriteFileError>>;

    //
    // Asset
    //

    fn load_asset_bytes(
        &self,
        path: impl AsRef<Path>,
    ) -> impl std::future::Future<Output = Result<Vec<u8>, LoadFileError>>;
    fn load_asset_string(
        &self,
        path: impl AsRef<Path>,
    ) -> impl std::future::Future<Output = Result<String, LoadFileError>>;

    //
    // Data
    //

    fn load_data_bytes(
        &self,
        path: impl AsRef<Path>,
    ) -> impl std::future::Future<Output = Result<Vec<u8>, LoadFileError>>;
    fn load_data_string(
        &self,
        path: impl AsRef<Path>,
    ) -> impl std::future::Future<Output = Result<String, LoadFileError>>;
    fn write_data_bytes(
        &self,
        path: impl AsRef<Path>,
        bytes: impl AsRef<[u8]>,
    ) -> impl std::future::Future<Output = Result<(), WriteFileError>>;
    fn write_data_string(
        &self,
        path: impl AsRef<Path>,
        string: impl AsRef<str>,
    ) -> impl std::future::Future<Output = Result<(), WriteFileError>>;
}
