use std::{
    fs,
    path::{self, Path, PathBuf},
};

use anyhow::anyhow;

use crate::Context;

pub(crate) struct FileSystemContext {
    res_path: PathBuf,
}

impl FileSystemContext {
    pub(crate) fn update(&mut self) {}

    pub(crate) fn new(assets_path: &PathBuf) -> Self {
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
}

//
// Commands
//

pub async fn load_bytes(ctx: &Context, path: &Path) -> anyhow::Result<Vec<u8>> {
    ctx.filesystem.load_bytes(path).await
}
