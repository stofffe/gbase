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

    pub fn add_dependencies(
        &mut self,
        handle: &DynAssetHandle,
        dependencies: &FxHashSet<DynAssetHandle>,
    ) {
        let handle_dependencies = self.dependencies.entry(handle.clone()).or_default();

        for dependency in dependencies {
            handle_dependencies.insert(dependency.clone());

            let dependency_dependents = self.dependents.entry(dependency.clone()).or_default();
            dependency_dependents.insert(handle.clone());
        }
    }

    pub fn get_dependencies(
        &mut self,
        handle: &DynAssetHandle,
    ) -> Option<FxHashSet<DynAssetHandle>> {
        self.dependencies.get(handle).cloned()
    }

    pub fn get_dependents(&mut self, handle: &DynAssetHandle) -> Option<FxHashSet<DynAssetHandle>> {
        self.dependents.get(handle).cloned()
    }
}
