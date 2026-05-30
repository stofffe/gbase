use std::{fs, os::unix::raw::mode_t, path::PathBuf};

use gbase::{
    asset::{
        self, AssetHandle, AssetLoader, ExtendedShaderLoader, ImageGpuConverter, ImageLoader,
        MeshGpuConverter, ShaderGpuConverter, ShaderLoader,
    },
    filesystem::LoadFileError,
    render::{self, ArcPipelineLayout, Image, ShaderBuilder},
    wgpu::{self},
    CallbackResult, Callbacks, Context,
};
use wesl::{syntax::PathOrigin, ModulePath};

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub async fn run() {
    gbase::run::<App>().await;
}

#[derive(Debug, Clone)]
pub struct WeslShaderLoader {
    package_folder: PathBuf,
}

impl AssetLoader for WeslShaderLoader {
    type Asset = ShaderBuilder;
    type Error = LoadFileError;

    async fn load(
        &self,
        load_ctx: asset::LoadContext,
        path: &std::path::Path,
    ) -> Result<Self::Asset, Self::Error> {
        let mut virtual_resolver = wesl::VirtualResolver::new();

        // let package_name = path.file_name().unwrap();
        // dbg!(&package_name);
        //
        // let mut stack = vec![self.folder.clone()];
        //
        // while let Some(p) = stack.pop() {
        //     if p.is_dir() {
        //         for child in p.read_dir().unwrap() {
        //             let path = child.unwrap().path();
        //             stack.push(path.to_path_buf());
        //         }
        //     } else {
        //         let source_code = load_ctx.load_string(&p).await?;
        //         let translation =
        //             wgsl_parse::parse_str(&source_code).expect("could not parse wesl file");
        //
        //         let p = p.strip_prefix("assets").unwrap();
        //         // dbg!(&p);
        //         let m = ModulePath::from_path(p);
        //         // dbg!(&m);
        //         virtual_resolver.add_translation_unit(m, translation);
        //     }
        // }

        let base_path = self.package_folder.clone();
        let root_path = path.to_path_buf();
        let stripped_root_path = root_path.strip_prefix(base_path);

        // assert!(base_path.is_dir());
        // assert!(root_path.is_file());

        let root = ModulePath::from_path(path);
        // let mut root = ModulePath::new_root();
        // root.push("texture");

        dbg!(&root);

        let mut stack = vec![root];
        while let Some(node) = stack.pop() {
            dbg!(&node);
            let full_path = node.to_path_buf().with_extension("wgsl");
            dbg!(&full_path);
            let source_code = load_ctx.load_string(&full_path).await?;
            let translation =
                wgsl_parse::parse_str(&source_code).expect("could not parse wesl file");

            for import in translation.imports.iter() {
                if let Some(import_path) = &import.path {
                    dbg!(import_path);
                    stack.push(import_path.clone());
                }
            }

            dbg!(&node);
            virtual_resolver.add_translation_unit(node, translation);
        }

        let compiled_source = wesl::Wesl::new(&self.package_folder)
            .set_custom_resolver(virtual_resolver)
            .compile(&ModulePath::from_path(root_path))
            .inspect_err(|e| {
                eprintln!("{e}");
                panic!();
            })
            .unwrap()
            .to_string();

        dbg!(&compiled_source);

        Ok(ShaderBuilder::new(compiled_source))
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
        // let shader_handle =
        //     asset::AssetBuilder::load(cache, "shaders/texture.wgsl", ShaderLoader {})
        //         .watch(ctx, cache)
        //         .build(cache);
        let shader_handle = asset::AssetBuilder::load(
            cache,
            "shaders/texture.wgsl",
            WeslShaderLoader {
                package_folder: PathBuf::from("shaders"),
            },
        )
        .watch(ctx, cache)
        .build(cache);
        // let shader_handle = asset::AssetBuilder::load(
        //     cache,
        //     "assets/shaders/texture.wgsl",
        //     WeslShaderLoader {
        //         package_folder: PathBuf::from("assets/shaders"),
        //     },
        // )
        // .watch(ctx, cache)
        // .build(cache);
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
