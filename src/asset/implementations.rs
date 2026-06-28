use std::env::temp_dir;

use super::{Asset, AssetCache, AssetHandle, AssetLoader};
use crate::{
    asset::{AssetConverter, ConvertAssetStatus, DerivedAsset, EmptyError, GetAssetResult},
    filesystem,
    render::{self, GpuImage, SamplerBuilder, Shader, ShaderBuilder, TextureBuilder},
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

impl Asset for Shader {}

#[derive(Clone)]
pub struct ShaderLoader {}
impl AssetLoader for ShaderLoader {
    type Asset = Shader;
    type Error = filesystem::LoadFileError;

    async fn load(
        &self,
        _load_ctx: super::LoadContext,
        path: &std::path::Path,
    ) -> Result<Self::Asset, Self::Error> {
        let source = _load_ctx.load_string(path).await?;
        let config = ShaderBuilder::new().label(
            path.to_str()
                .expect("could not convert path to string")
                .to_string(),
        );

        Ok(Self::Asset { source, config })
    }
}

impl DerivedAsset for wgpu::ShaderModule {}

pub struct ShaderGpuConverter;
impl AssetConverter for ShaderGpuConverter {
    type SourceAsset = render::Shader;
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
            match source.config.build_err_non_arc(ctx, source.source.clone()) {
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
//

impl Asset for render::Image {}

#[derive(Clone, Default)]
pub struct ImageLoader {
    pub texture_config: Option<TextureBuilder>,
    pub sampler_config: Option<SamplerBuilder>,
}

impl ImageLoader {
    pub fn new() -> Self {
        Self {
            texture_config: None,
            sampler_config: None,
        }
    }

    pub fn texture_config(mut self, texture_config: TextureBuilder) -> Self {
        self.texture_config = Some(texture_config);
        self
    }

    pub fn sampler_config(mut self, sampler_config: SamplerBuilder) -> Self {
        self.sampler_config = Some(sampler_config);
        self
    }
}

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
        let source = render::TextureSource::Data(img.width(), img.height(), img.to_vec());
        let texture_config = self.texture_config.clone().unwrap_or(TextureBuilder::new());
        let sampler_config = self.sampler_config.clone().unwrap_or(SamplerBuilder::new());

        Ok(Self::Asset {
            source,
            texture_config,
            sampler_config,
        })
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

        let sampler = source.sampler_config.clone().build(ctx);
        let texture = source.texture_config.build(ctx, source.source.clone());
        let view = render::TextureViewBuilder::new(texture.clone()).build(ctx);

        let gpu_image = GpuImage::new(texture, view, sampler);
        ConvertAssetStatus::Success(gpu_image)
    }
}
