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
    data_folder_path: PathBuf,
}

impl FileSystemPlatformTrait for NativeFileSystem {
    fn new(asset_path: PathBuf, data_path: PathBuf) -> Self {
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

        let data_folder_path = if data_path.is_absolute() {
            data_path
        } else {
            std::env::current_dir()
                .expect("could not get current working dir")
                .join(&data_path)
        };
        if !data_folder_path.is_dir() {
            std::fs::create_dir_all(&data_folder_path).expect("could not create data folder");
        }

        let config = NativeFileSystemConfig {
            asset_folder_path,
            data_folder_path,
        };

        Self {
            config: Arc::new(config),
        }
    }

    fn format_asset_path(&self, path: impl AsRef<Path>) -> PathBuf {
        self.config.asset_folder_path.join(path)
    }

    fn format_data_path(&self, path: impl AsRef<Path>) -> PathBuf {
        self.config.data_folder_path.join(path)
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
        self.create_dir_if_needed(&path);

        std::fs::write(path, bytes).map_err(|err| WriteFileError::Other(Box::new(err)))
    }
    async fn write_string(
        &self,
        path: impl AsRef<Path>,
        string: impl AsRef<str>,
    ) -> Result<(), WriteFileError> {
        self.create_dir_if_needed(&path);

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
    // Data
    //

    async fn load_data_bytes(&self, path: impl AsRef<Path>) -> Result<Vec<u8>, LoadFileError> {
        let data_path = self.format_data_path(path);
        self.load_bytes(data_path).await
    }

    async fn load_data_string(&self, path: impl AsRef<Path>) -> Result<String, LoadFileError> {
        let data_path = self.format_data_path(path);
        self.load_string(data_path).await
    }
    async fn write_data_bytes(
        &self,
        path: impl AsRef<Path>,
        bytes: impl AsRef<[u8]>,
    ) -> Result<(), WriteFileError> {
        let data_path = self.format_data_path(path);
        self.write_bytes(data_path, bytes).await
    }

    async fn write_data_string(
        &self,
        path: impl AsRef<Path>,
        string: impl AsRef<str>,
    ) -> Result<(), WriteFileError> {
        let data_path = self.format_data_path(path);
        self.write_string(data_path, string).await
    }
}

impl NativeFileSystem {
    fn create_dir_if_needed(&self, path: impl AsRef<Path>) {
        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent).expect("could not create dir");
        }
    }
}
