use crate::asset::DynAssetHandle;
use rustc_hash::{FxHashMap, FxHashSet};

pub(crate) struct AssetCacheDependency {
    dependencies: FxHashMap<DynAssetHandle, FxHashSet<DynAssetHandle>>,
    dependents: FxHashMap<DynAssetHandle, FxHashSet<DynAssetHandle>>,
}

impl AssetCacheDependency {
    pub(crate) fn new() -> Self {
        Self {
            dependencies: FxHashMap::default(),
            dependents: FxHashMap::default(),
        }
    }

    pub(crate) fn dependencies(
        &self,
        handle: &DynAssetHandle,
    ) -> Option<&FxHashSet<DynAssetHandle>> {
        self.dependencies.get(handle)
    }

    pub(crate) fn dependents(&self, handle: &DynAssetHandle) -> Option<&FxHashSet<DynAssetHandle>> {
        self.dependents.get(handle)
    }

    pub(crate) fn register_dependencies(
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

    pub(crate) fn debug_graph(&self) {
        // normal
        tracing::info!("--- dependencies ---");
        for (handle, deps) in self.dependencies.iter() {
            tracing::info!("{}:", handle);
            for dep in deps.iter() {
                tracing::info!("  ->{}", dep);
            }
        }
        tracing::info!("--- ----------- ---");
    }
}
