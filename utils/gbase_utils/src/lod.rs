use crate::GpuMaterial;
use gbase::{asset, render};
use std::sync::Arc;

pub struct MeshLod {
    /// lod ordererd from highest quality to lowest
    meshes: Vec<(asset::AssetHandle<render::Mesh>, Arc<GpuMaterial>, f32)>,
}

impl MeshLod {
    pub fn new(meshes: Vec<(asset::AssetHandle<render::Mesh>, Arc<GpuMaterial>, f32)>) -> Self {
        Self { meshes }
    }
}
