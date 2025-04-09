use crate::{GpuMesh, Mesh};
use gbase::{log, Context};
use std::{
    collections::HashMap,
    sync::{atomic::AtomicU64, Arc},
};

static NEXT_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Hash, PartialEq, PartialOrd, Ord, Eq, Clone)]
pub struct AssetHandle {
    id: u64,
}

pub struct Assets {
    meshes: HashMap<AssetHandle, Mesh>,
    gpu_meshes: HashMap<AssetHandle, (Arc<GpuMesh>, bool)>,
}

impl Assets {
    pub fn new() -> Self {
        Self {
            meshes: HashMap::new(),
            gpu_meshes: HashMap::new(),
        }
    }
    pub fn allocate(&mut self) -> AssetHandle {
        let handle = AssetHandle {
            id: NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst),
        };
        self.meshes.insert(handle.clone(), Mesh::default());
        handle
    }
    pub fn allocate_data(&mut self, mesh: Mesh) -> AssetHandle {
        let handle = AssetHandle {
            id: NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst),
        };
        self.meshes.insert(handle.clone(), mesh);
        handle
    }

    pub fn get(&self, id: AssetHandle) -> &Mesh {
        self.meshes.get(&id).unwrap()
    }
    pub fn get_mut(&mut self, id: AssetHandle) -> &mut Mesh {
        let (_, gpu_changed) = self.gpu_meshes.get_mut(&id).unwrap();
        *gpu_changed = true;

        self.meshes.get_mut(&id).unwrap()
    }

    pub fn get_gpu(&mut self, ctx: &Context, id: AssetHandle) -> Arc<GpuMesh> {
        debug_assert!(self.meshes.contains_key(&id), "handle doesnt exist");

        // if handle doesnt exist => create new
        match self.gpu_meshes.get_mut(&id) {
            // create buffer
            None => {
                let mesh = self.meshes.get(&id).expect("handle doesnt exist");

                let gpu_mesh = GpuMesh::new(ctx, mesh);
                self.gpu_meshes.insert(id.clone(), (gpu_mesh.into(), false));
                log::info!("create mesh gpu buffer");
            }
            Some((gpu_mesh, changed)) => {
                if *changed {
                    log::info!("Update mesh gpu buffer");
                    let mesh = self.meshes.get(&id).expect("handle doesnt exist");
                    *gpu_mesh = GpuMesh::new(ctx, mesh).into();
                    *changed = false;
                }
            }
        }

        self.gpu_meshes.get(&id).expect("should exst").0.clone()
    }
}
