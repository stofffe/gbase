use pollster::FutureExt;

use crate::filesystem::{FileSystemPlatformTrait, LoadFileError, WriteFileError};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

pub type FileSystemPlatform = NativeFileSystem;

#[derive(Clone)]
pub struct NativeFileSystem {
    config: Arc<NativeFileSystemConfig>,
}

#[derive(Clone)]
pub struct NativeFileSystemConfig {
    asset_folder_path: PathBuf,
    temporary_folder_path: PathBuf,
}

impl FileSystemPlatformTrait for NativeFileSystem {
    fn new(asset_path: PathBuf, temporary_path: PathBuf) -> Self {
        let asset_folder_path = if asset_path.is_absolute() {
            asset_path
        } else {
            std::env::current_dir()
                .expect("could not get current working dir")
                .join(asset_path)
        };
        if !asset_folder_path.is_dir() {
            std::fs::create_dir_all(&asset_folder_path).expect("could not create asset folder");
        }

        let temporary_folder_path = if temporary_path.is_absolute() {
            temporary_path
        } else {
            std::env::current_dir()
                .expect("could not get current working dir")
                .join(&temporary_path)
        };
        if !temporary_folder_path.is_dir() {
            std::fs::create_dir_all(&temporary_folder_path)
                .expect("could not create temporary folder");
        }

        let config = NativeFileSystemConfig {
            asset_folder_path,
            temporary_folder_path,
        };

        Self {
            config: Arc::new(config),
        }
    }

    fn format_asset_path(&self, path: impl AsRef<Path>) -> PathBuf {
        self.config.asset_folder_path.join(path)
    }

    fn format_temporary_path(&self, path: impl AsRef<Path>) -> PathBuf {
        self.config.temporary_folder_path.join(path)
    }

    //
    // Normal
    //

    async fn load_bytes(&self, path: impl AsRef<Path>) -> Result<Vec<u8>, LoadFileError> {
        std::fs::read(&path).map_err(|err| match err.kind() {
            std::io::ErrorKind::NotFound => LoadFileError::FileNotFound,
            _ => LoadFileError::Other(Box::new(err)),
        })
    }
    async fn load_string(&self, path: impl AsRef<Path>) -> Result<String, LoadFileError> {
        std::fs::read_to_string(&path).map_err(|err| match err.kind() {
            std::io::ErrorKind::NotFound => LoadFileError::FileNotFound,
            _ => LoadFileError::Other(Box::new(err)),
        })
    }
    async fn write_bytes(
        &self,
        path: impl AsRef<Path>,
        bytes: impl AsRef<[u8]>,
    ) -> Result<(), WriteFileError> {
        std::fs::write(path, bytes).map_err(|err| WriteFileError::Other(Box::new(err)))
    }
    async fn write_string(
        &self,
        path: impl AsRef<Path>,
        string: impl AsRef<str>,
    ) -> Result<(), WriteFileError> {
        std::fs::write(path, string.as_ref()).map_err(|err| WriteFileError::Other(Box::new(err)))
    }

    //
    // Asset
    //

    async fn load_asset_bytes(&self, path: impl AsRef<Path>) -> Result<Vec<u8>, LoadFileError> {
        let asset_path = self.format_asset_path(path);
        self.load_bytes(asset_path).await
    }
    async fn load_asset_string(&self, path: impl AsRef<Path>) -> Result<String, LoadFileError> {
        let asset_path = self.format_asset_path(path);
        self.load_string(asset_path).await
    }

    //
    // Temporary
    //

    async fn load_temporary_bytes(&self, path: impl AsRef<Path>) -> Result<Vec<u8>, LoadFileError> {
        let temporary_path = self.format_temporary_path(path);
        self.load_bytes(temporary_path).await
    }

    async fn load_temporary_string(&self, path: impl AsRef<Path>) -> Result<String, LoadFileError> {
        let temporary_path = self.format_temporary_path(path);
        self.load_string(temporary_path).await
    }
    async fn write_temporary_bytes(
        &self,
        path: impl AsRef<Path>,
        bytes: impl AsRef<[u8]>,
    ) -> Result<(), WriteFileError> {
        let temporary_path = self.format_temporary_path(path);
        self.write_bytes(temporary_path, bytes).await
    }

    async fn write_temporary_string(
        &self,
        path: impl AsRef<Path>,
        string: impl AsRef<str>,
    ) -> Result<(), WriteFileError> {
        let temporary_path = self.format_temporary_path(path);
        self.write_string(temporary_path, string).await
    }
}
