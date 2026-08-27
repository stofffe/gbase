use gbase::render::ArcHandle;
use gbase::{
    asset::{
        self, Asset, AssetConverter, AssetHandle, ConvertAssetStatus, ConvertContext, EmptyError,
        GetAssetResult, LoadContext,
    },
    filesystem,
    render::{self, ArcShaderModule},
    tracing, Context,
};
use std::path::PathBuf;

//
// Asset loading
//

#[derive(Debug, Clone)]
pub struct ShaderWithImports {
    source: String,
    imports: Vec<AssetHandle<ShaderWithImports>>,
}

impl Asset for ShaderWithImports {}

#[derive(Hash, PartialEq, Eq, Clone)]
pub struct ShaderWithImportsLoaderSettings {
    path: PathBuf,
}

impl ShaderWithImportsLoaderSettings {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }
}

#[derive(Debug, Clone)]
pub struct ShaderWithImportsLoader {}

impl asset::AssetLoader for ShaderWithImportsLoader {
    type Asset = ShaderWithImports;
    type Settings = ShaderWithImportsLoaderSettings;
    type Error = filesystem::LoadFileError;

    async fn load(
        load_ctx: &mut LoadContext,
        settings: Self::Settings,
    ) -> Result<Self::Asset, Self::Error> {
        let mut source = String::new();
        let mut imports = Vec::new();

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
                    let import = load_ctx
                        .request_load::<ShaderWithImportsLoader>(settings_with_new_path)
                        .await;

                    imports.push(import);

                    continue;
                }
            }

            source.push_str(line);
            source.push('\n');
        }

        // tracing::info!("Loaded {} {:?}", source, imports);

        Ok(ShaderWithImports { source, imports })
    }
}

#[derive(Clone)]
pub struct ShaderWithImportsFinal {
    source: String,
}

//
// Shader conversion
//

impl Asset for ShaderWithImportsFinal {}

#[derive(Clone, Hash, PartialEq, Eq)]
pub struct ShaderWithImportsConverterOptions {
    shader: AssetHandle<ShaderWithImports>,
}
impl ShaderWithImportsConverterOptions {
    pub fn new(shader: AssetHandle<ShaderWithImports>) -> Self {
        Self { shader }
    }
}

pub struct ShaderWithImportsConverter {}

impl AssetConverter for ShaderWithImportsConverter {
    type Asset = ShaderWithImportsFinal;
    type Settings = ShaderWithImportsConverterOptions;
    type Error = EmptyError;

    fn convert(
        _ctx: &mut Context,
        convert_ctx: &mut ConvertContext<'_>,
        settings: &Self::Settings,
    ) -> asset::ConvertAssetStatus<Self::Asset> {
        let source = match convert_ctx.get_asset(&settings.shader) {
            GetAssetResult::Loading => return ConvertAssetStatus::Loading,
            GetAssetResult::Error => return ConvertAssetStatus::Failed,
            GetAssetResult::Success(source) => source,
        }
        .clone();

        let mut import_sources = Vec::new();
        for import in source.imports.iter() {
            let conversion_result = convert_ctx.convert_asset::<ShaderWithImportsConverter>(
                &ShaderWithImportsConverterOptions::new(import.clone()),
            );
            match conversion_result {
                GetAssetResult::Loading => return ConvertAssetStatus::Loading,
                // TODO: add source failed?
                GetAssetResult::Error => return ConvertAssetStatus::Failed,
                GetAssetResult::Success(result) => import_sources.push(result.source.clone()),
            }
        }

        let mut resoved_source = String::new();
        for import in import_sources {
            // TODO: maybe insert on line it was included?
            resoved_source.push_str(&import);
        }
        resoved_source.push_str(&source.source);

        ConvertAssetStatus::Success(ShaderWithImportsFinal {
            source: resoved_source,
        })
    }
}

#[derive(Clone, Hash, PartialEq, Eq)]
pub struct ShaderWithImportsGpuConverterSettings {
    shader: AssetHandle<ShaderWithImports>,
}

impl ShaderWithImportsGpuConverterSettings {
    pub fn new(shader: AssetHandle<ShaderWithImports>) -> Self {
        Self { shader }
    }
}

pub struct ShaderWithImportsGpuConverter;

impl AssetConverter for ShaderWithImportsGpuConverter {
    type Asset = ArcShaderModule;
    type Settings = ShaderWithImportsGpuConverterSettings;
    type Error = EmptyError;

    fn convert(
        ctx: &mut Context,
        convert_ctx: &mut asset::ConvertContext,
        settings: &Self::Settings,
    ) -> ConvertAssetStatus<Self::Asset> {
        let shader_source = match convert_ctx.convert_asset::<ShaderWithImportsConverter>(
            &ShaderWithImportsConverterOptions::new(settings.shader.clone()),
        ) {
            GetAssetResult::Success(arc_handle) => arc_handle,
            GetAssetResult::Loading => return ConvertAssetStatus::Loading,
            GetAssetResult::Error => return ConvertAssetStatus::Failed,
        };

        #[cfg(not(target_arch = "wasm32"))]
        {
            let shader =
                render::ShaderBuilder::new().build_err_non_arc(ctx, shader_source.source.clone());

            match shader {
                Ok(shader) => ConvertAssetStatus::Success(ArcHandle::new(ctx, shader)),
                Err(err) => {
                    tracing::warn!("could not compile shader:\n{}", err);
                    ConvertAssetStatus::Failed
                }
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            let shader =
                render::ShaderBuilder::new().build_non_arc(ctx, shader_source.source.clone());
            ConvertAssetStatus::Success(ArcHandle::new(ctx, shader))
        }
    }
}
