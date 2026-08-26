//
// Modules
//

mod cache;
mod convert;
mod dependency;
mod handle;
mod implementation;
mod load;
mod registry;
mod storage;

#[cfg(not(target_arch = "wasm32"))]
mod reload;

pub use cache::*;
pub use convert::*;
pub use dependency::*;
pub use handle::*;
pub use implementation::*;
pub use load::*;
pub use registry::*;
pub use storage::*;

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

/// Check if a specific asset is loaded
pub fn handle_loaded<T: Asset>(cache: &mut AssetCache, handle: AssetHandle<T>) -> bool {
    cache.handle_successfully_loaded(handle.clone())
}

/// Check if a specific asset is loaded
pub fn handle_just_loaded<T: Asset>(cache: &AssetCache, handle: AssetHandle<T>) -> bool {
    cache.handle_just_loaded(handle.clone())
}

pub fn insert_asset<T: Asset + 'static>(cache: &mut AssetCache, asset: T) -> AssetHandle<T> {
    cache.insert_asset(asset)
}

pub fn load_asset<T: AssetLoader + 'static>(
    cache: &mut AssetCache,
    settings: &T::Settings,
) -> AssetHandle<T::Asset> {
    cache.load_asset::<T>(settings)
}

pub fn get_asset<T: Asset + 'static>(
    cache: &mut AssetCache,
    handle: AssetHandle<T>,
) -> GetAssetResult<'_, T> {
    // tracing::info!("get asset");
    cache.get_asset(&handle)
}

#[deprecated(since = "1.0.0", note = "use `new_function` instead")]
pub fn convert_asset<G: AssetConverter + 'static>(
    ctx: &mut Context,
    cache: &mut AssetCache,
    settings: &G::Settings,
) -> ConvertAssetResult<G::Asset> {
    todo!()
    // cache.convert::<G>(ctx, settings)
}

pub fn convert_asset_new<T: AssetConverter + 'static>(
    cache: &mut AssetCache,
    settings: T::Settings,
) -> AssetHandle<T::Asset> {
    cache.convert_asset::<T>(settings)
}

pub fn get_or_convert_asset<T: AssetConverter + 'static>(
    cache: &mut AssetCache,
    settings: T::Settings,
) -> GetAssetResult<'_, T::Asset> {
    // tracing::info!("get or convert asset");
    let handle = cache.convert_asset::<T>(settings);
    cache.get_asset(&handle)
}

pub fn debug_asset_dependency_graph(cache: &AssetCache) {
    cache.debug_asset_dependency_graph();
}
