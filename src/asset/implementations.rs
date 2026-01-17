use std::sync::Arc;

use super::{Asset, AssetCache, AssetHandle, AssetLoader, ConvertableRenderAsset, RenderAsset};
use crate::{
    asset::{ConvertRenderAssetResult, EmptyError, GetAssetResult},
    filesystem,
    render::{self, next_id, ArcHandle, GpuImage},
    Context,
};

//
// Mesh
//

impl Asset for render::Mesh {}

impl RenderAsset for render::GpuMesh {}
impl ConvertableRenderAsset for render::GpuMesh {
    type SourceAsset = render::Mesh;
    type Error = EmptyError;

    fn convert(
        ctx: &mut Context,
        cache: &mut AssetCache,
        source: AssetHandle<Self::SourceAsset>,
    ) -> ConvertRenderAssetResult<Self> {
        let source = match source.get(cache) {
            GetAssetResult::Loading => return ConvertRenderAssetResult::AssetLoading,
            GetAssetResult::Failed => return ConvertRenderAssetResult::Failed,
            GetAssetResult::Loaded(source) => source,
        };
        let handle = ArcHandle::new(next_id(ctx), render::GpuMesh::new(ctx, source));
        ConvertRenderAssetResult::Success(handle)
    }
}

impl RenderAsset for render::BoundingBox {}
impl ConvertableRenderAsset for render::BoundingBox {
    type SourceAsset = render::Mesh;
    type Error = EmptyError;

    fn convert(
        ctx: &mut Context,
        cache: &mut AssetCache,
        source: AssetHandle<Self::SourceAsset>,
    ) -> ConvertRenderAssetResult<Self> {
        let source = match source.get(cache) {
            GetAssetResult::Loading => return ConvertRenderAssetResult::AssetLoading,
            GetAssetResult::Failed => return ConvertRenderAssetResult::Failed,
            GetAssetResult::Loaded(source) => source,
        };

        let handle = ArcHandle::new(next_id(ctx), source.calculate_bounding_box());
        ConvertRenderAssetResult::Success(handle)
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
    type Error = EmptyError;

    async fn load(
        &self,
        _load_ctx: super::LoadContext,
        path: &std::path::Path,
    ) -> Result<Self::Asset, Self::Error> {
        let source = filesystem::load_str(path).await;

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

impl RenderAsset for wgpu::ShaderModule {}
impl ConvertableRenderAsset for wgpu::ShaderModule {
    type SourceAsset = render::ShaderBuilder;
    type Error = wgpu::Error;

    fn convert(
        ctx: &mut Context,
        cache: &mut AssetCache,
        source: AssetHandle<Self::SourceAsset>,
    ) -> ConvertRenderAssetResult<Self> {
        let source = match source.get(cache) {
            GetAssetResult::Loading => return ConvertRenderAssetResult::AssetLoading,
            GetAssetResult::Failed => return ConvertRenderAssetResult::Failed,
            GetAssetResult::Loaded(source) => source,
        };

        #[cfg(target_arch = "wasm32")]
        {
            Ok(source.build_non_arc(ctx))
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            match source.build_err_non_arc(ctx) {
                Ok(shader_module) => {
                    ConvertRenderAssetResult::Success(ArcHandle::new(next_id(ctx), shader_module))
                }
                Err(err) => {
                    tracing::error!("could not load shader module: {}", err);
                    ConvertRenderAssetResult::Failed
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
    type Error = EmptyError;

    async fn load(
        &self,
        _load_ctx: super::LoadContext,
        path: &std::path::Path,
    ) -> Result<Self::Asset, Self::Error> {
        let bytes = filesystem::load_bytes(path).await;

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

impl RenderAsset for render::GpuImage {}

impl ConvertableRenderAsset for render::GpuImage {
    type SourceAsset = render::Image;
    type Error = EmptyError;

    fn convert(
        ctx: &mut Context,
        cache: &mut AssetCache,
        source: AssetHandle<Self::SourceAsset>,
    ) -> ConvertRenderAssetResult<Self> {
        let source = match source.get(cache) {
            GetAssetResult::Loading => return ConvertRenderAssetResult::AssetLoading,
            GetAssetResult::Failed => return ConvertRenderAssetResult::Failed,
            GetAssetResult::Loaded(source) => source,
        };

        let sampler = source.sampler.clone().build(ctx);
        let texture = source.texture.build(ctx);
        let view = render::TextureViewBuilder::new(texture.clone()).build(ctx);

        let handle = ArcHandle::new(next_id(ctx), GpuImage::new(texture, view, sampler));
        ConvertRenderAssetResult::Success(handle)
    }
}
