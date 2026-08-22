use crate::asset::DynAssetHandle;
use rustc_hash::{FxHashMap, FxHashSet};

pub struct AssetCacheDependency {
    dependencies: FxHashMap<DynAssetHandle, FxHashSet<DynAssetHandle>>,
    dependents: FxHashMap<DynAssetHandle, FxHashSet<DynAssetHandle>>,
}

impl AssetCacheDependency {
    pub fn new() -> Self {
        Self {
            dependencies: FxHashMap::default(),
            dependents: FxHashMap::default(),
        }
    }

    pub fn dependencies_or_default(
        &mut self,
        handle: &DynAssetHandle,
    ) -> &mut FxHashSet<DynAssetHandle> {
        self.dependencies.entry(handle.clone()).or_default()
    }

    pub fn dependents_or_default(
        &mut self,
        handle: &DynAssetHandle,
    ) -> &mut FxHashSet<DynAssetHandle> {
        self.dependents.entry(handle.clone()).or_default()
    }

    pub fn dependencies(&self, handle: &DynAssetHandle) -> Option<&FxHashSet<DynAssetHandle>> {
        self.dependencies.get(handle)
    }

    pub fn dependents(&self, handle: &DynAssetHandle) -> Option<&FxHashSet<DynAssetHandle>> {
        self.dependents.get(handle)
    }

    pub fn register_dependencies(
        &mut self,
        handle: &DynAssetHandle,
        dependencies: &FxHashSet<DynAssetHandle>,
    ) {
        for dependency in dependencies {
            self.dependencies_or_default(handle)
                .insert(dependency.clone());
            self.dependents_or_default(dependency)
                .insert(handle.clone());
        }
    }
}
