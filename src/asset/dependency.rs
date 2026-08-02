#[cfg(not(target_arch = "wasm32"))]
use crate::asset::AssetCacheReload;
use crate::asset::{AssetCacheDerived, AssetCacheLoad, DynAssetHandle};
use rustc_hash::{FxHashMap, FxHashSet};

pub struct AssetCacheDependency {
    dependencies: FxHashMap<DynAssetHandle, FxHashSet<DynAssetHandle>>,
    dependents: FxHashMap<DynAssetHandle, FxHashSet<DynAssetHandle>>,

    currently_reloading: FxHashSet<DynAssetHandle>,
}

impl AssetCacheDependency {
    pub fn new() -> Self {
        Self {
            dependencies: FxHashMap::default(),
            dependents: FxHashMap::default(),

            currently_reloading: FxHashSet::default(),
        }
    }

    pub fn handle_asset_changed(
        &mut self,
        handle: &DynAssetHandle,
        derived: &mut AssetCacheDerived,
        loader: &mut AssetCacheLoad,
        #[cfg(not(target_arch = "wasm32"))] reloader: &mut AssetCacheReload,
    ) {
        tracing::error!("PROPAGATE {}", handle.id());

        derived.invalidate_derived_assets_depending_on_handle(handle.clone());

        #[cfg(not(target_arch = "wasm32"))]
        for dependent in self.dependents(handle).clone().iter() {
            reloader.reload(dependent.clone(), self, derived, loader);
        }
    }

    pub fn set_currently_reloading(&mut self, handle: DynAssetHandle) {
        self.currently_reloading.insert(handle);
    }
    pub fn is_currently_reloading(&mut self, handle: &DynAssetHandle) -> bool {
        self.currently_reloading.remove(handle)
    }

    pub fn dependencies(&mut self, handle: &DynAssetHandle) -> &mut FxHashSet<DynAssetHandle> {
        self.dependencies.entry(handle.clone()).or_default()
    }

    pub fn dependents(&mut self, handle: &DynAssetHandle) -> &mut FxHashSet<DynAssetHandle> {
        self.dependents.entry(handle.clone()).or_default()
    }

    pub fn add_dependencies(
        &mut self,
        handle: &DynAssetHandle,
        dependencies: &FxHashSet<DynAssetHandle>,
    ) {
        for dependency in dependencies {
            self.dependencies(handle).insert(dependency.clone());
            self.dependents(dependency).insert(handle.clone());
        }
    }
}
