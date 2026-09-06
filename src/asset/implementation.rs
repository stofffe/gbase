use super::{AssetHandle, AssetLoader};
use crate::{
    asset::{AssetInserter, LoadContext},
    filesystem::{self, LoadFileError},
    render::{
        self, ArcHandle, ArcShaderModule, ArcTexture, GpuMesh, Mesh, Shader, TextureBuilder,
        TextureSource,
    },
};
use image::RgbaImage;
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
pub enum BytesOrPathSettings {
    Bytes(Vec<u8>),
    Path(PathBuf),
}

#[derive(Hash, PartialEq, Eq, Clone, Debug)]
pub enum StringOrPathSettings {
    Path(PathBuf),
    String(String),
}

#[derive(Hash, PartialEq, Eq, Clone, Debug)]
pub struct PathSettings {
    path: PathBuf,
}

impl PathSettings {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }
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
pub struct MeshGpuLoaderSettings {
    mesh: AssetHandle<Mesh>,
}

impl MeshGpuLoaderSettings {
    pub fn new(handle: AssetHandle<Mesh>) -> Self {
        Self { mesh: handle }
    }
}

pub struct MeshGpuLoader;

impl AssetLoader for MeshGpuLoader {
    type Asset = GpuMesh;
    type Settings = MeshGpuLoaderSettings;
    type Error = EmptyError;

    async fn load(
        load_ctx: &mut LoadContext,
        settings: Self::Settings,
    ) -> Result<Self::Asset, Self::Error> {
        let mesh = load_ctx.request_get(settings.mesh).await;

        let gpu_mesh =
            render::GpuMesh::new(load_ctx.render_runtime(), load_ctx.arc_runtime(), &mesh);

        Ok(gpu_mesh)
    }
}

//
// Shader
//

#[derive(Clone, Hash, Eq, PartialEq, Debug)]
pub struct ShaderLoaderSettings {
    source: StringOrPathSettings,
}

impl ShaderLoaderSettings {
    pub fn new(source: StringOrPathSettings) -> Self {
        Self { source }
    }

    pub fn from_path(path: impl Into<PathBuf>) -> Self {
        Self {
            source: StringOrPathSettings::Path(path.into()),
        }
    }

    pub fn from_string(string: impl Into<String>) -> Self {
        Self {
            source: StringOrPathSettings::String(string.into()),
        }
    }
}

pub struct ShaderLoader;

impl AssetLoader for ShaderLoader {
    type Asset = Shader;
    type Settings = ShaderLoaderSettings;
    type Error = LoadFileError;

    async fn load(
        load_ctx: &mut LoadContext,
        settings: Self::Settings,
    ) -> Result<Self::Asset, Self::Error> {
        let source = match settings.source {
            StringOrPathSettings::Path(path_buf) => load_ctx.load_string(path_buf).await?,
            StringOrPathSettings::String(string) => string,
        };

        Ok(Shader::new(source))
    }
}

#[derive(Hash, PartialEq, Eq, Clone, Debug)]
pub struct ShaderGpuLoaderSettings {
    source: AssetHandle<Shader>,
}

impl ShaderGpuLoaderSettings {
    pub fn new(source: AssetHandle<Shader>) -> Self {
        Self { source }
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
        let source = load_ctx.request_get(settings.source).await;

        let shader = render::ShaderBuilder::new()
            .build_err(&load_ctx.render_runtime().device, source.source.clone())
            .await;

        let arc_runtime = load_ctx.arc_runtime().clone();
        match shader {
            Ok(shader) => Ok(ArcHandle::new(load_ctx.arc_runtime(), shader)),
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
pub struct ImageLoaderSettings {
    pub source: BytesOrPathSettings,
}

impl ImageLoaderSettings {
    pub fn new(source: BytesOrPathSettings) -> Self {
        Self { source }
    }

    pub fn from_path(path: impl Into<PathBuf>) -> Self {
        Self {
            source: BytesOrPathSettings::Path(path.into()),
        }
    }

    pub fn from_bytes(bytes: impl Into<Vec<u8>>) -> Self {
        Self {
            source: BytesOrPathSettings::Bytes(bytes.into()),
        }
    }
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
            BytesOrPathSettings::Path(path) => load_ctx.load_bytes(&path).await?,
            BytesOrPathSettings::Bytes(bytes) => bytes,
        };
        let image_buffer = image::load_from_memory(&bytes)
            .expect("could not load image")
            .to_rgba8();

        Ok(image_buffer)
    }
}

#[derive(Hash, PartialEq, Eq, Clone, Debug)]
pub struct ImageGpuLoaderSettings {
    pub source: AssetHandle<RgbaImage>,
    pub texture_config: Option<TextureBuilder>,
}

impl ImageGpuLoaderSettings {
    pub fn new(source: AssetHandle<RgbaImage>) -> Self {
        Self {
            source,
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
        let image = load_ctx.request_get(settings.source).await;

        let source = TextureSource::Data(image.width(), image.height(), image.clone().to_vec());
        let image = settings
            .texture_config
            .unwrap_or(TextureBuilder::new())
            .build(
                &load_ctx.render_runtime().device,
                &load_ctx.render_runtime().queue,
                source,
            );

        Ok(ArcHandle::new(load_ctx.arc_runtime(), image))
    }
}
