use super::{Asset, AssetHandle, AssetLoader};
use crate::{
    asset::{
        AssetConverter, AssetInserter, ConvertAssetState, ConvertContext, GetAssetState,
        LoadContext,
    },
    filesystem,
    render::{
        self, ArcHandle, ArcShaderModule, GpuImage, Image, Mesh, SamplerBuilder, Shader,
        TextureBuilder,
    },
    Context,
};
use std::{fmt::Debug, hash::Hash, path::PathBuf};

//
// Error
//

#[derive(thiserror::Error, Debug)]
pub enum EmptyError {}

//
// Settings
//

#[derive(Hash, PartialEq, Eq, Clone, Debug)]
pub struct PathSettings {
    path: PathBuf,
}

impl PathSettings {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }
}

#[derive(Hash, PartialEq, Eq, Clone, Debug)]
pub enum PathOrStringSettings {
    Path(PathBuf),
    String(String),
}

#[derive(Hash, PartialEq, Eq, Clone, Debug)]
pub enum PathOrBytesSettings {
    Path(PathBuf),
    Bytes(Vec<u8>),
}

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

impl<T: Into<String>> From<T> for NamedInserterKey {
    fn from(value: T) -> Self {
        NamedInserterKey { name: value.into() }
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
    ) -> ConvertAssetState<Self::Asset> {
        let source = match convert_ctx.get_asset(&settings.mesh) {
            Ok(source) => source,
            Err(state) => match state {
                GetAssetState::Loading => return ConvertAssetState::Loading,
                GetAssetState::Failed => return ConvertAssetState::Failed,
            },
        };
        let gpu_mesh = render::GpuMesh::new(ctx, source);
        ConvertAssetState::Success(gpu_mesh)
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
    ) -> ConvertAssetState<Self::Asset> {
        let source = match convert_ctx.get_asset(&settings.mesh) {
            Ok(source) => source,
            Err(state) => match state {
                GetAssetState::Loading => return ConvertAssetState::Loading,
                GetAssetState::Failed => return ConvertAssetState::Failed,
            },
        };

        let bounding_box = source.calculate_bounding_box();
        ConvertAssetState::Success(bounding_box)
    }
}

//
// Shader
//

impl Asset for ArcHandle<wgpu::ShaderModule> {}

impl Asset for Shader {}

#[derive(Hash, PartialEq, Eq, Clone, Debug)]
pub enum ShaderSource {
    String(String),
    Path(PathBuf),
    Handle(AssetHandle<Shader>),
}

#[derive(Hash, PartialEq, Eq, Clone, Debug)]
pub struct ShaderGpuLoaderSettings {
    source: ShaderSource,
}

impl ShaderGpuLoaderSettings {
    pub fn new(source: PathOrStringSettings) -> Self {
        match source {
            PathOrStringSettings::Path(path) => Self::from_path(path),
            PathOrStringSettings::String(string) => Self::from_string(string),
        }
    }
    pub fn from_path(path: impl Into<PathBuf>) -> Self {
        Self {
            source: ShaderSource::Path(path.into()),
        }
    }
    pub fn from_string(string: impl Into<String>) -> Self {
        Self {
            source: ShaderSource::String(string.into()),
        }
    }
    pub fn from_handle(handle: AssetHandle<Shader>) -> Self {
        Self {
            source: ShaderSource::Handle(handle),
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum LoadShaderError {
    #[error("could not load file")]
    LoadFileError(#[from] filesystem::LoadFileError),
    #[error("could not compile shader")]
    CompileShaderError(#[from] wgpu::Error),
}

pub struct ShaderGpuLoader {}
impl AssetLoader for ShaderGpuLoader {
    type Asset = ArcShaderModule;
    type Settings = ShaderGpuLoaderSettings;
    type Error = LoadShaderError;

    async fn load(
        load_ctx: &mut LoadContext,
        settings: Self::Settings,
    ) -> Result<Self::Asset, Self::Error> {
        let source = match settings.source {
            ShaderSource::Path(path_buf) => load_ctx.load_string(&path_buf).await?,
            ShaderSource::String(source) => source,
            ShaderSource::Handle(handle) => load_ctx.request_get(handle).await.source.clone(),
        };

        let shader = render::ShaderBuilder::new()
            .build_err(&load_ctx.render_runtime().device, source)
            .await;

        let arc_runtime = load_ctx.arc_runtime().clone();
        match shader {
            Ok(shader) => Ok(ArcHandle::new(arc_runtime, shader)),
            Err(err) => {
                tracing::warn!("could not compile shader:\n{}", err);
                Err(LoadShaderError::CompileShaderError(err))
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

#[derive(Hash, PartialEq, Eq, Clone, Debug)]
pub struct ImageLoaderSettings {
    pub source: PathOrBytesSettings,
    pub texture_config: Option<TextureBuilder>,
    pub sampler_config: Option<SamplerBuilder>,
}

impl ImageLoaderSettings {
    pub fn new(source: PathOrBytesSettings) -> Self {
        match source {
            PathOrBytesSettings::Path(path) => Self::from_path(path),
            PathOrBytesSettings::Bytes(bytes) => Self::from_bytes(bytes),
        }
    }

    pub fn from_path(path: impl Into<PathBuf>) -> Self {
        Self {
            source: PathOrBytesSettings::Path(path.into()),
            texture_config: None,
            sampler_config: None,
        }
    }

    pub fn from_bytes(bytes: impl Into<Vec<u8>>) -> Self {
        Self {
            source: PathOrBytesSettings::Bytes(bytes.into()),
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
        let bytes = match settings.source {
            PathOrBytesSettings::Path(path) => load_ctx.load_bytes(&path).await?,
            PathOrBytesSettings::Bytes(bytes) => bytes,
        };

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
    ) -> ConvertAssetState<Self::Asset> {
        let source = match convert_ctx.get_asset(&settings.image) {
            Ok(source) => source,
            Err(state) => match state {
                GetAssetState::Loading => return ConvertAssetState::Loading,
                GetAssetState::Failed => return ConvertAssetState::Failed,
            },
        };

        let sampler = source.sampler_config.clone().build(ctx);
        let texture = source.texture_config.build(ctx, source.source.clone());
        let view = render::TextureViewBuilder::new(texture.clone()).build(ctx);

        let gpu_image = GpuImage::new(texture, view, sampler);
        ConvertAssetState::Success(gpu_image)
    }
}
