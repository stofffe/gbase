use tracing::Instrument;

use super::{Asset, AssetHandle, AssetLoader};
use crate::{
    asset::{
        AssetConverter, AssetInserter, ConvertAssetStatus, ConvertContext, GetAssetResult,
        LoadContext,
    },
    filesystem,
    render::{
        self, ArcHandle, GpuImage, Image, Mesh, SamplerBuilder, Shader, ShaderBuilder,
        TextureBuilder,
    },
    Context,
};
use std::{fmt::Debug, hash::Hash, marker::PhantomData, path::PathBuf};

#[derive(thiserror::Error, Debug)]
pub enum EmptyError {}

//
// Named inserter
//

#[derive(Clone, Hash, Eq, PartialEq, Debug)]
pub struct NamedInserterKey {
    name: String, // TODO: arc or something instead?
}

impl NamedInserterKey {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }
}

pub struct NamedInserter;

impl AssetInserter for NamedInserter {
    type Key = NamedInserterKey;
}

//
// Named
//

pub struct ScopedNamedInserterKey<T: Asset> {
    parent: AssetHandle<T>,
    name: String, // TODO: arc or something instead?
}

impl<T: Asset> ScopedNamedInserterKey<T> {
    pub fn new(name: impl Into<String>, parent: AssetHandle<T>) -> Self {
        Self {
            name: name.into(),
            parent,
        }
    }
}

pub struct ScopedNamedInserter<T: Asset> {
    ty: PhantomData<T>,
}

impl<T: Asset> AssetInserter for ScopedNamedInserter<T> {
    type Key = ScopedNamedInserterKey<T>;
}

impl<T: Asset> Hash for ScopedNamedInserterKey<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.parent.hash(state);
        self.name.hash(state);
    }
}

impl<T: Asset> Clone for ScopedNamedInserterKey<T> {
    fn clone(&self) -> Self {
        Self {
            parent: self.parent.clone(),
            name: self.name.clone(),
        }
    }
}

impl<T: Asset> PartialEq for ScopedNamedInserterKey<T> {
    fn eq(&self, other: &Self) -> bool {
        self.parent == other.parent && self.name == other.name
    }
}
impl<T: Asset> Eq for ScopedNamedInserterKey<T> {}

impl<T: Asset> Debug for ScopedNamedInserterKey<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ScopedNamedInserterKey")
            .field("parent", &self.parent)
            .field("name", &self.name)
            .finish()
    }
}

//
// Id inserter
//

#[derive(Clone, Hash, Eq, PartialEq, Debug)]
pub struct IdInserterKey {
    id: u64,
}

pub struct IdInserter;

impl AssetInserter for IdInserter {
    type Key = IdInserterKey;
}

//
// Mesh
//

impl Asset for render::Mesh {}

#[derive(Clone, Hash, Eq, PartialEq, Debug)]
pub struct MeshGpuConverterSettings {
    mesh: AssetHandle<Mesh>,
}

impl MeshGpuConverterSettings {
    pub fn new(mesh: AssetHandle<Mesh>) -> Self {
        Self { mesh }
    }
}

pub struct MeshGpuConverter;
impl AssetConverter for MeshGpuConverter {
    type Asset = render::GpuMesh;
    type Error = EmptyError;
    type Settings = MeshGpuConverterSettings;

    fn convert(
        ctx: &mut Context,
        convert_ctx: &mut ConvertContext<'_>, // TODO: should this be mutable reference?
        settings: &Self::Settings,
    ) -> ConvertAssetStatus<Self::Asset> {
        let source = match convert_ctx.get_asset(&settings.mesh) {
            GetAssetResult::Loading => return ConvertAssetStatus::Loading,
            GetAssetResult::Error => return ConvertAssetStatus::Failed,
            GetAssetResult::Success(source) => source,
        };
        let gpu_mesh = render::GpuMesh::new(ctx, source);
        ConvertAssetStatus::Success(gpu_mesh)
    }
}

#[derive(Clone, Hash, PartialEq, Eq, Debug)]
pub struct BoundingBoxConverterOptions {
    mesh: AssetHandle<Mesh>,
}

