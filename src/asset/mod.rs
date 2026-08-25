//
// Modules
//

mod builders;
mod cache;
mod dependency;
mod derive_convert;
mod derive_storage;
mod handle;
mod implementation;
mod load;
mod registry;
mod storage;

#[cfg(not(target_arch = "wasm32"))]
mod reload;

pub use builders::*;
pub use cache::*;
pub use dependency::*;
pub use derive_convert::*;
pub use derive_storage::*;
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

pub fn get<T: Asset + 'static>(
    cache: &mut AssetCache,
    handle: AssetHandle<T>,
) -> GetAssetResult<'_, T> {
    cache.get(&handle)
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

pub fn convert_derived_asset<T: AssetConverter + 'static>(
    cache: &mut AssetCache,
    settings: T::Settings,
) -> AssetHandle<T::Asset> {
    cache.convert_derived::<T>(settings)
}

pub fn get_derived_asset<T: Asset + 'static>(
    cache: &AssetCache,
    handle: AssetHandle<T>,
) -> GetDerivedResult<T> {
    cache.get_derived::<T>(&handle)
}

pub fn get_or_convert_derived_asset<T: AssetConverter + 'static>(
    cache: &mut AssetCache,
    settings: T::Settings,
) -> GetDerivedResult<T::Asset> {
    let handle = cache.convert_derived::<T>(settings);
    cache.get_derived(&handle)
}
