use gbase::{
    asset::{AssetLoader, LoadContext},
    filesystem::{self, LoadFileError},
    render::Shader,
};
use std::path::PathBuf;

//
// Shader string
//

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct ShaderWithImportsLoaderSettings {
    path: PathBuf,
}
impl ShaderWithImportsLoaderSettings {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }
}

pub struct ShaderWithImportsLoader;

impl AssetLoader for ShaderWithImportsLoader {
    type Asset = Shader;

    type Settings = ShaderWithImportsLoaderSettings;

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
                        .request_load::<ShaderWithImportsLoader>(settings_with_new_path)
                        .await;
                    let import_source = load_ctx.request_get(import_source_handle).await;

                    source.push_str(&import_source.source);

                    continue;
                }
            }

            source.push_str(line);
            source.push('\n');
        }

        Ok(Shader { source })
    }
}
