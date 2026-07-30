use std::path::PathBuf;

use gbase::{
    asset::{
        self, Asset, AssetConverter, AssetHandle, ConvertAssetResult, ConvertAssetStatus,
        ConvertContext, DerivedAsset, EmptyError, GetAssetResult, ImageGpuConverter,
        ImageGpuConverterOptions, ImageLoader, ImageLoaderSettings, LoadContext, MeshGpuConverter,
        MeshGpuConverterSettings,
    },
    filesystem,
    render::{self, ArcPipelineLayout, Image},
    tracing,
    wgpu::{self},
    CallbackResult, Callbacks, Context,
};

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub fn run() {
    gbase::run::<App>();
}

#[derive(Debug, Clone)]
pub struct ShaderWithImports {
    source: String,
    imports: Vec<AssetHandle<ShaderWithImports>>,
}

impl Asset for ShaderWithImports {}

#[derive(Clone)]
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
                    let import =
                        load_ctx.load_asset::<ShaderWithImportsLoader>(settings_with_new_path);

                    imports.push(import);

                    continue;
                }
            }

            source.push_str(line);
            source.push('\n');
        }

        // tracing::info!("LOADED ASSET {}\n{}", load_ctx.handle().id(), source);

        Ok(ShaderWithImports { source, imports })
    }
}

#[derive(Clone)]
struct ShaderWithImportsFinal {
    source: String,
}

impl DerivedAsset for ShaderWithImportsFinal {}

#[derive(Clone, Hash, PartialEq, Eq)]
pub struct ShaderWithImportsConverterOptions {
    shader: AssetHandle<ShaderWithImports>,
}
impl ShaderWithImportsConverterOptions {
    pub fn new(shader: AssetHandle<ShaderWithImports>) -> Self {
        Self { shader }
    }
}

struct ShaderWithImportsConverter {}

impl AssetConverter for ShaderWithImportsConverter {
    type TargetAsset = ShaderWithImportsFinal;
    type Settings = ShaderWithImportsConverterOptions;
    type Error = EmptyError;

