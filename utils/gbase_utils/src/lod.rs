use gbase::render;

pub struct MeshLod {
    /// lod ordererd from highest quality to lowest
    meshes: Vec<render::Mesh>,
}

impl MeshLod {
    pub fn new(meshes: Vec<render::Mesh>) -> Self {
        Self { meshes }
    }
}
