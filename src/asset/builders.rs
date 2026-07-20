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
        if let Some(handle) = self.handle {
            cache.overwrite_handle(self.value, handle)
        } else {
            cache.insert(self.value)
        }
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
}

impl<T: AssetLoader + 'static> LoadAssetBuilder<T> {
    pub fn build_default_settings(self, cache: &mut AssetCache) -> AssetHandle<T::Asset>
    where
        T::Settings: Default,
    {
        self.build_custom_settings(cache, T::Settings::default())
    }

    pub fn build_custom_settings(
        self,
        cache: &mut AssetCache,
        settings: T::Settings,
    ) -> AssetHandle<T::Asset> {
        let handle = self.handle.unwrap_or(cache.new_empty_handle());

        cache.load::<T>(handle.clone(), &self.path, settings);

        handle
    }

    pub fn handle(mut self, handle: AssetHandle<T::Asset>) -> Self {
        self.handle = Some(handle);
        self
    }

    pub fn watch(mut self, watch: bool) -> Self {
        self.watch = watch;
        self
    }
}
