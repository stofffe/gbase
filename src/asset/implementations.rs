use std::path::PathBuf;

use super::{Asset, AssetCache, AssetHandle, AssetLoader};
use crate::{
    asset::{
        derive, AssetConverter, ConvertAssetStatus, ConvertContext, DerivedAsset, EmptyError,
        GetAssetResult, NoSettings,
    },
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
    type TargetAsset = render::GpuMesh;
    type Error = EmptyError;
    type Settings = NoSettings;

    fn convert(
        ctx: &mut Context,
        convert_ctx: &mut ConvertContext<'_>,
        source: AssetHandle<Self::SourceAsset>, // TODO: make this refernce?
        _settings: &Self::Settings,
    ) -> ConvertAssetStatus<Self::TargetAsset> {
        let source = match convert_ctx.get(source) {
            GetAssetResult::Loading => return ConvertAssetStatus::SourceLoading,
            GetAssetResult::Failed => return ConvertAssetStatus::Failed,
            GetAssetResult::Success(source) => source,
        };
        let gpu_mesh = render::GpuMesh::new(ctx, source);
        ConvertAssetStatus::Success(gpu_mesh)
    }
}

impl DerivedAsset for render::BoundingBox {}

pub struct BoundingBoxConverter;
impl AssetConverter for BoundingBoxConverter {
    type SourceAsset = render::Mesh;
    type TargetAsset = render::BoundingBox;
    type Error = EmptyError;
    type Settings = NoSettings;

    fn convert(
        _ctx: &mut Context,
        convert_ctx: &mut ConvertContext<'_>,
        source: AssetHandle<Self::SourceAsset>, // TODO: make this refernce?
        _settings: &Self::Settings,
    ) -> ConvertAssetStatus<Self::TargetAsset> {
        let source = match convert_ctx.get(source) {
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
pub struct ShaderLoaderSettings {
    path: PathBuf,
}

impl ShaderLoaderSettings {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }
}

#[derive(Clone)]
pub struct ShaderLoader {}
impl AssetLoader for ShaderLoader {
    type Asset = Shader;
    type Settings = ShaderLoaderSettings;
    type Error = filesystem::LoadFileError;

    async fn load(
        _load_ctx: super::LoadContext,
        settings: Self::Settings,
    ) -> Result<Self::Asset, Self::Error> {
        let source = _load_ctx.load_string(&settings.path).await?;
        let config = ShaderBuilder::new().label(
            settings
                .path
                .to_str()
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
    type Settings = NoSettings;

    fn convert(
        ctx: &mut Context,
        convert_ctx: &mut ConvertContext<'_>,
        source: AssetHandle<Self::SourceAsset>, // TODO: make this refernce?
        _settings: &Self::Settings,
    ) -> ConvertAssetStatus<Self::TargetAsset> {
        let source = match convert_ctx.get(source) {
            GetAssetResult::Loading => return ConvertAssetStatus::SourceLoading,
            GetAssetResult::Failed => return ConvertAssetStatus::Failed,
            GetAssetResult::Success(source) => source,
        };

        let shader_source = source.source.clone();

        #[cfg(target_arch = "wasm32")]
        {
            let shader_module = source.config.build_non_arc(ctx, shader_source);
            crate::asset::ConvertAssetStatus::Success(shader_module)
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            match source.config.build_err_non_arc(ctx, shader_source) {
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

pub struct ImageLoader {}

#[derive(Clone, Default)]
pub struct ImageLoaderSettings {
    pub path: PathBuf,
    pub texture_config: Option<TextureBuilder>,
    pub sampler_config: Option<SamplerBuilder>,
}

impl ImageLoaderSettings {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
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
    type Settings = ImageLoaderSettings;
    type Error = filesystem::LoadFileError;

    async fn load(
        load_ctx: super::LoadContext,
        settings: Self::Settings,
    ) -> Result<Self::Asset, Self::Error> {
        let bytes = load_ctx.load_bytes(&settings.path).await?;

        let img = image::load_from_memory(&bytes)
            .expect("could not load image")
            .to_rgba8();
        let source = render::TextureSource::Data(img.width(), img.height(), img.to_vec());
        let texture_config = settings
            .texture_config
            .clone()
            .unwrap_or(TextureBuilder::new());
        let sampler_config = settings
            .sampler_config
            .clone()
            .unwrap_or(SamplerBuilder::new());

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
    type Settings = NoSettings;

    fn convert(
        ctx: &mut Context,
        convert_ctx: &mut ConvertContext<'_>,
        source: AssetHandle<Self::SourceAsset>, // TODO: make this refernce?
        _settings: &Self::Settings,
    ) -> ConvertAssetStatus<Self::TargetAsset> {
        let source = match convert_ctx.get(source) {
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
