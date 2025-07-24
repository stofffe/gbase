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
use std::{ops::Deref, primitive};

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

#[derive(Clone)]
pub struct MeshLodLoader {
    node_name: Option<String>,
}

impl MeshLodLoader {
    pub fn empty() -> Self {
        Self { node_name: None }
    }
    pub fn new(mesh_name: impl Into<String>) -> Self {
        Self {
            node_name: Some(mesh_name.into()),
        }
    }
}

impl AssetLoader for MeshLodLoader {
    type Asset = MeshLod;

    async fn load(&self, load_ctx: LoadContext, path: &std::path::Path) -> Self::Asset {
        let bytes = filesystem::load_bytes(path).await;
        let primitives = parse_gltf_primitives(&load_ctx, &bytes);

        // extract lod levels

        let mut parsed_primitives = Vec::new();

        match &self.node_name {
            Some(node_name) => {
                for prim in primitives.iter() {
                    dbg!(&prim.name);
                    if let Some(a) = prim.name.strip_prefix(node_name) {
                        if let Some(a) = a.strip_prefix("_LOD") {
                            let lod_level = a.parse::<usize>().expect("could not parse lod level");
                            parsed_primitives.push((lod_level, prim.mesh));
                        }
                    }
                }
                parsed_primitives.sort_by_key(|(lod_level, _)| *lod_level);
            }
            None => {
                parsed_primitives = primitives
                    .iter()
                    .enumerate()
                    .map(|(i, prim)| (i, prim.mesh))
                    .collect::<Vec<_>>();
            }
        }

        // for prim in primitives.iter() {
        //     if let Some(a) = prim.name.strip_prefix(&self.node_name) {
        //         if let Some(a) = a.strip_prefix("_LOD") {
        //             let lod_level = a.parse::<usize>().expect("could not parse lod level");
        //             parsed_primitives.push((lod_level, prim.mesh));
        //         }
        //     }
        // }

        // sort by lod level

        // create lod
        let material = primitives[0].material; // TODO: using material of LOD0 currently
        let meshes = parsed_primitives
            .iter()
            .enumerate()
            .map(|(i, (_, mesh))| (*mesh, THRESHOLDS[i]))
            .collect::<Vec<_>>();

        MeshLod { meshes, material }
    }
}

impl AssetWriter for MeshLodLoader {
    fn write(asset: &Self::Asset, path: &std::path::Path) {
        tracing::info!("write {:?} lod to {:?}", asset, path);
    }
}

#[derive(Clone)]
pub struct GltfLoader {}

impl AssetLoader for GltfLoader {
    type Asset = Gltf;

    async fn load(&self, load_ctx: LoadContext, path: &std::path::Path) -> Self::Asset {
        let bytes = filesystem::load_bytes(path).await;
        parse_gltf_file(&load_ctx, &bytes)
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
