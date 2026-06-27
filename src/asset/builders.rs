use std::path::PathBuf;

use crate::{
    asset::{Asset, AssetCache, AssetHandle, AssetLoader, AssetWriter},
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
            loader,
            handle: AssetHandle::new(cache.asset_handle_ctx()),
            path: path.into(),
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
}

// TODO: can these just store bool instead?
impl<T: AssetWriter> LoadAssetBuilder<T> {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn write(self, cache: &mut AssetCache) -> Self {
        cache.ext.write::<T>(self.handle.clone(), &self.path);
        self
    }
}

// TODO: can these just store bool instead?
impl<T: AssetLoader + 'static> LoadAssetBuilder<T> {
    pub fn watch(self, ctx: &Context, cache: &mut AssetCache) -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        cache.ext.watch::<T>(
            &ctx.filesystem,
            self.handle.clone(),
            &self.path,
            self.loader.clone(),
        ); //TODO: make this arc?
        self
    }
}

impl<T: AssetLoader + 'static> LoadAssetBuilder<T> {
    pub fn build(self, cache: &mut AssetCache) -> AssetHandle<T::Asset> {
        cache.load::<T>(self.handle, &self.path, self.loader)
    }
}

// Loaded Sync (non wasm)

#[cfg(not(target_arch = "wasm32"))]
pub struct LoadSyncAssetBuilder<T: AssetLoader> {
    loader: T,
    handle: AssetHandle<T::Asset>,
    path: PathBuf,
}
#[cfg(not(target_arch = "wasm32"))]
impl<T: AssetWriter> LoadSyncAssetBuilder<T> {
    pub fn write(self, cache: &mut AssetCache) -> Self {
        cache.ext.write::<T>(self.handle.clone(), &self.path);
        self
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl<T: AssetLoader + 'static> LoadSyncAssetBuilder<T> {
    pub fn watch(self, ctx: &Context, cache: &mut AssetCache) -> Self {
        cache.ext.watch::<T>(
            &ctx.filesystem,
            self.handle.clone(),
            &self.path,
            self.loader.clone(),
        ); //TODO: make this arc?
        self
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl<T: AssetLoader + 'static> LoadSyncAssetBuilder<T> {
    pub fn build(self, cache: &mut AssetCache) -> AssetHandle<T::Asset> {
        cache.load_sync::<T>(self.handle, &self.path, self.loader)
    }
}
