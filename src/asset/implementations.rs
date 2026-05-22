use super::{Asset, AssetCache, AssetHandle, AssetLoader};
use crate::{
    asset::{AssetConverter, ConvertAssetStatus, DerivedAsset, EmptyError, GetAssetResult},
    filesystem,
    render::{self, GpuImage},
    Context,
};

//
// Mesh
//

impl Asset for render::Mesh {}

impl DerivedAsset for render::GpuMesh {}

pub struct MeshGpuConverter;
impl AssetConverter for MeshGpuConverter {
    type SourceAsset = render::Mesh;
    type Error = EmptyError;
    type TargetAsset = render::GpuMesh;

    fn convert(
        &self,
        ctx: &mut Context,
        cache: &mut AssetCache,
        source: AssetHandle<Self::SourceAsset>, // TODO: make this refernce?
    ) -> ConvertAssetStatus<Self::TargetAsset> {
        let source = match source.get(cache) {
            GetAssetResult::Loading => return ConvertAssetStatus::SourceLoading,
            GetAssetResult::Failed => return ConvertAssetStatus::Failed,
            GetAssetResult::Success(source) => source,
        };
        let gpu_mesh = render::GpuMesh::new(ctx, source);
        ConvertAssetStatus::Success(gpu_mesh)
    }
}

impl DerivedAsset for render::BoundingBox {}

pub struct BoundingBoxConverter {}
impl AssetConverter for BoundingBoxConverter {
    type SourceAsset = render::Mesh;
    type TargetAsset = render::BoundingBox;
    type Error = EmptyError;

    fn convert(
        &self,
        _ctx: &mut Context,
        cache: &mut AssetCache,
        source: AssetHandle<Self::SourceAsset>, // TODO: make this refernce?
    ) -> ConvertAssetStatus<Self::TargetAsset> {
        let source = match source.get(cache) {
            GetAssetResult::Loading => return ConvertAssetStatus::SourceLoading,
            GetAssetResult::Failed => return ConvertAssetStatus::Failed,
            GetAssetResult::Success(source) => source,
        };

        let bounding_box = source.calculate_bounding_box();
        ConvertAssetStatus::Success(bounding_box)
    }
}

//
// Shader
//

impl Asset for render::ShaderBuilder {}

#[derive(Clone)]
pub struct ShaderLoader {}
impl AssetLoader for ShaderLoader {
    type Asset = render::ShaderBuilder;
    type Error = filesystem::LoadFileError;

    async fn load(
        &self,
        _load_ctx: super::LoadContext,
        path: &std::path::Path,
    ) -> Result<Self::Asset, Self::Error> {
        let source = _load_ctx.load_string(path).await?;

        Ok(Self::Asset {
            label: Some(
                path.to_str()
                    .expect("could not convert path to string")
                    .to_string(),
            ),
            source,
        })
    }
}

impl DerivedAsset for wgpu::ShaderModule {}

pub struct ShaderGpuConverter;
impl AssetConverter for ShaderGpuConverter {
    type SourceAsset = render::ShaderBuilder;
    type TargetAsset = wgpu::ShaderModule;
    type Error = wgpu::Error;

    fn convert(
        &self,
        ctx: &mut Context,
        cache: &mut AssetCache,
        source: AssetHandle<Self::SourceAsset>, // TODO: make this refernce?
    ) -> ConvertAssetStatus<Self::TargetAsset> {
        let source = match source.get(cache) {
            GetAssetResult::Loading => return ConvertAssetStatus::SourceLoading,
            GetAssetResult::Failed => return ConvertAssetStatus::Failed,
            GetAssetResult::Success(source) => source,
        };

        #[cfg(target_arch = "wasm32")]
        {
            let shader_module = source.build_non_arc(ctx);
            crate::asset::ConvertAssetStatus::Success(shader_module)
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            match source.build_err_non_arc(ctx) {
                Ok(shader_module) => ConvertAssetStatus::Success(shader_module),
                Err(err) => {
                    tracing::error!("could not load shader module: {}", err);
                    ConvertAssetStatus::Failed
                }
            }
        }
    }
}

//
// Image
//

impl Asset for render::Image {}

#[derive(Clone)]
pub struct ImageLoader {}
impl AssetLoader for ImageLoader {
    type Asset = render::Image;
    type Error = filesystem::LoadFileError;

    async fn load(
        &self,
        load_ctx: super::LoadContext,
        path: &std::path::Path,
    ) -> Result<Self::Asset, Self::Error> {
        let bytes = load_ctx.load_bytes(path).await?;

        let img = image::load_from_memory(&bytes)
            .expect("could not load image")
            .to_rgba8();
        let texture = render::TextureBuilder::new(render::TextureSource::Data(
            img.width(),
            img.height(),
            img.to_vec(),
        ));
        let sampler = render::SamplerBuilder::new();
        Ok(Self::Asset { texture, sampler })
    }
}

impl DerivedAsset for render::GpuImage {}

pub struct ImageGpuConverter;
impl AssetConverter for ImageGpuConverter {
    type SourceAsset = render::Image;
    type TargetAsset = render::GpuImage;
    type Error = EmptyError;

    fn convert(
        &self,
        ctx: &mut Context,
        cache: &mut AssetCache,
        source: AssetHandle<Self::SourceAsset>, // TODO: make this refernce?
    ) -> ConvertAssetStatus<Self::TargetAsset> {
        let source = match source.get(cache) {
            GetAssetResult::Loading => return ConvertAssetStatus::SourceLoading,
            GetAssetResult::Failed => return ConvertAssetStatus::Failed,
            GetAssetResult::Success(source) => source,
        };

        let sampler = source.sampler.clone().build(ctx);
        let texture = source.texture.build(ctx);
        let view = render::TextureViewBuilder::new(texture.clone()).build(ctx);

        let gpu_image = GpuImage::new(texture, view, sampler);
        ConvertAssetStatus::Success(gpu_image)
    }
}
