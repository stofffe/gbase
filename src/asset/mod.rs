//
// Modules
//

mod backend;
mod dependency;
mod frontend;
mod handle;
mod implementation;
mod insert;
mod load;
mod registry;
mod snapshot;
mod storage;

#[cfg(feature = "asset_reload")]
mod reload;

pub use backend::*;
pub use dependency::*;
pub use frontend::*;
pub use handle::*;
pub use implementation::*;
pub use insert::*;
pub use load::*;
pub use registry::*;
pub use snapshot::*;
pub use storage::*;

#[cfg(feature = "asset_reload")]
pub use reload::*;

//
// Commands
//

// force reload an asset
#[cfg(feature = "asset_reload")]
pub fn reload_asset<T: Asset + 'static>(cache: &mut AssetCache, handle: &AssetHandle<T>) {
    cache.reload::<T>(handle)
}

/// Check if a specific asset is loaded
pub fn handle_available<T: Asset>(cache: &mut AssetCache, handle: &AssetHandle<T>) -> bool {
    cache.handle_available(handle)
}

/// Check if a specific asset is loaded
pub fn handle_just_available<T: Asset>(cache: &AssetCache, handle: &AssetHandle<T>) -> bool {
    cache.handle_just_available(handle)
}

/// Inserts a new asset
///
/// If possible the AssetHandle will be reused depeneding on key
pub fn insert_asset<T: Asset + 'static, I: AssetInserter + 'static>(
    cache: &mut AssetCache,
    key: impl Into<I::Key>,
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

/// Add a debug name to an asset handle
pub fn set_debug_name<T: Asset>(
    cache: &mut AssetCache,
    handle: &AssetHandle<T>,
    name: impl Into<String>,
) {
    cache.set_debug_name(handle, name);
}

/// Load a new asset using the settings
///
/// Uses cached handle if available
pub fn load_asset<T: AssetLoader + 'static>(
    cache: &mut AssetCache,
    settings: &T::Settings,
) -> AssetHandle<T::Asset> {
    cache.load_asset::<T>(settings)
}

pub fn get_asset<T: Asset + 'static>(
    cache: &mut AssetCache,
    handle: AssetHandle<T>,
) -> Result<&T, GetAssetState> {
    cache.get_asset(&handle)
}

pub fn debug_asset_dependency_graph(cache: &AssetCache) {
    cache.debug_asset_dependency_graph();
}
