use super::{Asset, ConvertableRenderAsset, LoadableAsset, RenderAsset};
use crate::{
    filesystem,
    render::{self, GpuImage},
    Context,
};

//
// Mesh
//

impl Asset for render::Mesh {}

impl RenderAsset for render::GpuMesh {}
impl ConvertableRenderAsset for render::GpuMesh {
    type SourceAsset = render::Mesh;
    type Params = ();
    type Error = bool;

    fn convert(
        ctx: &mut Context,
        source: &Self::SourceAsset,
        _params: &Self::Params,
    ) -> Result<Self, Self::Error> {
        Ok(render::GpuMesh::new(ctx, source))
    }
}

impl RenderAsset for render::BoundingBox {}
impl ConvertableRenderAsset for render::BoundingBox {
    type SourceAsset = render::Mesh;
    type Params = ();
    type Error = bool;

    fn convert(
        ctx: &mut Context,
        source: &Self::SourceAsset,
        params: &Self::Params,
    ) -> Result<Self, Self::Error> {
        Ok(source.calculate_bounding_box())
    }
}

//
// Shader
//

impl Asset for render::ShaderBuilder {}

impl LoadableAsset for render::ShaderBuilder {
    async fn load(path: &std::path::Path) -> Self {
        let source = filesystem::load_str(path).await;

        Self {
            label: Some(path.to_str().unwrap().to_string()),
            source,
        }
    }
}

impl RenderAsset for wgpu::ShaderModule {}
impl ConvertableRenderAsset for wgpu::ShaderModule {
    type SourceAsset = render::ShaderBuilder;
    type Params = ();
    type Error = wgpu::Error;

    fn convert(
        ctx: &mut Context,
        source: &Self::SourceAsset,
        _params: &Self::Params,
    ) -> Result<Self, Self::Error> {
        #[cfg(target_arch = "wasm32")]
        {
            Ok(source.build_non_arc(ctx))
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            source.build_err_non_arc(ctx)
        }
    }
}

//
// Image
//

impl Asset for render::Image {}

impl LoadableAsset for render::Image {
    async fn load(path: &std::path::Path) -> Self {
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
        Self { texture, sampler }
    }
}

impl RenderAsset for render::GpuImage {}

impl ConvertableRenderAsset for render::GpuImage {
    type SourceAsset = render::Image;
    type Params = ();
    type Error = bool;

    fn convert(
        ctx: &mut Context,
        source: &Self::SourceAsset,
        _params: &Self::Params,
    ) -> Result<Self, Self::Error> {
        let sampler = source.sampler.clone().build(ctx);
        let texture = source.texture.build(ctx);
        let view = render::TextureViewBuilder::new(texture.clone()).build(ctx);
        Ok(GpuImage::new(texture, view, sampler))
    }
}
