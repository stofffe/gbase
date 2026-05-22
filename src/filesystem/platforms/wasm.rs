use crate::{filesystem::platforms::LoadFileError, ContextBuilder};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct FileSystemContext {
    config: std::sync::Arc<FileSystemConfig>,
}

#[derive(Debug)]
pub struct FileSystemConfig {
    asset_folder_path: PathBuf,
    base_url: reqwest::Url,
}

impl FileSystemContext {
    pub(crate) fn new(builder: &ContextBuilder) -> Self {
        let asset_folder_path = builder.assets_path.clone();

        let window = web_sys::window().expect("could not get window");
        let location = window.location();
        let origin = location.origin().expect("could not get origin");
        let base_url = reqwest::Url::parse(&origin).expect("could not base path");

        Self {
            config: std::sync::Arc::new(FileSystemConfig {
                asset_folder_path,
                base_url,
            }),
        }
    }
}

impl FileSystemContext {
    pub fn format_asset_path(&self, path: impl AsRef<Path>) -> PathBuf {
        self.config.asset_folder_path.join(path)
    }

    pub async fn load_bytes(&self, path: impl AsRef<Path>) -> Result<Vec<u8>, LoadFileError> {
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

    pub async fn load_string(&self, path: impl AsRef<Path>) -> Result<String, LoadFileError> {
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
}
