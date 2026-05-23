use crate::{
    filesystem::{platforms::LoadFileError, WriteFileError},
    ContextBuilder,
};
use std::path::PathBuf;

#[derive(Clone, Debug)]
pub struct FileSystemContext {
    config: std::sync::Arc<FileSystemConfig>,
}

#[derive(Debug)]
pub struct FileSystemConfig {
    asset_folder_path: PathBuf,
    temporary_folder_path: PathBuf,
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
        if !asset_folder_path.is_dir() {
            std::fs::create_dir_all(&asset_folder_path).expect("could not create asset folder");
        }

        let temporary_folder = builder.temporary_path.clone();
        let temporary_folder_path = if temporary_folder.is_absolute() {
            temporary_folder
        } else {
            std::env::current_dir()
                .expect("could not get current working dir")
                .join(&temporary_folder)
        };
        if !temporary_folder_path.is_dir() {
            std::fs::create_dir_all(&temporary_folder_path)
                .expect("could not create temporary folder");
        }

        Self {
            config: std::sync::Arc::new(FileSystemConfig {
                asset_folder_path,
                temporary_folder_path,
            }),
        }
    }
}

impl FileSystemContext {
    pub fn format_asset_path(&self, path: impl AsRef<std::path::Path>) -> PathBuf {
        self.config.asset_folder_path.join(path)
    }

    pub async fn load_asset_bytes(
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

    pub async fn load_asset_string(
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

    pub fn write_temporary_bytes(
        &self,
        path: impl AsRef<std::path::Path>,
        data: &[u8],
    ) -> Result<(), WriteFileError> {
        let temp_path = self.config.temporary_folder_path.join(path);
        let path = self.format_asset_path(&temp_path);

        std::fs::write(path, data).map_err(|err| WriteFileError::Other(Box::new(err)))?;

        Ok(())
    }

    pub fn load_temporary_bytes(
        &self,
        path: impl AsRef<std::path::Path>,
    ) -> Result<Vec<u8>, LoadFileError> {
        let temp_path = self.config.temporary_folder_path.join(path);
        let path = self.format_asset_path(&temp_path);

        let bytes = std::fs::read(path).map_err(|err| LoadFileError::Other(Box::new(err)))?;

        Ok(bytes)
    }

    pub fn write_temporary_string(
        &self,
        path: impl AsRef<std::path::Path>,
        data: &str,
    ) -> Result<(), WriteFileError> {
        let temp_path = self.config.temporary_folder_path.join(path);
        let path = self.format_asset_path(&temp_path);

        std::fs::write(path, data).map_err(|err| WriteFileError::Other(Box::new(err)))?;

        Ok(())
    }

    pub fn load_temporary_string(
        &self,
        path: impl AsRef<std::path::Path>,
    ) -> Result<String, LoadFileError> {
        let temp_path = self.config.temporary_folder_path.join(path);
        let path = self.format_asset_path(&temp_path);

        let str =
            std::fs::read_to_string(path).map_err(|err| LoadFileError::Other(Box::new(err)))?;

        Ok(str)
    }
}
