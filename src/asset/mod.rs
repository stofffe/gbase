mod cache;
mod handle;
mod implementations;
mod types;

pub use cache::*;
pub use handle::*;
pub use implementations::*;
pub use types::*;

use crate::{render::ArcHandle, Context};
use std::path::PathBuf;

//
// Builder
//

pub struct AssetBuilder {}
impl AssetBuilder {
    pub fn insert<T: Asset>(value: T) -> InsertAssetBuilder<T> {
        InsertAssetBuilder::<T> { value }
    }
    pub fn load<T: AssetLoader + 'static>(
        path: impl Into<PathBuf>,
        loader: T,
    ) -> LoadedAssetBuilder<T> {
        LoadedAssetBuilder::<T> {
            loader,
            handle: AssetHandle::new(),
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

pub struct LoadedAssetBuilder<T: AssetLoader> {
    loader: T,
    handle: AssetHandle<T::Asset>,
    path: PathBuf,
}

// TODO: can these just store bool instead?
impl<T: AssetWriter> LoadedAssetBuilder<T> {
    #[cfg(not(target_arch = "wasm32"))]
    pub fn write(self, cache: &mut AssetCache) -> Self {
        cache.ext.write::<T>(self.handle.clone(), &self.path);
        self
    }
}

// TODO: can these just store bool instead?
impl<T: AssetLoader + 'static> LoadedAssetBuilder<T> {
    pub fn watch(self, cache: &mut AssetCache) -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        cache
            .ext
            .watch::<T>(self.handle.clone(), &self.path, self.loader.clone()); //TODO: make this arc?
        self
    }
}

impl<T: AssetLoader + 'static> LoadedAssetBuilder<T> {
    pub fn build(self, cache: &mut AssetCache) -> AssetHandle<T::Asset> {
        cache.load::<T>(self.handle, &self.path, self.loader)
    }
}

//
// Commands
//

/// Check if all current assets are loaded
pub fn all_loaded(cache: &AssetCache) -> bool {
    cache.all_loaded()
}

/// Check if a specific asset is loaded
pub fn handle_loaded<T: Asset>(cache: &AssetCache, handle: AssetHandle<T>) -> bool {
    cache.handle_loaded(handle.clone())
}

/// Check if a specific asset is loaded
pub fn handle_just_loaded<T: Asset>(cache: &AssetCache, handle: AssetHandle<T>) -> bool {
    cache.handle_just_loaded(handle.clone())
}

pub fn get<T: Asset + 'static>(cache: &AssetCache, handle: AssetHandle<T>) -> Option<&T> {
    cache.get(handle)
}

pub fn get_mut<T: Asset + 'static>(
    cache: &mut AssetCache,
    handle: AssetHandle<T>,
) -> Option<&mut T> {
    cache.get_mut(handle)
}

pub fn convert_asset<G: ConvertableRenderAsset>(
    ctx: &mut Context,
    cache: &mut AssetCache,
    handle: AssetHandle<G::SourceAsset>,
) -> Option<ArcHandle<G>> {
    cache.convert(ctx, handle)
}
