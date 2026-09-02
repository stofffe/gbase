use gbase::{
    asset::{
        Asset, AssetConverter, AssetHandle, AssetLoader, ConvertAssetState, ConvertContext,
        EmptyError, GetAssetState, LoadContext,
    },
    filesystem::{self, LoadFileError},
    render::{self, ArcHandle, ArcShaderModule},
    tracing, Context,
};
use std::path::PathBuf;

//
// Shader string
//

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct ShaderStringLoaderSettings {
    path: PathBuf,
}
impl ShaderStringLoaderSettings {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }
}

pub struct ShaderString {
    source: String,
}
impl Asset for ShaderString {}

pub struct ShaderStringLoader;

impl AssetLoader for ShaderStringLoader {
    type Asset = ShaderString;

    type Settings = ShaderStringLoaderSettings;

    type Error = LoadFileError;

    async fn load(
        load_ctx: &mut LoadContext,
        settings: Self::Settings,
    ) -> Result<Self::Asset, Self::Error> {
        let mut source = String::new();

        let source_code = load_ctx.load_string(&settings.path).await?;

        for line in source_code.lines() {
            if let Some(rest) = line.trim().strip_prefix("import \"") {
                if let Some(import_relative_path) = rest.strip_suffix('"') {
                    let parent_folder = settings.path.parent().expect("could not get parent");
                    let full_path = parent_folder
                        .join(import_relative_path)
                        .with_extension("wgsl");
                    let normalized_full_path = filesystem::normalize_path(full_path);

                    let mut settings_with_new_path = settings.clone();
                    settings_with_new_path.path = normalized_full_path;

                    let import_source_handle = load_ctx
                        .request_load::<ShaderStringLoader>(settings_with_new_path)
                        .await;
                    let import_source = load_ctx.request_get(import_source_handle).await;

                    source.push_str(&import_source.source);

                    continue;
                }
            }

            source.push_str(line);
            source.push('\n');
        }

        tracing::info!("Loaded shader string\n{}", source);

        Ok(ShaderString { source })
    }
}

//
// Shader string gpu
//

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct ShaderStringGpuConverterSettings {
    shader_string_handle: AssetHandle<ShaderString>,
}

impl ShaderStringGpuConverterSettings {
    pub fn new(shader_string_handle: AssetHandle<ShaderString>) -> Self {
        Self {
            shader_string_handle,
        }
    }
}

pub struct ShaderStringGpuConverter {}
impl AssetConverter for ShaderStringGpuConverter {
    type Asset = ArcShaderModule;

    type Settings = ShaderStringGpuConverterSettings;

    type Error = EmptyError;

    fn convert(
        ctx: &mut Context,
        convert_ctx: &mut ConvertContext<'_>,
        settings: &Self::Settings,
    ) -> ConvertAssetState<Self::Asset> {
        let shader_string = match convert_ctx.get_asset(&settings.shader_string_handle) {
            Ok(shader_string) => shader_string,
            Err(state) => match state {
                GetAssetState::Loading => return ConvertAssetState::Loading,
                GetAssetState::Failed => return ConvertAssetState::Failed,
            },
        };

        #[cfg(not(target_arch = "wasm32"))]
        {
            let shader =
                render::ShaderBuilder::new().build_err_non_arc(ctx, shader_string.source.clone());

            match shader {
                Ok(shader) => ConvertAssetState::Success(ArcHandle::new(ctx, shader)),
                Err(err) => {
                    tracing::warn!("could not compile shader:\n{}", err);
                    ConvertAssetState::Failed
                }
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            let shader =
                render::ShaderBuilder::new().build_non_arc(ctx, shader_string.source.clone());
            ConvertAssetState::Success(ArcHandle::new(ctx, shader))
        }
    }
}
