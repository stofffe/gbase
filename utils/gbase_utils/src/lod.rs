use crate::{parse_gltf_file, parse_gltf_primitives, Gltf, Material};
use gbase::{
    asset::{
        self, Asset, AssetCache, AssetHandle, AssetLoader, AssetWriter, ConvertableRenderAsset,
        LoadContext, RenderAsset,
    },
    filesystem,
    render::{self, BoundingBox},
    tracing,
};
use std::ops::Deref;

#[derive(Debug, Clone)]
pub struct MeshLod {
    /// lod ordererd from highest quality to lowest
    /// TODO: move threshold out of here?
    pub meshes: Vec<(AssetHandle<render::Mesh>, f32)>,
    pub material: AssetHandle<Material>,
}

pub const THRESHOLDS: [f32; 3] = [0.25, 0.125, 0.0];

impl MeshLod {
    pub fn from_single_lod(
        mesh: AssetHandle<render::Mesh>,
        material: AssetHandle<Material>,
    ) -> Self {
        Self {
            meshes: vec![(mesh, 0.0)],
            material,
        }
    }

    pub fn get_lod_exact(&self, level: usize) -> Option<asset::AssetHandle<render::Mesh>> {
        self.meshes.get(level).map(|e| e.0)
    }
    pub fn get_lod_closest(&self, level: usize) -> asset::AssetHandle<render::Mesh> {
        let index = usize::min(level, self.meshes.len() - 1);
        self.meshes[index].0
    }
}

impl Asset for MeshLod {}

pub struct MeshLodLoader {}
impl AssetLoader for MeshLodLoader {
    type Asset = MeshLod;

    async fn load(load_ctx: LoadContext, path: &std::path::Path) -> Self::Asset {
        let bytes = filesystem::load_bytes(path).await;
        let primitives = parse_gltf_primitives(&load_ctx, &bytes);

        let material = primitives[0].material;
        let meshes = primitives
            .iter()
            .enumerate()
            .map(|(i, p)| (p.mesh, THRESHOLDS[i]))
            .collect();

        MeshLod { meshes, material }
    }
}
impl AssetWriter for MeshLodLoader {
    fn write(asset: &Self::Asset, path: &std::path::Path) {
        tracing::info!("write {:?} lod to {:?}", asset, path);
    }
}

pub struct GltfLoader {}

impl AssetLoader for GltfLoader {
    type Asset = Gltf;

    async fn load(load_ctx: LoadContext, path: &std::path::Path) -> Self::Asset {
        let bytes = filesystem::load_bytes(path).await;
        let gltf = parse_gltf_file(&load_ctx, &bytes);
        gltf
    }
}

#[derive(Clone)]
pub struct BoundingBoxWrapper(BoundingBox);

impl Deref for BoundingBoxWrapper {
    type Target = BoundingBox;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl RenderAsset for BoundingBoxWrapper {}
impl ConvertableRenderAsset for BoundingBoxWrapper {
    type SourceAsset = MeshLod;
    type Error = bool;

    fn convert(
        _ctx: &mut gbase::Context,
        cache: &mut AssetCache,
        source: AssetHandle<Self::SourceAsset>,
    ) -> Result<Self, Self::Error> {
        let source = cache.get(source).unwrap();
        Ok(BoundingBoxWrapper(
            source.meshes[0]
                .0
                .get(cache)
                .unwrap()
                .calculate_bounding_box(),
        ))
    }
}
