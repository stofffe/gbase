use crate::asset::{DynAssetHandle, DynDependency};
use rustc_hash::{FxHashMap, FxHashSet};

pub struct AssetCacheDependency {
    dependencies: FxHashMap<DynDependency, FxHashSet<DynDependency>>,
    dependents: FxHashMap<DynDependency, FxHashSet<DynDependency>>,
}

impl AssetCacheDependency {
    pub fn new() -> Self {
        Self {
            dependencies: FxHashMap::default(),
            dependents: FxHashMap::default(),
        }
    }

    pub fn dependencies(&self, handle: &DynDependency) -> Option<&FxHashSet<DynDependency>> {
        self.dependencies.get(handle)
    }

    pub fn dependents(&self, handle: &DynDependency) -> Option<&FxHashSet<DynDependency>> {
        self.dependents.get(handle)
    }

    pub fn register_dependencies(
        &mut self,
        handle: &DynDependency,
        dependencies: &FxHashSet<DynDependency>,
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

    pub fn debug_graph(&self) {
        // normal
        tracing::info!("dependencies");
        for (handle, deps) in self.dependencies.iter() {
            tracing::info!("{}:", handle);
            for dep in deps.iter() {
                tracing::info!("->{}", dep);
            }
        }
    }
}