    fn convert(
        ctx: &mut Context,
        convert_ctx: &mut ConvertContext<'_, '_>, // TODO: should this be mutable reference?
        settings: &Self::Settings,
    ) -> asset::ConvertAssetStatus<Self::TargetAsset> {
        let source = match convert_ctx.get(&settings.shader) {
            GetAssetResult::Loading => return ConvertAssetStatus::SourceLoading,
            GetAssetResult::Error => return ConvertAssetStatus::Failed,
            GetAssetResult::Success(source) => source,
        }
        .clone();

        let mut import_sources = Vec::new();
        for import in source.imports.iter() {
            let conversion_result = convert_ctx.convert::<ShaderWithImportsConverter>(
                ctx,
                &ShaderWithImportsConverterOptions::new(import.clone()),
            );
            match conversion_result {
                asset::ConvertAssetResult::Loading => return ConvertAssetStatus::SourceLoading,
                // TODO: add source failed?
                asset::ConvertAssetResult::Failed => return ConvertAssetStatus::Failed,
                asset::ConvertAssetResult::Success(result) => {
                    import_sources.push(result.source.clone())
                }
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

struct ShaderWithImportsGpuConverter;

impl AssetConverter for ShaderWithImportsGpuConverter {
    type TargetAsset = wgpu::ShaderModule;
    type Settings = ShaderWithImportsGpuConverterSettings;
    type Error = EmptyError;

    fn convert(
        ctx: &mut Context,
        convert_ctx: &mut asset::ConvertContext,
        settings: &Self::Settings,
    ) -> ConvertAssetStatus<Self::TargetAsset> {
        let source = match convert_ctx.get(&settings.shader) {
            GetAssetResult::Loading => return ConvertAssetStatus::SourceLoading,
            GetAssetResult::Error => return ConvertAssetStatus::Failed,
            GetAssetResult::Success(source) => source,
        }
        .clone();

        let mut import_sources = Vec::new();
        for import in source.imports.iter() {
            let conversion_result = convert_ctx.convert::<ShaderWithImportsConverter>(
                ctx,
                &ShaderWithImportsConverterOptions::new(import.clone()),
            );
            match conversion_result {
                asset::ConvertAssetResult::Loading => return ConvertAssetStatus::SourceLoading,
                // TODO: add source failed?
                asset::ConvertAssetResult::Failed => return ConvertAssetStatus::Failed,
                asset::ConvertAssetResult::Success(result) => {
                    import_sources.push(result.source.clone())
                }
            }
        }

        let mut resoved_source = String::new();
        for import in import_sources {
            // TODO: maybe insert on line it was included?
            resoved_source.push_str(&import);
        }
        resoved_source.push_str(&source.source);

        tracing::info!("CONVERTED GPU SHADER\n{}", &resoved_source);

        let shader = render::ShaderBuilder::new().build_non_arc(ctx, resoved_source);

        ConvertAssetStatus::Success(shader)
    }
}

// #[derive(Clone, Hash, PartialEq, Eq)]
// pub struct ShaderWithImportsGpuConverterSettings {
//     shader: AssetHandle<ShaderWithImports>,
// }
//
// impl ShaderWithImportsGpuConverterSettings {
//     pub fn new(shader: AssetHandle<ShaderWithImports>) -> Self {
//         Self { shader }
//     }
// }
//
// struct ShaderWithImportsGpuConverter;
//
// impl AssetConverter for ShaderWithImportsGpuConverter {
//     type TargetAsset = wgpu::ShaderModule;
//     type Settings = ShaderWithImportsGpuConverterSettings;
//     type Error = EmptyError;
//
//     fn convert(
//         ctx: &mut Context,
//         convert_ctx: &mut asset::ConvertContext,
//         settings: &Self::Settings,
//     ) -> ConvertAssetStatus<Self::TargetAsset> {
//         let result = convert_ctx.convert::<ShaderWithImportsConverter>(
//             ctx,
//             &ShaderWithImportsConverterOptions::new(settings.shader.clone()),
//         );
//
//         let shader = match result {
//             asset::ConvertAssetResult::Loading => return ConvertAssetStatus::SourceLoading,
//             asset::ConvertAssetResult::Failed => return ConvertAssetStatus::Failed,
//             asset::ConvertAssetResult::Success(arc_handle) => arc_handle,
//         };
//
//         let shader = render::ShaderBuilder::new().build_non_arc(ctx, shader.source.clone());
//
//         ConvertAssetStatus::Success(shader)
//     }
// }

struct App {
    pipeline_layout: ArcPipelineLayout,
    bindgroup_layout: render::ArcBindGroupLayout,

    texture_handle: AssetHandle<Image>,
    shader_handle: AssetHandle<ShaderWithImports>,
    mesh_handle: AssetHandle<render::Mesh>,
}

impl Callbacks for App {
    #[no_mangle]
    fn init_ctx() -> gbase::ContextBuilder {
        gbase::ContextBuilder::new()
            .vsync(true)
            .assets_path("assets")
    }
    #[no_mangle]
    fn new(ctx: &mut Context, cache: &mut gbase::asset::AssetCache) -> Self {
        let bindgroup_layout = render::BindGroupLayoutBuilder::new()
            .entries(vec![
                // texture
                render::BindGroupLayoutEntry::new()
                    .fragment()
                    .texture_float_filterable(),
                // sampler
                render::BindGroupLayoutEntry::new()
                    .fragment()
                    .sampler_filtering(),
            ])
            .build(ctx);

        let pipeline_layout = render::PipelineLayoutBuilder::new()
            .bind_groups(vec![bindgroup_layout.clone()])
            .build_uncached(ctx);
        let shader_handle = asset::AssetBuilder::load::<ShaderWithImportsLoader>().build(
            cache,
            ShaderWithImportsLoaderSettings::new("shaders/texture_import.wgsl"),
        );
        let texture_handle = asset::AssetBuilder::load::<ImageLoader>()
            .build(cache, ImageLoaderSettings::new("textures/texture.jpeg"));

        let mesh = render::MeshBuilder::quad()
            .build()
            .with_extracted_attributes([
                render::VertexAttributeId::Position,
                render::VertexAttributeId::Uv(0),
            ]);
        let mesh_handle = asset::AssetBuilder::insert(mesh).build(cache);

        Self {
            pipeline_layout,
            bindgroup_layout,

            texture_handle,
            shader_handle,
            mesh_handle,
        }
    }

    #[no_mangle]
    fn render(
        &mut self,
        ctx: &mut Context,
        cache: &mut gbase::asset::AssetCache,
        screen_view: &wgpu::TextureView,
    ) -> CallbackResult {
        let ConvertAssetResult::Success(mesh) = asset::convert_asset::<MeshGpuConverter>(
            ctx,
            cache,
            &MeshGpuConverterSettings::new(self.mesh_handle.clone()),
        ) else {
            return CallbackResult::Continue;
        };

        let ConvertAssetResult::Success(shader) =
            asset::convert_asset::<ShaderWithImportsGpuConverter>(
                ctx,
                cache,
                &ShaderWithImportsGpuConverterSettings::new(self.shader_handle.clone()),
            )
        else {
            return CallbackResult::Continue;
        };

        let ConvertAssetResult::Success(texture) = asset::convert_asset::<ImageGpuConverter>(
            ctx,
            cache,
            &ImageGpuConverterOptions::new(self.texture_handle.clone()),
        ) else {
            return CallbackResult::Continue;
        };

        let bindgroup = render::BindGroupBuilder::new(self.bindgroup_layout.clone())
            .entries(vec![
                // texture
                render::BindGroupEntry::Texture(texture.view()),
                // sampler
                render::BindGroupEntry::Sampler(texture.sampler()),
            ])
            .build(ctx);

        // TODO: place this on gpumesh instead?
        let buffer_layout = asset::get(cache, self.mesh_handle.clone())
            .unwrap_success()
            .buffer_layout();
        let pipeline = render::RenderPipelineBuilder::new(shader, self.pipeline_layout.clone())
            .single_target(render::ColorTargetState::from_current_screen(ctx))
            .buffers(buffer_layout)
            .build(ctx);

        render::RenderPassBuilder::new()
            .color_attachments(&[Some(
                render::RenderPassColorAttachment::new(screen_view).clear(wgpu::Color::BLACK),
            )])
            .build_run_submit(ctx, |mut render_pass| {
                render_pass.set_pipeline(&pipeline);

                mesh.bind_to_render_pass(&mut render_pass);

                render_pass.set_bind_group(0, Some(bindgroup.as_ref()), &[]);
                render_pass.draw_indexed(0..mesh.index_count.unwrap(), 0, 0..1);
            });

        CallbackResult::Continue
    }
}
