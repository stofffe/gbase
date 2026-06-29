use crate::{parse_gltf_file, parse_gltf_primitives, Gltf, Material};
use gbase::{
    asset::{
        self, Asset, AssetCache, AssetConverter, AssetHandle, AssetLoader, AssetWriter,
        ConvertAssetStatus, DerivedAsset, EmptyError, LoadContext,
    },
    filesystem,
    render::{self, BoundingBox, VertexAttributeId},
    tracing,
};
use std::{collections::BTreeSet, ops::Deref};

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
        self.meshes.get(level).map(|e| e.0.clone())
    }
    pub fn get_lod_closest(&self, level: usize) -> asset::AssetHandle<render::Mesh> {
        let index = usize::min(level, self.meshes.len() - 1);
        self.meshes[index].0.clone()
    }
}

impl Asset for MeshLod {}

#[derive(Clone)]
pub struct MeshLodLoader {}

#[derive(Clone)]
pub struct MeshLodLoaderSettings {
    node_name: Option<String>,
    required_attributes: Option<BTreeSet<VertexAttributeId>>,
}

impl MeshLodLoaderSettings {
    pub fn new() -> Self {
        Self {
            node_name: None,
            required_attributes: None,
        }
    }

    pub fn with_node_name(mut self, value: impl Into<String>) -> Self {
        self.node_name = Some(value.into());
        self
    }

    pub fn with_required_attr(mut self, value: impl Into<BTreeSet<VertexAttributeId>>) -> Self {
        self.required_attributes = Some(value.into());
        self
    }
}

impl AssetLoader for MeshLodLoader {
    type Asset = MeshLod;
    type Settings = MeshLodLoaderSettings;
    type Error = filesystem::LoadFileError;

    async fn load(
        load_ctx: LoadContext,
        path: &std::path::Path,
        settings: Self::Settings,
    ) -> Result<Self::Asset, Self::Error> {
        let bytes = load_ctx.load_bytes(path).await?;
        let primitives =
            parse_gltf_primitives(&load_ctx, &bytes, settings.required_attributes.as_ref());

        // extract material from LOD0
        let material = primitives[0].material.clone(); // TODO: using material of LOD0 currently

        // extract lod levels
        let mut parsed_primitives = Vec::new();
        match &settings.node_name {
            Some(node_name) => {
                for prim in primitives.iter() {
                    if let Some(a) = prim.name.strip_prefix(node_name) {
                        if let Some(a) = a.strip_prefix("_LOD") {
                            let lod_level = a.parse::<usize>().expect("could not parse lod level");
                            parsed_primitives.push((lod_level, prim.mesh.clone()));
                        }
                    }
                }
                parsed_primitives.sort_by_key(|(lod_level, _)| *lod_level);
            }
            None => {
                parsed_primitives = primitives
                    .into_iter()
                    .enumerate()
                    .map(|(i, prim)| (i, prim.mesh))
                    .collect::<Vec<_>>();
            }
        }

        // create lod
        let meshes = parsed_primitives
            .into_iter()
            .enumerate()
            .map(|(i, (_, mesh))| (mesh, THRESHOLDS[i]))
            .collect::<Vec<_>>();

        Ok(MeshLod { meshes, material })
    }
}

impl AssetWriter for MeshLodLoader {
    fn write(asset: &Self::Asset, path: &std::path::Path) {
        tracing::info!("write {:?} lod to {:?}", asset, path);
    }
}

#[derive(Clone, Default)]
pub struct GltfLoader {}

#[derive(Clone)]
pub struct GltfLoaderSettings {
    required_attributes: Option<BTreeSet<VertexAttributeId>>,
}

impl GltfLoaderSettings {
    pub fn new() -> Self {
        Self {
            required_attributes: None,
        }
    }

    pub fn required_attributes(mut self, attributes: BTreeSet<VertexAttributeId>) -> Self {
        self.required_attributes = Some(attributes);
        self
    }
}

impl AssetLoader for GltfLoader {
    type Asset = Gltf;
    type Settings = GltfLoaderSettings;
    type Error = filesystem::LoadFileError;

    async fn load(
        load_ctx: LoadContext,
        path: &std::path::Path,
        settings: Self::Settings,
    ) -> Result<Self::Asset, Self::Error> {
        let bytes = load_ctx.load_bytes(path).await?;
        Ok(parse_gltf_file(
            &load_ctx,
            &bytes,
            settings.required_attributes.as_ref(),
        ))
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

impl DerivedAsset for BoundingBoxWrapper {}
pub struct LodMeshToBoundingBoxConverter;
impl AssetConverter for LodMeshToBoundingBoxConverter {
    type SourceAsset = MeshLod;
    type Error = EmptyError;
    type TargetAsset = BoundingBoxWrapper;

    fn convert(
        &self,
        _ctx: &mut gbase::Context,
        cache: &mut AssetCache,
        source: AssetHandle<Self::SourceAsset>,
    ) -> ConvertAssetStatus<Self::TargetAsset> {
        let source = cache.get(source.clone()).unwrap_loaded();
        let handle = source.meshes[0].0.clone();

        let bounding_box =
            BoundingBoxWrapper(handle.get(cache).unwrap_loaded().calculate_bounding_box());
        ConvertAssetStatus::Success(bounding_box)
    }
}
