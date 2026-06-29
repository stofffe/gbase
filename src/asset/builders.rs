use std::path::PathBuf;

use crate::{
    asset::{Asset, AssetCache, AssetHandle, AssetLoader},
    Context,
};

pub struct AssetBuilder {}
impl AssetBuilder {
    pub fn insert<T: Asset>(value: T) -> InsertAssetBuilder<T> {
        InsertAssetBuilder::<T> {
            value,
            handle: None,
        }
    }
    pub fn load<T: AssetLoader + 'static>(
        cache: &AssetCache, // TODO: will this be needed later?
        path: impl Into<PathBuf>,
        loader: T,
    ) -> LoadAssetBuilder<T> {
        LoadAssetBuilder::<T> {
            loader,
            path: path.into(),

            handle: None,
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

    handle: Option<AssetHandle<T>>,
}

impl<T: Asset> InsertAssetBuilder<T> {
    pub fn build(self, cache: &mut AssetCache) -> AssetHandle<T> {
        let handle = self.handle.unwrap_or(cache.new_empty_handle());

        cache.insert_existing_handle(self.value, handle)
    }

    pub fn handle(mut self, handle: AssetHandle<T>) -> Self {
        self.handle = Some(handle);
        self
    }
}

//
// Loaded
//

pub struct LoadAssetBuilder<T: AssetLoader> {
    loader: T,
    path: PathBuf,

    handle: Option<AssetHandle<T::Asset>>,
    watch: bool,
    sync: bool,
}

impl<T: AssetLoader + 'static> LoadAssetBuilder<T> {
    pub fn build(self, ctx: &Context, cache: &mut AssetCache) -> AssetHandle<T::Asset> {
        let handle = self.handle.unwrap_or(cache.new_empty_handle());

        if self.watch {
            #[cfg(not(target_arch = "wasm32"))]
            cache
                .ext
                .watch::<T>(&ctx.filesystem, handle.clone(), &self.path);
        }

        #[cfg(not(target_arch = "wasm32"))]
        if self.sync {
            return cache.load_sync(handle, &self.path, self.loader);
        }

        cache.load::<T>(handle, &self.path, self.loader)
    }

    pub fn handle(mut self, handle: AssetHandle<T::Asset>) -> Self {
        self.handle = Some(handle);
        self
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
