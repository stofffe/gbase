use crate::GpuMaterial;
use gbase::{asset, render, tracing};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct MeshLod {
    /// lod ordererd from highest quality to lowest
    /// TODO: move threshold out of here?
    pub meshes: Vec<(asset::AssetHandle<render::Mesh>, f32)>,
    pub mat: Arc<GpuMaterial>,
}

impl MeshLod {
    pub fn from_single_lod(mesh: asset::AssetHandle<render::Mesh>, mat: Arc<GpuMaterial>) -> Self {
        Self {
            meshes: vec![(mesh, 0.0)],
            mat,
        }
    }

    pub fn get_lod_exact(&self, level: usize) -> Option<asset::AssetHandle<render::Mesh>> {
        self.meshes.get(level).map(|e| e.0.clone())
    }
    pub fn get_lod_closest(&self, level: usize) -> asset::AssetHandle<render::Mesh> {
        let index = usize::min(level, self.meshes.len() - 1);
        self.meshes[index].0.clone()
    }
}
