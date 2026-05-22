use std::path::PathBuf;

use crate::filesystem::LoadFileError;

#[derive(Clone, Debug)]
pub struct FileSystemContext {
    config: std::sync::Arc<FileSystemConfig>,
}

#[derive(Debug)]
pub struct FileSystemConfig {
    base_folder_path: reqwest::Url,
}

impl FileSystemContext {
    pub(crate) fn new() -> Self {
        let asset_folder = PathBuf::from(".");

        let base_folder_path = {
            let window = web_sys::window().expect("could not get window");
            let location = window.location();
            let path = location.origin().expect("could not get origin");
            let url = reqwest::Url::parse(&path).expect("could not base path");
            url
        };

        Self {
            config: std::sync::Arc::new(FileSystemConfig { base_folder_path }),
        }
    }
}

impl FileSystemContext {
    pub async fn load_bytes(
        &self,
        path: impl AsRef<std::path::Path>,
    ) -> Result<Vec<u8>, LoadFileError> {
        let path = path.as_ref().to_str().ok_or(LoadFileError::InvalidPath)?;
        let path = self
            .config
            .base_folder_path
            .join(path)
            .map_err(|err| LoadFileError::InvalidPath)?;
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

    pub async fn load_string(
        &self,
        path: impl AsRef<std::path::Path>,
    ) -> Result<String, LoadFileError> {
        let path = path.as_ref().to_str().ok_or(LoadFileError::InvalidPath)?;
        let path = self
            .config
            .base_folder_path
            .join(path)
            .map_err(|err| LoadFileError::InvalidPath)?;
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
