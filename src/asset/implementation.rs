use super::{AssetHandle, AssetLoader};
use crate::{
    asset::{
        AssetConverter, AssetInserter, ConvertAssetState, ConvertContext, GetAssetState,
        LoadContext,
    },
    filesystem::{self, LoadFileError},
    render::{
        self, ArcHandle, ArcShaderModule, ArcTexture, Mesh, Shader, TextureBuilder, TextureSource,
    },
    Context,
};
use image::{ImageBuffer, Rgba, RgbaImage};
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

#[derive(Hash, PartialEq, Eq, Clone, Debug)]
pub enum ImageSource {
    // TODO: probably wanna remove
    Bytes(Vec<u8>),
    Path(PathBuf),
}

#[derive(Hash, PartialEq, Eq, Clone, Debug)]
pub struct ImageLoaderSettings {
    pub source: ImageSource,
}

pub struct ImageLoader {}

impl AssetLoader for ImageLoader {
    type Asset = RgbaImage;
    type Settings = ImageLoaderSettings;
    type Error = LoadFileError;

    async fn load(
        load_ctx: &mut LoadContext,
        settings: Self::Settings,
    ) -> Result<Self::Asset, Self::Error> {
        let bytes = match settings.source {
            ImageSource::Path(path) => load_ctx.load_bytes(&path).await?,
            ImageSource::Bytes(bytes) => bytes,
        };
        let image_buffer = image::load_from_memory(&bytes)
            .expect("could not load image")
            .to_rgba8();

        Ok(image_buffer)
    }
}

#[derive(Hash, PartialEq, Eq, Clone, Debug)]
pub enum ImageGpuSource {
    ImageSettings(ImageLoaderSettings),
    Handle(AssetHandle<ImageBuffer<Rgba<u8>, Vec<u8>>>),
}

#[derive(Hash, PartialEq, Eq, Clone, Debug)]
pub struct ImageGpuLoaderSettings {
    pub source: ImageGpuSource,
    pub texture_config: Option<TextureBuilder>,
}

impl ImageGpuLoaderSettings {
    pub fn new(source: ImageGpuSource, texture_config: Option<TextureBuilder>) -> Self {
        let mut settings = match source {
            ImageGpuSource::ImageSettings(settings) => Self::from_image_settings(settings),
            ImageGpuSource::Handle(handle) => Self::from_handle(handle),
        };
        settings.texture_config = texture_config;
        settings
    }

    pub fn from_image_settings(settings: ImageLoaderSettings) -> Self {
        Self {
            source: ImageGpuSource::ImageSettings(settings),
            texture_config: None,
        }
    }

    pub fn from_handle(handle: AssetHandle<ImageBuffer<Rgba<u8>, Vec<u8>>>) -> Self {
        Self {
            source: ImageGpuSource::Handle(handle),
            texture_config: None,
        }
    }

    pub fn from_path(path: impl Into<PathBuf>) -> Self {
        Self {
            source: ImageGpuSource::ImageSettings(ImageLoaderSettings {
                source: ImageSource::Path(path.into()),
            }),
            texture_config: None,
        }
    }
    pub fn with_config(mut self, config: TextureBuilder) -> Self {
        self.texture_config = Some(config);
        self
    }
}

pub struct ImageGpuLoader {}

impl AssetLoader for ImageGpuLoader {
    type Asset = ArcTexture;
    type Settings = ImageGpuLoaderSettings;
    type Error = filesystem::LoadFileError;

    async fn load(
        load_ctx: &mut LoadContext,
        settings: Self::Settings,
    ) -> Result<Self::Asset, Self::Error> {
        let image_handle = match settings.source {
            ImageGpuSource::Handle(asset_handle) => asset_handle,
            ImageGpuSource::ImageSettings(image_loader_settings) => {
                load_ctx
                    .request_load::<ImageLoader>(image_loader_settings)
                    .await
            }
        };

        let image = load_ctx.request_get(image_handle).await;

        let source = TextureSource::Data(image.width(), image.height(), image.clone().to_vec());
        let image = settings
            .texture_config
            .unwrap_or(TextureBuilder::new())
            .build(
                &load_ctx.render_runtime().device,
                &load_ctx.render_runtime().queue,
                source,
            );

        Ok(ArcHandle::new(load_ctx.arc_runtime().clone(), image))
    }
}
