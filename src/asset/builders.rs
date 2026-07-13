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

    pub fn load<T: AssetLoader>(path: impl Into<PathBuf>) -> LoadAssetBuilder<T> {
        LoadAssetBuilder::<T> {
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
    path: PathBuf,

    handle: Option<AssetHandle<T::Asset>>,
    watch: bool,
    sync: bool,
}

impl<T: AssetLoader + 'static> LoadAssetBuilder<T> {
    pub fn build_default_settings(
        self,
        ctx: &Context,
        cache: &mut AssetCache,
    ) -> AssetHandle<T::Asset>
    where
        T::Settings: Default,
    {
        self.build_custom_settings(ctx, cache, T::Settings::default())
    }

    pub fn build_custom_settings(
        self,
        ctx: &Context,
        cache: &mut AssetCache,
        settings: T::Settings,
    ) -> AssetHandle<T::Asset> {
        let handle = self.handle.unwrap_or(cache.new_empty_handle());

        #[cfg(not(target_arch = "wasm32"))]
        if self.watch {
            cache
                .ext
                .watch_asset::<T>(&ctx.filesystem, handle.clone(), &self.path);
        }

        #[cfg(not(target_arch = "wasm32"))]
        if self.sync {
            return cache.load_sync::<T>(handle, &self.path, settings);
        }

        cache.load::<T>(handle, &self.path, settings)
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
