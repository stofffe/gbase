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

    pub(crate) fn set_dependencies(
        &mut self,
        handle: &DynAssetHandle,
        dependencies: &FxHashSet<DynAssetHandle>,
    ) {
        // remove old dependencies
        if let Some(dependencies) = self.dependencies.remove(handle) {
            for dependency in dependencies {
                if let Some(dependents) = self.dependents.get_mut(&dependency) {
                    // remove handle from dependent
                    dependents.remove(handle);
                    if dependents.is_empty() {
                        self.dependents.remove(&dependency);
                    }
                }
            }
        }

        // set new dependencies
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
