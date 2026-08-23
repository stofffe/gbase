use crate::asset::DynAssetHandle;
use rustc_hash::{FxHashMap, FxHashSet};

pub struct AssetCacheDependency {
    dependencies: FxHashMap<DynAssetHandle, FxHashSet<DynAssetHandle>>,
    dependents: FxHashMap<DynAssetHandle, FxHashSet<DynAssetHandle>>,

    waiting_dependencies: FxHashMap<DynAssetHandle, FxHashSet<DynAssetHandle>>,
    waiting_dependents: FxHashMap<DynAssetHandle, FxHashSet<DynAssetHandle>>,
}

impl AssetCacheDependency {
    pub fn new() -> Self {
        Self {
            dependencies: FxHashMap::default(),
            dependents: FxHashMap::default(),

            waiting_dependencies: FxHashMap::default(),
            waiting_dependents: FxHashMap::default(),
        }
    }

    pub fn dependencies(&self, handle: &DynAssetHandle) -> Option<&FxHashSet<DynAssetHandle>> {
        self.dependencies.get(handle)
    }

    pub fn dependents(&self, handle: &DynAssetHandle) -> Option<&FxHashSet<DynAssetHandle>> {
        self.dependents.get(handle)
    }

    pub fn waiting_dependencies(
        &self,
        handle: &DynAssetHandle,
    ) -> Option<&FxHashSet<DynAssetHandle>> {
        self.waiting_dependencies.get(handle)
    }

    pub fn waiting_dependents(
        &self,
        handle: &DynAssetHandle,
    ) -> Option<&FxHashSet<DynAssetHandle>> {
        self.waiting_dependents.get(handle)
    }

    pub fn register_dependencies(
        &mut self,
        handle: &DynAssetHandle,
        dependencies: &FxHashSet<DynAssetHandle>,
    ) {
        for dependency in dependencies {
            self.dependencies
                .entry(handle.clone())
                .or_default()
                .insert(dependency.clone());
            self.dependents
                .entry(dependency.clone())
                .or_default()
                .insert(handle.clone());
        }
    }

    pub fn register_waiting_dependencies(
        &mut self,
        handle: &DynAssetHandle,
        dependencies: &FxHashSet<DynAssetHandle>,
    ) {
        for dependency in dependencies {
            self.waiting_dependencies
                .entry(handle.clone())
                .or_default()
                .insert(dependency.clone());
            self.waiting_dependents
                .entry(dependency.clone())
                .or_default()
                .insert(handle.clone());
        }
    }
}
