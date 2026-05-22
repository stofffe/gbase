use std::path::PathBuf;

use crate::Context;

#[derive(Clone, Debug)]
pub struct FileSystemContext {
    config: std::sync::Arc<FileSystemConfig>,
}

#[derive(Debug)]
pub struct FileSystemConfig {
    #[cfg(target_arch = "wasm32")]
    base_folder_path: reqwest::Url,
    #[cfg(not(target_arch = "wasm32"))]
    base_folder_path: PathBuf,
}

impl FileSystemContext {
    pub(crate) fn new() -> Self {
        let asset_folder = PathBuf::from(".");

        #[cfg(target_arch = "wasm32")]
        let base_folder_path = {
            let window = web_sys::window().expect("could not get window");
            let location = window.location();
            let path = location.origin().expect("could not get origin");
            let url = reqwest::Url::parse(&path).expect("could not base path");
            url
        };

        #[cfg(not(target_arch = "wasm32"))]
        let base_folder_path = if asset_folder.is_absolute() {
            asset_folder
        } else {
            std::env::current_dir()
                .expect("could not get current working dir")
                .join(&asset_folder)
        };

        dbg!(&base_folder_path);

        Self {
            config: std::sync::Arc::new(FileSystemConfig { base_folder_path }),
        }
    }
}

// TODO: store orign/abs path to asset dir in filesystem
#[derive(thiserror::Error, Debug)]
pub enum LoadFileError {
    #[error("file not found")]
    FileNotFound,
    #[error("invalid path")]
    InvalidPath,
    #[error("other error: {0}")]
    Other(Box<dyn std::error::Error + Send + Sync>),
}

impl FileSystemContext {
    pub async fn load_bytes(
        &self,
        path: impl AsRef<std::path::Path>,
    ) -> Result<Vec<u8>, LoadFileError> {
        // cache address
        #[cfg(target_arch = "wasm32")]
        let bytes = {
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
            bytes.to_vec()
        };

        #[cfg(not(target_arch = "wasm32"))]
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
        #[cfg(target_arch = "wasm32")]
        let str = {
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
                .text()
                .await
                .map_err(|err| LoadFileError::Other(Box::new(err)))?;
            bytes
        };

        #[cfg(not(target_arch = "wasm32"))]
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

//
// Commands
//

pub fn store_bytes_tmp(_ctx: &Context, path: &str, data: &[u8]) -> anyhow::Result<()> {
    let path = tmp_path_format(path);

    #[cfg(target_arch = "wasm32")]
    {
        let storage = get_local_storage();
        use base64::Engine;
        let encoded = base64::engine::general_purpose::STANDARD.encode(data);
        storage.set_item(&path, &encoded);
    }

    #[cfg(not(target_arch = "wasm32"))]
    std::fs::write(path, data)?;

    Ok(())
}

pub fn load_bytes_tmp(_ctx: &Context, path: &str) -> anyhow::Result<Vec<u8>> {
    let path = tmp_path_format(path);

    #[cfg(target_arch = "wasm32")]
    {
        let storage = get_local_storage();
        let data = storage
            .get_item(&path)
            .map_err(|e| anyhow::anyhow!("could not get item"))?
            .ok_or(anyhow::anyhow!("empty"))?;
        use base64::Engine;
        let decoded = base64::engine::general_purpose::STANDARD.decode(data)?;
        return Ok(decoded.to_vec());
    }

    #[cfg(not(target_arch = "wasm32"))]
    Ok(std::fs::read(path)?)
}

pub fn store_str_tmp(_ctx: &Context, path: &str, data: &str) -> anyhow::Result<()> {
    let path = tmp_path_format(path);

    #[cfg(target_arch = "wasm32")]
    {
        let storage = get_local_storage();
        storage
            .set_item(&path, data)
            .map_err(|err| anyhow::anyhow!("could not set item"))?;
    }

    #[cfg(not(target_arch = "wasm32"))]
    std::fs::write(path, data)?;

    Ok(())
}

pub fn load_str_tmp(_ctx: &Context, path: &str) -> anyhow::Result<String> {
    let path = tmp_path_format(path);

    #[cfg(target_arch = "wasm32")]
    {
        let storage = get_local_storage();
        let data = storage
            .get_item(&path)
            .map_err(|err| anyhow::anyhow!("could not get item"))?
            .ok_or(anyhow::anyhow!("empty"));
        return data;
    }

    #[cfg(not(target_arch = "wasm32"))]
    Ok(std::fs::read_to_string(path)?)
}

#[cfg(target_arch = "wasm32")]
fn get_local_storage() -> web_sys::Storage {
    let window = web_sys::window().expect("could not get window");
    let storage = window
        .local_storage()
        .expect("could not get local storage")
        .expect("local storage is empty");
    storage
}

/// Path to temporary storage folder
pub fn tmp_path() -> &'static str {
    "assets/tmp"
}

fn tmp_path_format(path: &str) -> String {
    format!("{}/{}", tmp_path(), path)
}

/// Load bytes from assets folder
///
/// # Input/Output
///
/// Path from \[PROJECT ROOT\]/assets/ => std::io::Result\<??\>
///
/// # Mode & Platform
///
/// Debug:      Load using `std::fs::read`
/// Release:    Load using `load_bytes!`
///
/// WASM: Always uses `load_bytes!`
///
/// # Examples
/// ```
/// use gbase::filesystem;
/// let shader_bytes = filesystem::load_b!("shaders/shader.wgsl").unwrap();
/// ```
///
#[macro_export]
macro_rules! load_b {
    ($path:literal) => {
        if cfg!(target_arch = "wasm32") || cfg!(not(debug_assertions)) {
            Ok(include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/", $path)).to_vec())
        } else {
            std::fs::read(concat!("assets/", $path))
        }
    };
}

/// Load string from assets folder
///
/// # Input/Output
///
/// Path from \[PROJECT ROOT\]/assets/ => std::io::Result\<String\>
///
/// # Mode & Platform
///
/// Debug:      Load using `std::fs::read_to_string`
/// Release:    Load using `load_str!`
///
/// WASM: Always uses `load_str!`
///
/// # Examples
/// ```
/// use gbase::filesystem;
/// let shader_str = filesystem::load_s!("shaders/shader.wgsl").unwrap();
/// ```
///
#[macro_export]
macro_rules! load_s {
    ($path:literal) => {
        if cfg!(target_arch = "wasm32") || cfg!(not(debug_assertions)) {
            Ok(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/", $path)).to_string())
        } else {
            std::fs::read_to_string(concat!("assets/", $path))
        }
    };
}

pub use load_b;
pub use load_s;
