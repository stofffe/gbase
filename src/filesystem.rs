use crate::Context;
use anyhow::anyhow;
use std::{
    fs,
    path::{self, Path, PathBuf},
};

pub(crate) struct FileSystemContext {
    res_path: PathBuf,
}

impl FileSystemContext {
    pub(crate) fn new(assets_path: &Path) -> Self {
        // set default resource path
        let mut res_path = path::PathBuf::new();

        #[cfg(target_arch = "wasm32")]
        {
            // TODO handle error
            let origin = web_sys::window()
                .expect("could not get window")
                .location()
                .origin()
                .expect("could not get origin");
            res_path.push(origin);
        }

        res_path.push(assets_path);

        Self { res_path }
    }

    /// Loads bytes from file in assets folder
    pub(crate) async fn load_bytes(&self, path: &Path) -> anyhow::Result<Vec<u8>> {
        let path = self.res_path.join(path);
        let path = path.to_str().ok_or(anyhow!("invalid file path"))?;
        log::info!("load bytes from {:?}", path);

        #[cfg(not(target_arch = "wasm32"))]
        let image_data = fs::read(path)?;

        #[cfg(target_arch = "wasm32")]
        let image_data = reqwest::get(path).await?.bytes().await?.into();

        Ok(image_data)
    }

    /// Loads string from file in assets folder
    pub(crate) async fn load_string(&self, path: &Path) -> anyhow::Result<String> {
        let bytes = self.load_bytes(path).await?;
        let str = String::from_utf8(bytes)?;
        Ok(str)
    }

    /// Loads bytes SYNC from file in assets folder
    pub(crate) fn load_bytes_sync(&self, path: &Path) -> anyhow::Result<Vec<u8>> {
        #[cfg(target_arch = "wasm32")]
        panic!("load_bytes_sync not supported on wasm32");

        let path = self.res_path.join(path);
        let path = path.to_str().ok_or(anyhow!("invalid file path"))?;
        log::info!("load bytes from {:?}", path);

        let image_data = fs::read(path)?;

        Ok(image_data)
    }

    /// Loads string SYNC from file in assets folder
    pub(crate) fn load_string_sync(&self, path: &Path) -> anyhow::Result<String> {
        let bytes = self.load_bytes_sync(path)?;
        let str = String::from_utf8(bytes)?;
        Ok(str)
    }
}

//
// Commands
//

pub async fn load_bytes(ctx: &Context, path: impl Into<PathBuf>) -> anyhow::Result<Vec<u8>> {
    ctx.filesystem.load_bytes(&path.into()).await
}

pub async fn load_string(ctx: &Context, path: impl Into<PathBuf>) -> anyhow::Result<String> {
    ctx.filesystem.load_string(&path.into()).await
}

pub fn load_bytes_sync(ctx: &Context, path: impl Into<PathBuf>) -> anyhow::Result<Vec<u8>> {
    ctx.filesystem.load_bytes_sync(&path.into())
}

pub fn load_string_sync(ctx: &Context, path: impl Into<PathBuf>) -> anyhow::Result<String> {
    ctx.filesystem.load_string_sync(&path.into())
}
