use crate::{
    filesystem::{platforms::LoadFileError, WriteFileError},
    ContextBuilder,
};
use base64::Engine;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct FileSystemContext {
    config: std::sync::Arc<FileSystemConfig>,
}

#[derive(Debug)]
pub struct FileSystemConfig {
    asset_folder_path: PathBuf,
    temporary_folder_path: PathBuf,

    base_url: reqwest::Url,
    local_storage: web_sys::Storage,
}

impl FileSystemContext {
    pub(crate) fn new(builder: &ContextBuilder) -> Self {
        let asset_folder_path = builder.assets_path.clone();
        let temporary_folder_path = builder.temporary_path.clone();

        let window = web_sys::window().expect("could not get window");
        let location = window.location();
        let origin = location.origin().expect("could not get origin");
        let base_url = reqwest::Url::parse(&origin).expect("could not base path");

        let local_storage = window
            .local_storage()
            .expect("could not get local storage")
            .expect("local storage is empty");

        Self {
            config: std::sync::Arc::new(FileSystemConfig {
                asset_folder_path,
                temporary_folder_path,
                base_url,
                local_storage,
            }),
        }
    }
}

impl FileSystemContext {
    pub fn format_asset_path(&self, path: impl AsRef<Path>) -> PathBuf {
        self.config.asset_folder_path.join(path)
    }

    pub async fn load_asset_bytes(&self, path: impl AsRef<Path>) -> Result<Vec<u8>, LoadFileError> {
        let path = self.format_asset_path(path);
        let path = path.to_str().ok_or(LoadFileError::InvalidPath)?;

        let path = self
            .config
            .base_url
            .join(path)
            .map_err(|_| LoadFileError::InvalidPath)?;
        let response = reqwest::Client::new()
            .get(path)
            .send()
            .await
            .map_err(|err| LoadFileError::Other(Box::new(err)))?;
        let bytes = response
            .bytes()
            .await
            .map_err(|err| LoadFileError::Other(Box::new(err)))?;

        Ok(bytes.to_vec())
    }

    pub async fn load_asset_string(&self, path: impl AsRef<Path>) -> Result<String, LoadFileError> {
        let path = self.format_asset_path(path);
        let path = path.to_str().ok_or(LoadFileError::InvalidPath)?;

        let path = self
            .config
            .base_url
            .join(path)
            .map_err(|_| LoadFileError::InvalidPath)?;
        let response = reqwest::Client::new()
            .get(path)
            .send()
            .await
            .map_err(|err| LoadFileError::Other(Box::new(err)))?;
        let str = response
            .text()
            .await
            .map_err(|err| LoadFileError::Other(Box::new(err)))?;

        Ok(str)
    }

    pub fn write_temporary_bytes(
        &self,
        path: impl AsRef<std::path::Path>,
        data: &[u8],
    ) -> Result<(), WriteFileError> {
        let temp_path = self.config.temporary_folder_path.join(path);
        let path = self.format_asset_path(&temp_path);
        let path = path.to_str().ok_or(WriteFileError::InvalidPath)?;

        let encoded = base64::engine::general_purpose::STANDARD.encode(data);
        self.config.local_storage.set_item(&path, &encoded);

        Ok(())
    }

    pub fn load_temporary_bytes(
        &self,
        path: impl AsRef<std::path::Path>,
    ) -> Result<Vec<u8>, LoadFileError> {
        let temp_path = self.config.temporary_folder_path.join(path);
        let path = self.format_asset_path(&temp_path);
        let path = path.to_str().ok_or(LoadFileError::InvalidPath)?;

        let data = self
            .config
            .local_storage
            .get_item(&path)
            .map_err(|_| LoadFileError::Placeholder)?
            .ok_or(LoadFileError::Placeholder)?;
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(data)
            .map_err(|err| LoadFileError::Other(Box::new(err)))?;

        Ok(decoded)
    }

    pub fn write_temporary_string(
        &self,
        path: impl AsRef<std::path::Path>,
        data: &str,
    ) -> Result<(), WriteFileError> {
        let temp_path = self.config.temporary_folder_path.join(path);
        let path = self.format_asset_path(&temp_path);
        let path = path.to_str().ok_or(WriteFileError::InvalidPath)?;

        self.config.local_storage.set_item(&path, &data);

        Ok(())
    }

    pub fn load_temporary_string(
        &self,
        path: impl AsRef<std::path::Path>,
    ) -> Result<String, LoadFileError> {
        let temp_path = self.config.temporary_folder_path.join(path);
        let path = self.format_asset_path(&temp_path);
        let path = path.to_str().ok_or(LoadFileError::InvalidPath)?;

        let data = self
            .config
            .local_storage
            .get_item(&path)
            .map_err(|_| LoadFileError::Placeholder)?
            .ok_or(LoadFileError::Placeholder)?;

        Ok(data)
    }
}
