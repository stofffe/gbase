use std::path::PathBuf;

use crate::filesystem::platforms::LoadFileError;

#[derive(Clone, Debug)]
pub struct FileSystemContext {
    config: std::sync::Arc<FileSystemConfig>,
}

#[derive(Debug)]
pub struct FileSystemConfig {
    base_folder_path: PathBuf,
}

impl FileSystemContext {
    pub(crate) fn new() -> Self {
        let asset_folder = PathBuf::from(".");

        let base_folder_path = if asset_folder.is_absolute() {
            asset_folder
        } else {
            std::env::current_dir()
                .expect("could not get current working dir")
                .join(&asset_folder)
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
        let bytes = {
            let path = self.config.base_folder_path.join(path);
            std::fs::read(&path).map_err(|err| match err.kind() {
                std::io::ErrorKind::NotFound => LoadFileError::FileNotFound,
                _ => LoadFileError::Other(Box::new(err)),
            })?
        };

        Ok(bytes)
    }

    pub async fn load_string(
        &self,
        path: impl AsRef<std::path::Path>,
    ) -> Result<String, LoadFileError> {
        let str = {
            let path = self.config.base_folder_path.join(path);
            std::fs::read_to_string(&path).map_err(|err| match err.kind() {
                std::io::ErrorKind::NotFound => LoadFileError::FileNotFound,
                _ => LoadFileError::Other(Box::new(err)),
            })?
        };

        Ok(str)
    }
}
