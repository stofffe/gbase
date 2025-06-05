mod cache;
mod handle;
mod implementations;
mod types;

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

//
// Commands
//

pub fn insert<T: Asset + 'static>(ctx: &mut Context, asset: T) -> AssetHandle<T> {
    ctx.assets.asset_cache.insert(asset)
}

pub fn get<T: Asset + 'static>(ctx: &Context, handle: AssetHandle<T>) -> Option<&T> {
    ctx.assets.asset_cache.get(handle)
}

pub fn get_mut<T: Asset + 'static>(ctx: &mut Context, handle: AssetHandle<T>) -> Option<&mut T> {
    ctx.assets.asset_cache.get_mut(handle)
}

pub fn load<T: Asset + LoadableAsset + 'static>(
    ctx: &mut Context,
    path: &std::path::Path,
    sync: bool,
) -> AssetHandle<T> {
    ctx.assets.asset_cache.load(path, sync)
}

pub fn load_watch<T: Asset + LoadableAsset + 'static>(
    ctx: &mut Context,
    path: &std::path::Path,
    sync: bool,
) -> AssetHandle<T> {
    ctx.assets.asset_cache.load_watch(path, sync)
}

pub fn load_write<T: Asset + LoadableAsset + WriteableAsset + 'static>(
    ctx: &mut Context,
    path: &std::path::Path,
    sync: bool,
) -> AssetHandle<T> {
    ctx.assets.asset_cache.load_write(path, sync)
}

pub fn load_watch_write<T: Asset + LoadableAsset + WriteableAsset + 'static>(
    ctx: &mut Context,
    path: &std::path::Path,
    sync: bool,
) -> AssetHandle<T> {
    ctx.assets.asset_cache.load_watch_write(path, sync)
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
