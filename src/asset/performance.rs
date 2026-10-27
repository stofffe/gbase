use rustc_hash::FxHashMap;

use crate::asset::DynAssetHandle;

pub struct AssetCachePerformance {
    pending_times: FxHashMap<DynAssetHandle, web_time::Instant>,
    finished_times: FxHashMap<DynAssetHandle, web_time::Duration>,
}

impl AssetCachePerformance {
    pub fn new() -> Self {
        Self {
            pending_times: FxHashMap::default(),
            finished_times: FxHashMap::default(),
        }
    }

    pub fn start_load(&mut self, handle: DynAssetHandle) {
        let current_time = web_time::Instant::now();
        self.pending_times.insert(handle, current_time);
    }

    pub fn finish_load(&mut self, handle: DynAssetHandle) {
        let Some(start_time) = self.pending_times.remove(&handle) else {
            tracing::warn!("trying to finish load that hasnt started");
            return;
        };
        let elapsed_time = start_time.elapsed();
        self.finished_times.insert(handle, elapsed_time);
    }

    pub fn is_pending(&self, handle: &DynAssetHandle) -> bool {
        self.pending_times.contains_key(handle)
    }
    pub fn get_time(&self, handle: &DynAssetHandle) -> Option<web_time::Duration> {
        self.finished_times.get(handle).copied()
    }
}
