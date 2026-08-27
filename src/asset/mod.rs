//
// Modules
//

mod cache;
mod convert;
mod dependency;
mod handle;
mod implementation;
mod insert;
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
pub use insert::*;
pub use load::*;
pub use registry::*;
pub use storage::*;

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
pub fn handle_loaded<T: Asset>(cache: &mut AssetCache, handle: AssetHandle<T>) -> bool {
    cache.handle_successfully_loaded(handle.clone())
}

/// Check if a specific asset is loaded
pub fn handle_just_loaded<T: Asset>(cache: &AssetCache, handle: AssetHandle<T>) -> bool {
    cache.handle_just_loaded(handle.clone())
}

/// Inserts a new asset
///
/// If possible the AssetHandle will be reused depeneding on key
pub fn insert_asset<T: Asset + 'static, I: AssetInserter + 'static>(
    cache: &mut AssetCache,
    key: &I::Key,
    asset: T,
) -> AssetHandle<T> {
    cache.insert_asset::<T, I>(key, asset)
}

/// Inserts a new asset WITHOUT any caching
///
/// Will always create a new AssetHandle
pub fn insert_asset_force<T: Asset + 'static>(cache: &mut AssetCache, asset: T) -> AssetHandle<T> {
    cache.insert_asset_force::<T>(asset)
}

pub fn load_asset<T: AssetLoader + 'static>(
    cache: &mut AssetCache,
    settings: &T::Settings,
) -> AssetHandle<T::Asset> {
    cache.load_asset::<T>(settings)
}

pub fn convert_asset<T: AssetConverter + 'static>(
    cache: &mut AssetCache,
    settings: &T::Settings,
) -> AssetHandle<T::Asset> {
    cache.convert_asset::<T>(settings)
}

pub fn get_asset<T: Asset + 'static>(
    cache: &mut AssetCache,
    handle: AssetHandle<T>,
) -> GetAssetResult<'_, T> {
    cache.get_asset(&handle)
}

pub fn get_or_convert_asset<'a, T: AssetConverter + 'static>(
    cache: &'a mut AssetCache,
    settings: &T::Settings,
) -> GetAssetResult<'a, T::Asset> {
    let handle = cache.convert_asset::<T>(settings);
    cache.get_asset(&handle)
}

pub fn debug_asset_dependency_graph(cache: &AssetCache) {
    cache.debug_asset_dependency_graph();
}
