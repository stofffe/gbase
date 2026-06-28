use std::path::PathBuf;

use crate::{
    asset::{Asset, AssetCache, AssetHandle, AssetLoader},
    Context,
};

pub struct AssetBuilder {}
impl AssetBuilder {
    pub fn insert<T: Asset>(value: T) -> InsertAssetBuilder<T> {
        InsertAssetBuilder::<T> { value }
    }
    pub fn load<T: AssetLoader + 'static>(
        cache: &AssetCache,
        path: impl Into<PathBuf>,
        loader: T,
    ) -> LoadAssetBuilder<T> {
        LoadAssetBuilder::<T> {
            handle: AssetHandle::new(cache.asset_handle_ctx()),
            path: path.into(),
            loader,

            sync: false,
            watch: false,
        }
    }
}

//
// Insert
//

pub struct InsertAssetBuilder<T: Asset> {
    value: T,
}

impl<T: Asset> InsertAssetBuilder<T> {
    pub fn build(self, cache: &mut AssetCache) -> AssetHandle<T> {
        cache.insert(self.value)
    }
}

//
// Loaded
//

pub struct LoadAssetBuilder<T: AssetLoader> {
    loader: T,
    handle: AssetHandle<T::Asset>,
    path: PathBuf,

    watch: bool,
    sync: bool,
}

impl<T: AssetLoader + 'static> LoadAssetBuilder<T> {
    pub fn build(self, ctx: &Context, cache: &mut AssetCache) -> AssetHandle<T::Asset> {
        if self.watch {
            #[cfg(not(target_arch = "wasm32"))]
            cache
                .ext
                .watch::<T>(&ctx.filesystem, self.handle.clone(), &self.path);
        }

        if self.sync {
            cache.load_sync(self.handle, &self.path, self.loader)
        } else {
            cache.load::<T>(self.handle, &self.path, self.loader)
        }
    }

    pub fn watch(mut self, watch: bool) -> Self {
        self.watch = watch;
        self
    }

    pub fn sync(mut self, sync: bool) -> Self {
        self.sync = sync;
        self
    }
}
