use base64::Engine;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::filesystem::{FileSystemPlatformTrait, LoadFileError, WriteFileError};

pub type FileSystemPlatform = WasmFileSystem;

#[derive(Clone)]
pub struct WasmFileSystem {
    config: Arc<WasmFileSystemConfig>,
}

pub struct WasmFileSystemConfig {
    asset_folder_path: PathBuf,
    data_folder_path: PathBuf,

    base_url: reqwest::Url,
    client: reqwest::Client,

    local_storage: web_sys::Storage,
}

impl FileSystemPlatformTrait for WasmFileSystem {
    fn new(asset_path: std::path::PathBuf, data_path: std::path::PathBuf) -> Self {
        let asset_folder_path = asset_path.to_path_buf();
        let data_folder_path = data_path.to_path_buf();

        let window = web_sys::window().expect("could not get window");
        let location = window.location();
        let origin = location.origin().expect("could not get origin");
        let base_url = reqwest::Url::parse(&origin).expect("could not base path");
        let client = reqwest::Client::new();

        let local_storage = window
            .local_storage()
            .expect("could not get local storage")
            .expect("local storage is empty");

        Self {
            config: std::sync::Arc::new(WasmFileSystemConfig {
                asset_folder_path,
                data_folder_path,
                base_url,
                client,
                local_storage,
            }),
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
        let url = self.resolve_url(path)?;
        let response = self
            .config
            .client
            .get(url)
            .send()
            .await
            .map_err(|err| LoadFileError::Other(Box::new(err)))?;
        let bytes = response
            .bytes()
            .await
            .map_err(|err| LoadFileError::Other(Box::new(err)))?;

        Ok(bytes.to_vec())
    }

    async fn load_string(&self, path: impl AsRef<Path>) -> Result<String, LoadFileError> {
        let url = self.resolve_url(path)?;
        let response = self
            .config
            .client
            .get(url)
            .send()
            .await
            .map_err(|err| LoadFileError::Other(Box::new(err)))?;
        let str = response
            .text()
            .await
            .map_err(|err| LoadFileError::Other(Box::new(err)))?;

        Ok(str)
    }

    async fn write_bytes(
        &self,
        _path: impl AsRef<Path>,
        _bytes: impl AsRef<[u8]>,
    ) -> Result<(), WriteFileError> {
        panic!("writing bytes currently unsupported in wasm");
    }

    async fn write_string(
        &self,
        _path: impl AsRef<Path>,
        _string: impl AsRef<str>,
    ) -> Result<(), WriteFileError> {
        panic!("writing string currently unsupported in wasm");
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
        let temp_path = self.config.data_folder_path.join(path);
        let path = self.format_asset_path(&temp_path);
        let path = path.to_str().ok_or(LoadFileError::InvalidPath)?;

        let data = self
            .config
            .local_storage
            .get_item(path)
            .map_err(|_| LoadFileError::Placeholder)?
            .ok_or(LoadFileError::Placeholder)?;
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(data)
            .map_err(|err| LoadFileError::Other(Box::new(err)))?;

        Ok(decoded)
    }

    async fn load_data_string(&self, path: impl AsRef<Path>) -> Result<String, LoadFileError> {
        let temp_path = self.config.data_folder_path.join(path);
        let path = self.format_asset_path(&temp_path);
        let path = path.to_str().ok_or(LoadFileError::InvalidPath)?;

        let data = self
            .config
            .local_storage
            .get_item(path)
            .map_err(|_| LoadFileError::Placeholder)?
            .ok_or(LoadFileError::Placeholder)?;

        Ok(data)
    }

    async fn write_data_bytes(
        &self,
        path: impl AsRef<Path>,
        bytes: impl AsRef<[u8]>,
    ) -> Result<(), WriteFileError> {
        let temp_path = self.config.data_folder_path.join(path);
        let path = self.format_asset_path(&temp_path);
        let path = path.to_str().ok_or(WriteFileError::InvalidPath)?;

        let encoded = base64::engine::general_purpose::STANDARD.encode(bytes);
        self.config
            .local_storage
            .set_item(path, &encoded)
            .map_err(|err| {
                WriteFileError::Other(
                    format!("could not set string in local storage: {:?}", err).into(),
                )
            })?;

        Ok(())
    }

    async fn write_data_string(
        &self,
        path: impl AsRef<Path>,
        string: impl AsRef<str>,
    ) -> Result<(), WriteFileError> {
        let temp_path = self.config.data_folder_path.join(path);
        let path = self.format_asset_path(&temp_path);
        let path = path.to_str().ok_or(WriteFileError::InvalidPath)?;

        self.config
            .local_storage
            .set_item(path, string.as_ref())
            .map_err(|err| {
                WriteFileError::Other(
                    format!("could not set string in local storage: {:?}", err).into(),
                )
            })?;

        Ok(())
    }
}

impl WasmFileSystem {
    fn resolve_url(&self, path: impl AsRef<Path>) -> Result<reqwest::Url, LoadFileError> {
        let path_str = path.as_ref().to_str().ok_or(LoadFileError::InvalidPath)?;

        // TODO: this might be needed
        // let path_str = path_str.replace('\\', "/");

        self.config
            .base_url
            .join(&path_str)
            .map_err(|_| LoadFileError::InvalidPath)
    }
}