pub struct BoundingBoxConverter;
impl AssetConverter for BoundingBoxConverter {
    type Asset = render::BoundingBox;
    type Error = EmptyError;
    type Settings = BoundingBoxConverterOptions;

    fn convert(
        _ctx: &mut Context,
        convert_ctx: &mut ConvertContext<'_>, // TODO: should this be mutable reference?
        settings: &Self::Settings,
    ) -> ConvertAssetStatus<Self::Asset> {
        let source = match convert_ctx.get_asset(&settings.mesh) {
            GetAssetResult::Loading => return ConvertAssetStatus::Loading,
            GetAssetResult::Error => return ConvertAssetStatus::Failed,
            GetAssetResult::Success(source) => source,
        };

        let bounding_box = source.calculate_bounding_box();
        ConvertAssetStatus::Success(bounding_box)
    }
}

//
// Shader
//

impl Asset for ArcHandle<wgpu::ShaderModule> {}

impl Asset for Shader {}

#[derive(Hash, PartialEq, Eq, Clone, Debug)]
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
        _load_ctx: &mut LoadContext,
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

pub struct ShaderGpuConverter;

#[derive(Clone, Hash, PartialEq, Eq, Debug)]
pub struct ShaderGpuConverterSettings {
    shader: AssetHandle<Shader>,
}
impl ShaderGpuConverterSettings {
    pub fn new(shader: AssetHandle<Shader>) -> Self {
        Self { shader }
    }
}
impl Asset for wgpu::ShaderModule {}

impl AssetConverter for ShaderGpuConverter {
    type Asset = ArcHandle<wgpu::ShaderModule>;
    type Error = wgpu::Error;
    type Settings = ShaderGpuConverterSettings;

    fn convert(
        ctx: &mut Context,
        convert_ctx: &mut ConvertContext<'_>, // TODO: should this be mutable reference?
        settings: &Self::Settings,
    ) -> ConvertAssetStatus<Self::Asset> {
        let source = match convert_ctx.get_asset(&settings.shader) {
            GetAssetResult::Loading => return ConvertAssetStatus::Loading,
            GetAssetResult::Error => return ConvertAssetStatus::Failed,
            GetAssetResult::Success(source) => source,
        };

        let shader_source = source.source.clone();

        #[cfg(target_arch = "wasm32")]
        {
            let shader_module = source.config.build_non_arc(ctx, shader_source);
            crate::asset::ConvertAssetStatus::Success(ArcHandle::new(ctx, shader_module))
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            match source.config.build_err_non_arc(ctx, shader_source) {
                Ok(shader_module) => {
                    ConvertAssetStatus::Success(ArcHandle::new(ctx, shader_module))
                }
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

#[derive(Hash, PartialEq, Eq, Clone, Default, Debug)]
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
        load_ctx: &mut LoadContext,
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

#[derive(Clone, Hash, PartialEq, Eq, Debug)]
pub struct ImageGpuConverterOptions {
    image: AssetHandle<Image>,
}

impl ImageGpuConverterOptions {
    pub fn new(image: AssetHandle<Image>) -> Self {
        Self { image }
    }
}

impl Asset for GpuImage {}

pub struct ImageGpuConverter;
impl AssetConverter for ImageGpuConverter {
    type Asset = render::GpuImage;
    type Error = EmptyError;
    type Settings = ImageGpuConverterOptions;

    fn convert(
        ctx: &mut Context,
        convert_ctx: &mut ConvertContext<'_>, // TODO: should this be mutable reference?
        settings: &Self::Settings,
    ) -> ConvertAssetStatus<Self::Asset> {
        let source = match convert_ctx.get_asset(&settings.image) {
            GetAssetResult::Loading => return ConvertAssetStatus::Loading,
            GetAssetResult::Error => return ConvertAssetStatus::Failed,
            GetAssetResult::Success(source) => source,
        };

        let sampler = source.sampler_config.clone().build(ctx);
        let texture = source.texture_config.build(ctx, source.source.clone());
        let view = render::TextureViewBuilder::new(texture.clone()).build(ctx);

        let gpu_image = GpuImage::new(texture, view, sampler);
        ConvertAssetStatus::Success(gpu_image)
    }
}
