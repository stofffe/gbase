use crate::{parse_gltf_file, Material};
use gbase::{
    asset::{self, Asset, AssetHandle, LoadableAsset},
    filesystem, render,
};

#[derive(Debug, Clone)]
pub struct MeshLod {
    /// lod ordererd from highest quality to lowest
    /// TODO: move threshold out of here?
    pub meshes: Vec<(AssetHandle<render::Mesh>, f32)>,
    pub material: AssetHandle<Material>,
}

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
        self.meshes.get(level).map(|e| e.0.clone())
    }
    pub fn get_lod_closest(&self, level: usize) -> asset::AssetHandle<render::Mesh> {
        let index = usize::min(level, self.meshes.len() - 1);
        self.meshes[index].0.clone()
    }
}

// impl Asset for MeshLod {}
// impl LoadableAsset for MeshLod {
//     async fn load(load_ctx: asset::LoadContext, path: &std::path::Path) -> Self {
//         let bytes = filesystem::load_bytes(path).await;
//         let gltf = parse_gltf_file(&load_ctx, &bytes);
//
//         let mut meshes = Vec::new();
//         let mut material = None;
//         for (i, mesh) in gltf.meshes.iter().enumerate() {
//             meshes.push(mesh.clone());
//
//             // mesh is asset handle, how do i access it?
//         }
//
//         MeshLod {
//             meshes,
//             material: material.unwrap(),
//         }
//     }
// }

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
