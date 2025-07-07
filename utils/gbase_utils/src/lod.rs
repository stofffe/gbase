use std::sync::Arc;

use gbase::{
    asset,
    render::{self, BoundingBox},
    Context,
};

use crate::{BoundingSphere, GpuMaterial};

#[derive(Debug, Clone)]
pub struct MeshLod {
    /// lod ordererd from highest quality to lowest
    pub meshes: Vec<(asset::AssetHandle<render::Mesh>, f32)>,
    pub mat: Arc<GpuMaterial>,
}
