mod builders;
mod cache;
mod derive;
mod handle;
mod implementations;
mod load;
mod storage;
mod types;

#[cfg(not(target_arch = "wasm32"))]
mod reload;

pub use builders::*;
pub use cache::*;
pub use derive::*;
pub use handle::*;
pub use implementations::*;
pub use load::*;
pub use storage::*;
pub use types::*;

#[cfg(not(target_arch = "wasm32"))]
pub use reload::*;

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

pub fn convert_asset<G: AssetConverter + 'static>(
    ctx: &mut crate::Context,
    cache: &mut AssetCache,
    settings: &G::Settings,
) -> ConvertAssetResult<G::TargetAsset> {
    cache.convert::<G>(ctx, settings)
}
