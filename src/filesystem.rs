use crate::Context;
use std::fs;

pub(crate) struct FileSystemContext {}

impl FileSystemContext {
    pub(crate) fn new() -> Self {
        Self {}
    }
}

//
// Commands
//

pub fn store_bytes(_ctx: &Context, path: &str, data: &[u8]) -> anyhow::Result<()> {
    let path = tmp_path_format(path);

    #[cfg(target_arch = "wasm32")]
    {
        let storage = get_local_storage();
        use base64::Engine;
        let encoded = base64::engine::general_purpose::STANDARD.encode(data);
        storage.set_item(&path, &encoded);
    }

    #[cfg(not(target_arch = "wasm32"))]
    fs::write(path, data)?;

    Ok(())
}

pub fn load_bytes(_ctx: &Context, path: &str) -> anyhow::Result<Vec<u8>> {
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
    Ok(fs::read(path)?)
}

pub fn store_str(_ctx: &Context, path: &str, data: &str) -> anyhow::Result<()> {
    let path = tmp_path_format(path);

    #[cfg(target_arch = "wasm32")]
    {
        let storage = get_local_storage();
        storage
            .set_item(&path, data)
            .map_err(|err| anyhow::anyhow!("could not set item"))?;
    }

    #[cfg(not(target_arch = "wasm32"))]
    fs::write(path, data)?;

    Ok(())
}

pub fn load_str(_ctx: &Context, path: &str) -> anyhow::Result<String> {
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
    Ok(fs::read_to_string(path)?)
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
