pub(crate) struct FileSystemContext {}

impl FileSystemContext {
    pub(crate) fn new() -> Self {
        Self {}
    }
}

//
// Commands
//

pub fn store_bytes(ctx: &Context, path: &str, data: &[u8]) {
    #[cfg(target_arch = "wasm32")]
    {
        let storage = get_local_storage();
        use base64::Engine;
        let encoded = base64::engine::general_purpose::STANDARD.encode(data);
        storage
            .set_item(path, &encoded)
            .expect("could not set data in localstorage");
    }

    #[cfg(not(target_arch = "wasm32"))]
    fs::write(Path::new(path), data).expect("could not write file");
}

pub fn load_bytes(ctx: &Context, path: &str) -> Vec<u8> {
    #[cfg(target_arch = "wasm32")]
    {
        let storage = get_local_storage();
        let data = storage
            .get_item(path)
            .expect("could not get data from localstorage")
            .expect("data does not exist");
        use base64::Engine;
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(data)
            .expect("could not decode bytes");
        return decoded.to_vec();
    }

    #[cfg(not(target_arch = "wasm32"))]
    fs::read(path).expect("could not read data from file")
}

pub fn store_str(ctx: &Context, path: &str, data: &str) {
    #[cfg(target_arch = "wasm32")]
    {
        let storage = get_local_storage();
        storage
            .set_item(path, data)
            .expect("could not set data in localstorage");
    }

    #[cfg(not(target_arch = "wasm32"))]
    fs::write(Path::new(path), data).expect("could not write file");
}

pub fn load_str(ctx: &Context, path: &str) -> String {
    #[cfg(target_arch = "wasm32")]
    {
        let storage = get_local_storage();
        let data = storage
            .get_item(path)
            .expect("could not get data from localstorage");

        return data.expect("data does not exist");
    }

    #[cfg(not(target_arch = "wasm32"))]
    fs::read_to_string(path).expect("could not read data from file")
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
/// WASM: Always use `load_bytes!`
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
/// WASM: Always use `load_str!`
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

use std::{fs, path::Path};

pub use load_b;
pub use load_s;

use crate::Context;

// /// Loads bytes from file in assets folder
// pub(crate) async fn load_bytes(&self, path: &Path) -> anyhow::Result<Vec<u8>> {
//     let path = self.res_path.join(path);
//     let path = path.to_str().ok_or(anyhow!("invalid file path"))?;
//     log::info!("load bytes from {:?}", path);
//
//     #[cfg(not(target_arch = "wasm32"))]
//     let image_data = fs::read(path)?;
//
//     #[cfg(target_arch = "wasm32")]
//     let image_data = reqwest::get(path).await?.bytes().await?.into();
//
//     Ok(image_data)
// }
//
// /// Loads string from file in assets folder
// pub(crate) async fn load_string(&self, path: &Path) -> anyhow::Result<String> {
//     let bytes = self.load_bytes(path).await?;
//     let str = String::from_utf8(bytes)?;
//     Ok(str)
// }
//
// /// Loads bytes SYNC from file in assets folder
// #[cfg(not(target_arch = "wasm32"))]
// pub(crate) fn load_bytes_sync(&self, path: &Path) -> anyhow::Result<Vec<u8>> {
//     let path = self.res_path.join(path);
//     let path = path.to_str().ok_or(anyhow!("invalid file path"))?;
//     log::info!("load bytes from {:?}", path);
//
//     let image_data = fs::read(path)?;
//
//     Ok(image_data)
// }
//
// /// Loads string SYNC from file in assets folder
// #[cfg(not(target_arch = "wasm32"))]
// pub(crate) fn load_string_sync(&self, path: &Path) -> anyhow::Result<String> {
//     let bytes = self.load_bytes_sync(path)?;
//     let str = String::from_utf8(bytes)?;
//     Ok(str)
// }

//
// Commands
//

// pub async fn load_bytes(ctx: &Context, path: impl Into<PathBuf>) -> anyhow::Result<Vec<u8>> {
//     ctx.filesystem.load_bytes(&path.into()).await
// }
//
// pub async fn load_string(ctx: &Context, path: impl Into<PathBuf>) -> anyhow::Result<String> {
//     ctx.filesystem.load_string(&path.into()).await
// }
//
// #[cfg(not(target_arch = "wasm32"))]
// pub fn load_bytes_sync(ctx: &Context, path: impl Into<PathBuf>) -> anyhow::Result<Vec<u8>> {
//     ctx.filesystem.load_bytes_sync(&path.into())
// }
//
// #[cfg(not(target_arch = "wasm32"))]
// pub fn load_string_sync(ctx: &Context, path: impl Into<PathBuf>) -> anyhow::Result<String> {
//     ctx.filesystem.load_string_sync(&path.into())
// }
