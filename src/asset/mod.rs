mod builders;
mod cache;
mod convert;
mod handle;
mod implementations;
mod load;
mod storage;
mod types;

#[cfg(not(target_arch = "wasm32"))]
mod reload;

pub use builders::*;
pub use cache::*;
pub use convert::*;
pub use handle::*;
pub use implementations::*;
pub use load::*;
pub use storage::*;
pub use types::*;

#[cfg(not(target_arch = "wasm32"))]
pub use reload::*;

use crate::Context;

//
// Commands
//

// force reload an asset
#[cfg(not(target_arch = "wasm32"))]
pub fn reload_asset<T: AssetLoader + 'static>(
    cache: &mut AssetCache,
    handle: AssetHandle<T::Asset>,
) {
    cache.reload::<T>(handle)
}

// force reload an asset
#[cfg(not(target_arch = "wasm32"))]
pub fn reload_asset_sync<T: AssetLoader + 'static>(
    cache: &mut AssetCache,
    handle: AssetHandle<T::Asset>,
) {
    cache.reload_sync::<T>(handle)
}

/// Check if a specific asset is loaded
pub fn handle_loaded<T: Asset>(cache: &AssetCache, handle: AssetHandle<T>) -> bool {
    cache.handle_successfully_loaded(handle.clone())
}

/// Check if a specific asset is loaded
pub fn handle_just_loaded<T: Asset>(cache: &AssetCache, handle: AssetHandle<T>) -> bool {
    cache.handle_just_loaded(handle.clone())
}

pub fn get<'a, T: Asset + 'static>(
    cache: &'a AssetCache,
    handle: AssetHandle<T>,
) -> GetAssetResult<'a, T> {
    cache.get(handle)
}

pub fn convert_asset_custom_settings<G: AssetConverter>(
    ctx: &mut Context,
    cache: &mut AssetCache,
    handle: AssetHandle<G::SourceAsset>,
    settings: &G::Settings,
) -> ConvertAssetResult<G::TargetAsset> {
    cache.convert::<G>(ctx, handle, settings)
}

pub fn convert_asset_default_settings<G: AssetConverter<Settings: Default>>(
    ctx: &mut Context,
    cache: &mut AssetCache,
    handle: AssetHandle<G::SourceAsset>,
) -> ConvertAssetResult<G::TargetAsset> {
    cache.convert::<G>(ctx, handle, &G::Settings::default())
}
