use crate::{filesystem::platforms::LoadFileError, ContextBuilder};
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct FileSystemContext {
    config: std::sync::Arc<FileSystemConfig>,
}

#[derive(Debug)]
pub struct FileSystemConfig {
    asset_folder_path: PathBuf,
}

impl FileSystemContext {
    pub(crate) fn new(builder: &ContextBuilder) -> Self {
        let asset_folder = builder.assets_path.clone();

        let asset_folder_path = if asset_folder.is_absolute() {
            asset_folder
        } else {
            std::env::current_dir()
                .expect("could not get current working dir")
                .join(&asset_folder)
        };

        Self {
            config: std::sync::Arc::new(FileSystemConfig { asset_folder_path }),
        }
    }
}

impl FileSystemContext {
    pub fn format_asset_path(&self, path: impl AsRef<std::path::Path>) -> PathBuf {
        self.config.asset_folder_path.join(path)
    }

    pub async fn load_bytes(
        &self,
        path: impl AsRef<std::path::Path>,
    ) -> Result<Vec<u8>, LoadFileError> {
        let path = self.format_asset_path(path);
        let bytes = std::fs::read(&path).map_err(|err| match err.kind() {
            std::io::ErrorKind::NotFound => LoadFileError::FileNotFound,
            _ => LoadFileError::Other(Box::new(err)),
        })?;

        Ok(bytes)
    }

    pub async fn load_string(
        &self,
        path: impl AsRef<std::path::Path>,
    ) -> Result<String, LoadFileError> {
        let path = self.format_asset_path(path);
        let str = std::fs::read_to_string(&path).map_err(|err| match err.kind() {
            std::io::ErrorKind::NotFound => LoadFileError::FileNotFound,
            _ => LoadFileError::Other(Box::new(err)),
        })?;

        Ok(str)
    }
}
