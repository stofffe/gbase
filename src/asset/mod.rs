mod builders;
mod cache;
mod handle;
mod implementations;
mod types;

#[cfg(not(target_arch = "wasm32"))]
mod reload;

pub use builders::*;
pub use cache::*;
pub use handle::*;
pub use implementations::*;
pub use types::*;

#[cfg(not(target_arch = "wasm32"))]
pub use reload::*;

use crate::Context;

//
// Errors
//

#[derive(thiserror::Error, Debug)]
pub enum AssetError {
    #[error("asset path not found")]
    PathNotFound,
}

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
    cache.handle_loaded(handle.clone())
}

/// Check if a specific asset is loaded
pub fn handle_just_loaded<T: Asset>(cache: &AssetCache, handle: AssetHandle<T>) -> bool {
    cache.handle_just_loaded(handle.clone())
}

pub enum GetAssetResult<'a, T: Asset> {
    Loading,
    Success(&'a T),
    Failed,
}

impl<'a, T: Asset> GetAssetResult<'a, T> {
    pub fn unwrap_loaded(self) -> &'a T {
        match self {
            GetAssetResult::Success(asset) => asset,
            GetAssetResult::Loading => panic!("Asset is still loading"),
            GetAssetResult::Failed => panic!("Asset failed to load"),
        }
    }
}

pub fn get<'a, T: Asset + 'static>(
    cache: &'a AssetCache,
    handle: AssetHandle<T>,
) -> GetAssetResult<'a, T> {
    cache.get(handle)
}

pub fn convert_asset<G: AssetConverter>(
    ctx: &mut Context,
    cache: &mut AssetCache,
    handle: AssetHandle<G::SourceAsset>,
    converter: G,
) -> ConvertAssetResult<G::TargetAsset> {
    cache.convert(ctx, handle, converter)
}
