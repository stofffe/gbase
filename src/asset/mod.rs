mod cache;
mod handle;
mod implementations;
mod types;

use std::{
    marker::PhantomData,
    path::{Path, PathBuf},
};

pub use cache::*;
pub use handle::*;
pub use implementations::*;
pub use types::*;

use crate::{render::ArcHandle, Context};

//
// Context
//

pub(crate) struct AssetContext {
    pub(crate) asset_cache: AssetCache,
}

impl AssetContext {
    pub fn new() -> Self {
        let asset_cache = AssetCache::new();
        Self { asset_cache }
    }
}

pub struct AssetBuilder {}
impl AssetBuilder {
    pub fn insert<T: Asset>(value: T) -> InsertAssetBuilder<T> {
        InsertAssetBuilder::<T> { value }
    }
    pub fn load<T: Asset + LoadableAsset>(path: impl Into<PathBuf>) -> LoadedAssetBuilder<T> {
        LoadedAssetBuilder::<T> {
            handle: AssetHandle::new(),
            path: path.into(),
            ty: PhantomData,
            on_load: None,
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
    pub fn build(self, ctx: &mut Context) -> AssetHandle<T> {
        ctx.assets.asset_cache.insert(self.value)
    }
}

//
// Loaded
//

pub struct LoadedAssetBuilder<T: Asset + LoadableAsset> {
    handle: AssetHandle<T>,
    path: PathBuf,
    ty: PhantomData<T>,

    on_load: Option<TypedAssetOnLoadFn<T>>,
}

impl<T: Asset + LoadableAsset + WriteableAsset> LoadedAssetBuilder<T> {
    pub fn write(self, ctx: &mut Context) -> Self {
        ctx.assets
            .asset_cache
            .write::<T>(self.handle.clone(), &self.path);
        self
    }
}

impl<T: Asset + LoadableAsset> LoadedAssetBuilder<T> {
    pub fn watch(self, ctx: &mut Context) -> Self {
        ctx.assets
            .asset_cache
            .watch::<T>(self.handle.clone(), &self.path);
        self
    }
}

impl<T: Asset + LoadableAsset> LoadedAssetBuilder<T> {
    pub fn on_load<F: Fn(&mut T) + Send + Sync + 'static>(mut self, callback: F) -> Self {
        self.on_load = Some(Box::new(callback));
        self
    }

    pub fn build(self, ctx: &mut Context) -> AssetHandle<T> {
        ctx.assets
            .asset_cache
            .load::<T>(self.handle, &self.path, self.on_load)
    }
}

//
// Commands
//

pub fn wait_all(ctx: &mut Context) {
    ctx.assets.asset_cache.wait_all();
}

pub fn wait_for<T: Asset + LoadableAsset>(ctx: &mut Context, handle: AssetHandle<T>) {
    ctx.assets.asset_cache.wait_for(handle);
}

pub fn get<T: Asset + 'static>(ctx: &Context, handle: AssetHandle<T>) -> Option<&T> {
    ctx.assets.asset_cache.get(handle)
}

pub fn get_mut<T: Asset + 'static>(ctx: &mut Context, handle: AssetHandle<T>) -> Option<&mut T> {
    ctx.assets.asset_cache.get_mut(handle)
}

pub fn convert_asset<G: ConvertableRenderAsset>(
    ctx: &mut Context,
    handle: AssetHandle<G::SourceAsset>,
    params: &G::Params,
) -> Option<ArcHandle<G>> {
    ctx.assets.asset_cache.convert(
        &ctx.render.device,
        &ctx.render.queue,
        &mut ctx.render.cache,
        handle,
        params,
    )
}
// pub fn insert<T: Asset + 'static>(ctx: &mut Context, asset: T) -> AssetHandle<T> {
//     ctx.assets.asset_cache.insert(asset)
// }

// pub fn load<T: Asset + LoadableAsset + 'static>(
//     ctx: &mut Context,
//     path: &std::path::Path,
//     sync: bool,
// ) -> AssetHandle<T> {
//     ctx.assets.asset_cache.load(path, sync)
// }
//
// pub fn load_watch<T: Asset + LoadableAsset + 'static>(
//     ctx: &mut Context,
//     path: &std::path::Path,
//     sync: bool,
// ) -> AssetHandle<T> {
//     ctx.assets.asset_cache.load_watch(path, sync)
// }
//
// pub fn load_write<T: Asset + LoadableAsset + WriteableAsset + 'static>(
//     ctx: &mut Context,
//     path: &std::path::Path,
//     sync: bool,
// ) -> AssetHandle<T> {
//     ctx.assets.asset_cache.load_write(path, sync)
// }
//
// pub fn load_watch_write<T: Asset + LoadableAsset + WriteableAsset + 'static>(
//     ctx: &mut Context,
//     path: &std::path::Path,
//     sync: bool,
// ) -> AssetHandle<T> {
//     ctx.assets.asset_cache.load_watch_write(path, sync)
// }
