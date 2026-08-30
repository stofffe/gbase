use crate::{
    filesystem::{FileSystemPlatform, FileSystemPlatformTrait, LoadFileError, WriteFileError},
    ContextBuilder,
};
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct FileSystemRuntime {
    platform: FileSystemPlatform,
}

pub struct FileSystemRuntimeConfig {}

impl FileSystemRuntime {
    pub fn new(builder: &ContextBuilder) -> Self {
        let platform = <FileSystemPlatform as FileSystemPlatformTrait>::new(
            builder.assets_path.clone(),
            builder.data_path.clone(),
        );
        Self { platform }
    }

    //
    // Format
    //

    pub fn format_asset_path(&self, path: impl AsRef<Path>) -> PathBuf {
        <FileSystemPlatform as FileSystemPlatformTrait>::format_asset_path(&self.platform, path)
    }

    pub fn format_data_path(&self, path: impl AsRef<Path>) -> PathBuf {
        <FileSystemPlatform as FileSystemPlatformTrait>::format_data_path(&self.platform, path)
    }

    //
    // Normal
    //

    pub async fn load_bytes(&self, path: impl AsRef<Path>) -> Result<Vec<u8>, LoadFileError> {
        <FileSystemPlatform as FileSystemPlatformTrait>::load_bytes(&self.platform, path).await
    }

    pub async fn load_string(&self, path: impl AsRef<Path>) -> Result<String, LoadFileError> {
        <FileSystemPlatform as FileSystemPlatformTrait>::load_string(&self.platform, path).await
    }

    pub async fn write_bytes(
        &self,
        path: impl AsRef<Path>,
        bytes: impl AsRef<[u8]>,
    ) -> Result<(), WriteFileError> {
        <FileSystemPlatform as FileSystemPlatformTrait>::write_bytes(&self.platform, path, bytes)
            .await
    }

    pub async fn write_string(
        &self,
        path: impl AsRef<Path>,
        string: impl AsRef<str>,
    ) -> Result<(), WriteFileError> {
        <FileSystemPlatform as FileSystemPlatformTrait>::write_string(&self.platform, path, string)
            .await
    }

    //
    // Asset
    //

    pub async fn load_asset_bytes(&self, path: impl AsRef<Path>) -> Result<Vec<u8>, LoadFileError> {
        <FileSystemPlatform as FileSystemPlatformTrait>::load_asset_bytes(&self.platform, path)
            .await
    }

    pub async fn load_asset_string(&self, path: impl AsRef<Path>) -> Result<String, LoadFileError> {
        <FileSystemPlatform as FileSystemPlatformTrait>::load_asset_string(&self.platform, path)
            .await
    }

    //
    // Data
    //

    pub async fn load_data_bytes(&self, path: impl AsRef<Path>) -> Result<Vec<u8>, LoadFileError> {
        <FileSystemPlatform as FileSystemPlatformTrait>::load_data_bytes(&self.platform, path).await
    }

    pub async fn load_data_string(&self, path: impl AsRef<Path>) -> Result<String, LoadFileError> {
        <FileSystemPlatform as FileSystemPlatformTrait>::load_data_string(&self.platform, path)
            .await
    }

    pub async fn write_data_bytes(
        &self,
        path: impl AsRef<Path>,
        bytes: impl AsRef<[u8]>,
    ) -> Result<(), WriteFileError> {
        <FileSystemPlatform as FileSystemPlatformTrait>::write_data_bytes(
            &self.platform,
            path,
            bytes,
        )
        .await
    }

    pub async fn write_data_string(
        &self,
        path: impl AsRef<Path>,
        string: impl AsRef<str>,
    ) -> Result<(), WriteFileError> {
        <FileSystemPlatform as FileSystemPlatformTrait>::write_data_string(
            &self.platform,
            path,
            string,
        )
        .await
    }
}
