mod platforms;
mod runtime;

pub use platforms::*;
pub use runtime::*;

use crate::{Context, ContextBuilder};
use std::path::{self, Path, PathBuf};

pub struct FileSystemContext {
    runtime: FileSystemRuntime,
}

impl FileSystemContext {
    pub fn new(builder: &ContextBuilder) -> Self {
        let runtime = FileSystemRuntime::new(builder);
        Self { runtime }
    }

    pub fn runtime(&self) -> FileSystemRuntime {
        self.runtime.clone()
    }

    //
    // Normal
    //

    pub async fn load_bytes(&self, path: impl AsRef<Path>) -> Result<Vec<u8>, LoadFileError> {
        self.runtime.load_bytes(path).await
    }
    pub async fn load_string(&self, path: impl AsRef<Path>) -> Result<String, LoadFileError> {
        self.runtime.load_string(path).await
    }
    pub async fn write_bytes(
        &self,
        path: impl AsRef<Path>,
        bytes: impl AsRef<[u8]>,
    ) -> Result<(), WriteFileError> {
        self.runtime.write_bytes(path, bytes).await
    }
    pub async fn write_string(
        &self,
        path: impl AsRef<Path>,
        string: impl AsRef<str>,
    ) -> Result<(), WriteFileError> {
        self.runtime.write_string(path, string).await
    }

    //
    // Asset
    //

    pub async fn load_asset_bytes(&self, path: impl AsRef<Path>) -> Result<Vec<u8>, LoadFileError> {
        self.runtime.load_asset_bytes(path).await
    }
    pub async fn load_asset_string(&self, path: impl AsRef<Path>) -> Result<String, LoadFileError> {
        self.runtime.load_asset_string(path).await
    }

    //
    // Data
    //

    pub async fn load_data_bytes(&self, path: impl AsRef<Path>) -> Result<Vec<u8>, LoadFileError> {
        self.runtime.load_data_bytes(path).await
    }
    pub async fn load_data_string(&self, path: impl AsRef<Path>) -> Result<String, LoadFileError> {
        self.runtime.load_data_string(path).await
    }
    pub async fn write_data_bytes(
        &self,
        path: impl AsRef<Path>,
        bytes: impl AsRef<[u8]>,
    ) -> Result<(), WriteFileError> {
        self.runtime.write_data_bytes(path, bytes).await
    }
    pub async fn write_data_string(
        &self,
        path: impl AsRef<Path>,
        string: impl AsRef<str>,
    ) -> Result<(), WriteFileError> {
        self.runtime.write_data_string(path, string).await
    }
}

pub fn normalize_path(path: impl AsRef<std::path::Path>) -> PathBuf {
    let mut out = PathBuf::new();

    for component in path.as_ref().components() {
        match component {
            path::Component::CurDir => {
                // skip "."
            }

            path::Component::ParentDir => {
                out.pop();
            }

            path::Component::Normal(part) => {
                out.push(part);
            }

            path::Component::RootDir | path::Component::Prefix(_) => {
                out.clear();
                out.push(component);
            }
        }
    }

    out
}

#[derive(thiserror::Error, Debug)]
pub enum LoadFileError {
    #[error("file not found")]
    FileNotFound,
    #[error("invalid path")]
    InvalidPath,
    #[error("other error: {0}")]
    Other(Box<dyn std::error::Error + Send + Sync>),

    #[error("internal")]
    Placeholder,
}

#[derive(thiserror::Error, Debug)]
pub enum WriteFileError {
    #[error("invalid path")]
    InvalidPath,
    #[error("other error: {0}")]
    Other(Box<dyn std::error::Error + Send + Sync>),
}

//
// Commands
//

pub fn get_filesystem_runtime(ctx: &Context) -> FileSystemRuntime {
    ctx.filesystem.runtime()
}

//
// Normal
//

pub async fn load_bytes(ctx: &Context, path: impl AsRef<Path>) -> Result<Vec<u8>, LoadFileError> {
    ctx.filesystem.load_bytes(path).await
}

pub async fn load_string(ctx: &Context, path: impl AsRef<Path>) -> Result<String, LoadFileError> {
    ctx.filesystem.load_string(path).await
}

pub async fn write_bytes(
    ctx: &Context,
    path: impl AsRef<Path>,
    bytes: impl AsRef<[u8]>,
) -> Result<(), WriteFileError> {
    ctx.filesystem.write_bytes(path, bytes).await
}

pub async fn write_string(
    ctx: &Context,
    path: impl AsRef<Path>,
    string: impl AsRef<str>,
) -> Result<(), WriteFileError> {
    ctx.filesystem.write_string(path, string).await
}

//
// Asset
//

pub async fn load_asset_bytes(
    ctx: &Context,
    path: impl AsRef<Path>,
) -> Result<Vec<u8>, LoadFileError> {
    ctx.filesystem.load_asset_bytes(path).await
}

pub async fn load_asset_string(
    ctx: &Context,
    path: impl AsRef<Path>,
) -> Result<String, LoadFileError> {
    ctx.filesystem.load_asset_string(path).await
}

//
// data
//

pub async fn load_data_bytes(
    ctx: &Context,
    path: impl AsRef<Path>,
) -> Result<Vec<u8>, LoadFileError> {
    ctx.filesystem.load_data_bytes(path).await
}

pub async fn load_data_string(
    ctx: &Context,
    path: impl AsRef<Path>,
) -> Result<String, LoadFileError> {
    ctx.filesystem.load_data_string(path).await
}

pub async fn write_data_bytes(
    ctx: &Context,
    path: impl AsRef<Path>,
    bytes: impl AsRef<[u8]>,
) -> Result<(), WriteFileError> {
    ctx.filesystem.write_data_bytes(path, bytes).await
}

pub async fn write_data_string(
    ctx: &Context,
    path: impl AsRef<Path>,
    string: impl AsRef<str>,
) -> Result<(), WriteFileError> {
    ctx.filesystem.write_data_string(path, string).await
}

// TODO: use filesystem context
/// Path to data storage folder
pub fn tmp_path() -> &'static str {
    "tmp"
}

#[cfg(test)]
mod tests {
    use crate::filesystem::normalize_path;
    use std::path::PathBuf;

    #[test]
    fn test_dot_segments() {
        assert_eq!(normalize_path("a/./b"), PathBuf::from("a/b"));
    }

    #[test]
    fn test_parent_segments() {
        assert_eq!(normalize_path("a/b/../c"), PathBuf::from("a/c"));
    }

    #[test]
    fn test_multiple_parents() {
        assert_eq!(normalize_path("a/b/c/../../d"), PathBuf::from("a/d"));
    }

    #[test]
    fn test_leading_parent() {
        // behavior choice: we allow popping beyond root -> stays minimal
        assert_eq!(normalize_path("../a"), PathBuf::from("a"));
    }

    #[test]
    fn test_complex() {
        assert_eq!(normalize_path("./a/../b/./c"), PathBuf::from("b/c"));
    }
}
