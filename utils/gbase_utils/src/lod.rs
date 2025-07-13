use crate::{parse_gltf_primitives, Material};
use gbase::{
    asset::{
        self, Asset, AssetCache, AssetHandle, ConvertableRenderAsset, LoadableAsset, RenderAsset,
    },
    filesystem,
    render::{self, ArcHandle, BoundingBox, GpuMesh},
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
impl LoadableAsset for MeshLod {
    async fn load(load_ctx: asset::LoadContext, path: &std::path::Path) -> Self {
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

#[derive(Clone)]
pub struct MeshWrapper(ArcHandle<GpuMesh>);

impl RenderAsset for MeshWrapper {}
impl ConvertableRenderAsset for MeshWrapper {
    type SourceAsset = MeshLod;
    type Params = usize; // lod level
    type Error = bool;

    fn convert(
        ctx: &mut gbase::Context,
        cache: &mut AssetCache,
        source: AssetHandle<Self::SourceAsset>,
        params: &Self::Params,
    ) -> Result<Self, Self::Error> {
        let source = cache.get(source).unwrap();
        let mesh = source.get_lod_closest(*params);
        let gpu_mesh = mesh.convert::<GpuMesh>(ctx, cache, &()).unwrap();
        Ok(MeshWrapper(gpu_mesh))
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
    type Params = ();
    type Error = bool;

    fn convert(
        _ctx: &mut gbase::Context,
        cache: &mut AssetCache,
        source: AssetHandle<Self::SourceAsset>,
        _params: &Self::Params,
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

// impl Asset for MeshLod {}
// impl LoadableAsset for MeshLod {
//     async fn load(
//         path: &std::path::Path,
//         sender: futures_channel::mpsc::UnboundedSender<(asset::DynAssetHandle, asset::DynAsset)>,
//     ) -> Self {
//         let gltf = crate::parse_gltf_file(cache, bytes)
//             .get(cache)
//             .unwrap()
//             .clone();
//
//         // TODO: remove this
//         let thresholds = [0.25, 0.125, 0.0];
//         let mut meshes = Vec::new();
//         let mut material = None;
//         for (i, mesh) in gltf.meshes.iter().enumerate() {
//             let mesh = mesh.clone().get_mut(cache).unwrap().clone();
//             let primitive = &mesh.primitives[0];
//
//             let mesh_mut = primitive.mesh.clone().get_mut(cache).unwrap();
//             *mesh_mut = mesh_mut
//                 .clone()
//                 .extract_attributes(pbr.required_attributes().clone());
//
//             meshes.push((primitive.mesh.clone(), thresholds[i]));
//             material = Some(primitive.material.clone());
//         }
//
//         MeshLod {
//             meshes,
//             material: material.unwrap(),
//         }
//     }
// }
