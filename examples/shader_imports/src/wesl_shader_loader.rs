use gbase::{
    asset::AssetLoader,
    filesystem::{FileSystemRuntime, LoadFileError},
    render::Shader,
};
use std::path::PathBuf;

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct WeslShaderLoaderSettings {
    root: PathBuf,
    file: PathBuf,
}
impl WeslShaderLoaderSettings {
    pub fn new(root: impl Into<PathBuf>, file: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            file: file.into(),
        }
    }
}

pub struct WeslShaderLoader {}

impl AssetLoader for WeslShaderLoader {
    type Asset = Shader;
    type Settings = WeslShaderLoaderSettings;
    type Error = LoadFileError;

    async fn load(
        load_ctx: &mut gbase::asset::LoadContext,
        settings: Self::Settings,
    ) -> Result<Self::Asset, Self::Error> {
        let compiler = wesl::Compiler::default();
        let compile_result = compiler
            .with_resolver(AsyncWeslResolver::new(
                load_ctx.filesystem_runtime().clone(),
                load_ctx
                    .filesystem_runtime()
                    .format_asset_path(settings.root),
            ))
            .compile_async(&settings.file)
            .await
            .inspect_err(|e| eprintln!("WESL error: {e}")) // pretty-print error diagnostics
            .expect("compilation error");
        let wgsl_string = compile_result.syntax.to_string();

        Ok(Shader {
            source: wgsl_string,
        })
    }
}

pub struct AsyncWeslResolver {
    filesystem: FileSystemRuntime,
    root: PathBuf,
}

impl AsyncWeslResolver {
    pub fn new(filesystem: FileSystemRuntime, pkg_root_dir: impl Into<PathBuf>) -> Self {
        Self {
            filesystem,
            root: pkg_root_dir.into(),
        }
    }
}

impl wesl::Resolver for AsyncWeslResolver {}
impl wesl::AsyncResolver for AsyncWeslResolver {
    async fn resolve_source_async<'a>(
        &'a self,
        path: &wesl::syntax::ModulePath,
    ) -> Result<std::borrow::Cow<'a, str>, wesl::error::ResolveError> {
        if path.origin.is_package() {
            panic!("package imports not implemented yet");
        }
        let mut fs_path = path.to_path_buf();
        dbg!(&path.components);
        if path.is_root() {
            // the root module path (package) corresponds to the special file `package.wesl` in the root directory.
            fs_path.push("package");
        }

        let path = self.root.join(fs_path.strip_prefix("/").unwrap());

        let path = path.with_extension("wesl");
        if let Ok(source) = self.filesystem.load_string(&path).await {
            return Ok(source.into());
        }

        let path = path.with_extension("wgsl");
        if let Ok(source) = self.filesystem.load_string(&path).await {
            return Ok(source.into());
        }

        Err(wesl::error::ResolveError::FileNotFound(path, "".into()))
    }
}
