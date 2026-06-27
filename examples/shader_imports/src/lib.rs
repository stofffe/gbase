use gbase::{
    asset::{
        self, AssetHandle, ImageGpuConverter, ImageLoader, MeshGpuConverter, ShaderGpuConverter,
    },
    filesystem,
    render::{self, ArcPipelineLayout, Image},
    wgpu::{self},
    CallbackResult, Callbacks, Context,
};
use std::collections::{HashSet, VecDeque};

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub fn run() {
    gbase::run::<App>();
}

#[derive(Debug, Clone)]
pub struct ShaderExtendedLoader {}

impl asset::AssetLoader for ShaderExtendedLoader {
    type Asset = render::ShaderBuilder;
    type Error = filesystem::LoadFileError;

    async fn load(
        &self,
        load_ctx: asset::LoadContext,
        path: &std::path::Path,
    ) -> Result<Self::Asset, Self::Error> {
        // pseduo code
        // load file content of path (for this asset)
        // for each import
        //  load new asset
        //  add new asset to dependencies (load ctx?)
        //
        // wait for dependencies? async? (need wait for single asset load?)
        // combine str

        let mut imported_paths = HashSet::new();
        let mut import_path_stack = VecDeque::new();

        let normalized_path = filesystem::normalize_path(path);
        import_path_stack.push_front(normalized_path);

        let mut output = String::new();
        while let Some(path) = import_path_stack.pop_front() {
            // only include once
            if imported_paths.contains(&path) {
                continue;
            } else {
                imported_paths.insert(path.clone());
            }

            let source_code = load_ctx.load_string(&path).await?;

            // resolve imports
            for line in source_code.lines() {
                if let Some(rest) = line.trim().strip_prefix("import \"") {
                    if let Some(import_relative_path) = rest.strip_suffix('"') {
                        let parent_folder = path.parent().expect("could not get parent");
                        let full_path = parent_folder
                            .join(import_relative_path)
                            .with_extension("wgsl");
                        let normalized_full_path = filesystem::normalize_path(full_path);
                        import_path_stack.push_front(normalized_full_path);
                        continue;
                    }
                }

                output.push_str(line);
                output.push('\n');
            }
        }

        // TODO: add defines

        Ok(render::ShaderBuilder::new(output))
    }
}

struct App {
    pipeline_layout: ArcPipelineLayout,
    bindgroup_layout: render::ArcBindGroupLayout,

    texture_handle: AssetHandle<Image>,
    shader_handle: AssetHandle<render::ShaderBuilder>,
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
        let shader_handle = asset::AssetBuilder::load(
            cache,
            "shaders/texture_import.wgsl",
            ShaderExtendedLoader {},
        )
        .watch(ctx, cache)
        .build(cache);
        let texture_handle =
            asset::AssetBuilder::load(cache, "textures/texture.jpeg", ImageLoader {})
                .watch(ctx, cache)
                .build(cache);

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
        if !asset::handle_loaded(cache, self.mesh_handle.clone())
            || !asset::handle_loaded(cache, self.shader_handle.clone())
            || !asset::handle_loaded(cache, self.texture_handle.clone())
        {
            return CallbackResult::Continue;
        }
        let mesh = asset::convert_asset(ctx, cache, self.mesh_handle.clone(), MeshGpuConverter)
            .unwrap_success();
        let shader =
            asset::convert_asset(ctx, cache, self.shader_handle.clone(), ShaderGpuConverter)
                .unwrap_success();
        let texture =
            asset::convert_asset(ctx, cache, self.texture_handle.clone(), ImageGpuConverter)
                .unwrap_success();

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
