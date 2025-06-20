use super::{Asset, ConvertableRenderAsset, LoadableAsset, RenderAsset};
use crate::{
    filesystem,
    render::{self, GpuImage},
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
        device: &wgpu::Device,
        _queue: &wgpu::Queue,
        _render_cache: &mut render::RenderCache,
        source: &Self::SourceAsset,
        _params: &Self::Params,
    ) -> Result<Self, Self::Error> {
        Ok(render::GpuMesh::new_inner(device, source))
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
        device: &wgpu::Device,
        _queue: &wgpu::Queue,
        _render_cache: &mut render::RenderCache,
        source: &Self::SourceAsset,
        _params: &Self::Params,
    ) -> Result<Self, Self::Error> {
        #[cfg(target_arch = "wasm32")]
        {
            Ok(source.build_inner_2(device))
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            source.build_inner_err_2(device)
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
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        render_cache: &mut render::RenderCache,
        source: &Self::SourceAsset,
        _params: &Self::Params,
    ) -> Result<Self, Self::Error> {
        let sampler = source.sampler.build_inner(render_cache, device);
        let texture = source.texture.build_inner(device, queue);
        let view = render::TextureViewBuilder::new(texture.clone()).build_inner(render_cache);
        Ok(GpuImage::new(texture, view, sampler))
    }
}
