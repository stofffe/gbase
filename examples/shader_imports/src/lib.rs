use gbase::{
    asset::{
        self, Asset, AssetConverter, AssetHandle, ConvertAssetResult, ConvertAssetStatus,
        DerivedAsset, EmptyError, GetAssetResult, ImageGpuConverter, ImageLoader, MeshGpuConverter,
        NoSettings, ShaderGpuConverter,
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

#[derive(Debug, Clone)]
pub struct ShaderWithImportsLoader {}

impl asset::AssetLoader for ShaderWithImportsLoader {
    type Asset = ShaderWithImports;
    type Settings = NoSettings;
    type Error = filesystem::LoadFileError;

    async fn load(
        load_ctx: asset::LoadContext,
        path: &std::path::Path,
        settings: Self::Settings,
    ) -> Result<Self::Asset, Self::Error> {
        let mut source = String::new();
        let mut imports = Vec::new();

        let source_code = load_ctx.load_string(&path).await?;

        for line in source_code.lines() {
            if let Some(rest) = line.trim().strip_prefix("import \"") {
                if let Some(import_relative_path) = rest.strip_suffix('"') {
                    let parent_folder = path.parent().expect("could not get parent");
                    let full_path = parent_folder
                        .join(import_relative_path)
                        .with_extension("wgsl");
                    let normalized_full_path = filesystem::normalize_path(full_path);

                    let import = load_ctx.request_load::<ShaderWithImportsLoader>(
                        &normalized_full_path,
                        settings.clone(),
                    );

                    imports.push(import);

                    continue;
                }
            }

            source.push_str(line);
            source.push('\n');
        }

        tracing::info!("LOADED ASSET SOURCE\n {}", source);
        tracing::info!("LOADED ASSET IMPORTS\n {:?}", imports);

        Ok(ShaderWithImports { source, imports })
    }
}

#[derive(Clone)]
struct ShaderWithImportsFinal {
    source: String,
}

impl DerivedAsset for ShaderWithImportsFinal {}

struct ShaderWithImportsConverter {}

impl AssetConverter for ShaderWithImportsConverter {
    type SourceAsset = ShaderWithImports;
    type TargetAsset = ShaderWithImportsFinal;
    type Settings = NoSettings;
    type Error = EmptyError;

    fn convert(
        ctx: &mut Context,
        cache: &mut asset::AssetCache,
        source: AssetHandle<Self::SourceAsset>, // TODO: make this refernce?
        settings: &Self::Settings,
    ) -> asset::ConvertAssetStatus<Self::TargetAsset> {
        let source = match source.get(cache) {
            GetAssetResult::Loading => return ConvertAssetStatus::SourceLoading,
            GetAssetResult::Failed => return ConvertAssetStatus::Failed,
            GetAssetResult::Success(source) => source,
        }
        .clone();

        let mut import_sources = Vec::new();
        for import in source.imports.iter() {
            let conversion_result =
                import.convert_custom_settings::<ShaderWithImportsConverter>(ctx, cache, settings);
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

        tracing::info!("CONVERTED ASSET {}", resoved_source);

        ConvertAssetStatus::Success(ShaderWithImportsFinal {
            source: resoved_source,
        })
    }
}

struct ShaderWithImportGpuConverter;

impl AssetConverter for ShaderWithImportGpuConverter {
    type SourceAsset = ShaderWithImports;
    type TargetAsset = wgpu::ShaderModule;
    type Settings = NoSettings;
    type Error = EmptyError;

    fn convert(
        ctx: &mut Context,
        cache: &mut asset::AssetCache,
        source: AssetHandle<Self::SourceAsset>, // TODO: make this refernce?
        settings: &Self::Settings,
    ) -> ConvertAssetStatus<Self::TargetAsset> {
        let result = cache.convert::<ShaderWithImportsConverter>(ctx, source.clone(), settings);

        let shader = match result {
            asset::ConvertAssetResult::Loading => return ConvertAssetStatus::SourceLoading,
            asset::ConvertAssetResult::Failed => return ConvertAssetStatus::Failed,
            asset::ConvertAssetResult::Success(arc_handle) => arc_handle,
        };

        let shader = render::ShaderBuilder::new().build_non_arc(ctx, shader.source.clone());

        ConvertAssetStatus::Success(shader)
    }
}

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
        let shader_handle =
            asset::AssetBuilder::load::<ShaderWithImportsLoader>("shaders/texture_import.wgsl")
                .watch(true)
                .build_default_settings(ctx, cache);
        let texture_handle = asset::AssetBuilder::load::<ImageLoader>("textures/texture.jpeg")
            .watch(true)
            .build_default_settings(ctx, cache);

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
        let ConvertAssetResult::Success(mesh) = asset::convert_asset_default_settings::<
            MeshGpuConverter,
        >(ctx, cache, self.mesh_handle.clone()) else {
            return CallbackResult::Continue;
        };

        let ConvertAssetResult::Success(shader) = asset::convert_asset_default_settings::<
            ShaderWithImportGpuConverter,
        >(ctx, cache, self.shader_handle.clone()) else {
            return CallbackResult::Continue;
        };

        let ConvertAssetResult::Success(texture) = asset::convert_asset_default_settings::<
            ImageGpuConverter,
        >(
            ctx, cache, self.texture_handle.clone()
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
            .unwrap_loaded()
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
