mod shader_import_asset;
pub use shader_import_asset::*;

use gbase::input::{self, KeyCode};
use gbase::render::{ArcShaderModule, GpuImage, GpuMesh};
use gbase::{
    asset::{
        self, AssetHandle, ImageGpuConverter, ImageGpuConverterOptions, ImageLoader,
        ImageLoaderSettings, MeshGpuConverter, MeshGpuConverterSettings,
    },
    render::{self, ArcPipelineLayout, Image},
    tracing,
    wgpu::{self},
    CallbackResult, Callbacks, Context,
};

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub fn run() {
    gbase::run::<App>();
}

struct App {
    pipeline_layout: ArcPipelineLayout,
    bindgroup_layout: render::ArcBindGroupLayout,

    texture_handle: AssetHandle<Image>,
    texture_gpu_handle: AssetHandle<GpuImage>,

    mesh_handle: AssetHandle<render::Mesh>,
    mesh_gpu_handle: AssetHandle<GpuMesh>,

    shader_handle: AssetHandle<ShaderWithImports>,
    shader_gpu_handle: AssetHandle<ArcShaderModule>,
}

impl Callbacks for App {
    #[no_mangle]
    fn init_ctx() -> gbase::ContextBuilder {
        gbase::ContextBuilder::new()
            .vsync(true)
            .log_level(tracing::Level::INFO)
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

        let shader_handle = cache.load_asset::<ShaderWithImportsLoader>(
            &ShaderWithImportsLoaderSettings::new("shaders/texture_import.wgsl"),
        );
        let shader_gpu_handle = cache.convert_asset::<ShaderWithImportsGpuConverter>(
            &ShaderWithImportsGpuConverterSettings::new(shader_handle.clone()),
        );

        let texture_handle =
            cache.load_asset::<ImageLoader>(&ImageLoaderSettings::new("textures/texture.jpeg"));
        let texture_gpu_handle = cache.convert_asset::<ImageGpuConverter>(
            &ImageGpuConverterOptions::new(texture_handle.clone()),
        );

        let mesh = render::MeshBuilder::quad()
            .build()
            .with_extracted_attributes([
                render::VertexAttributeId::Position,
                render::VertexAttributeId::Uv(0),
            ]);
        let mesh_handle = asset::insert_asset_force(cache, mesh);
        let mesh_gpu_handle = cache
            .convert_asset::<MeshGpuConverter>(&MeshGpuConverterSettings::new(mesh_handle.clone()));

        Self {
            pipeline_layout,
            bindgroup_layout,

            texture_handle,
            texture_gpu_handle,
            shader_handle,
            shader_gpu_handle,
            mesh_handle,
            mesh_gpu_handle,
        }
    }

    #[no_mangle]
    fn render(
        &mut self,
        ctx: &mut Context,
        cache: &mut gbase::asset::AssetCache,
        screen_view: &wgpu::TextureView,
    ) -> CallbackResult {
        if input::key_just_pressed(ctx, KeyCode::KeyD) {
            asset::debug_asset_dependency_graph(cache);
        }

        let Ok(mesh) = cache.get_asset(&self.mesh_gpu_handle).cloned() else {
            return CallbackResult::Continue;
        };

        let Ok(shader) = cache.get_asset(&self.shader_gpu_handle).cloned() else {
            return CallbackResult::Continue;
        };

        let Ok(texture) = cache.get_asset(&self.texture_gpu_handle) else {
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
        let Ok(buffer) = asset::get_asset(cache, self.mesh_handle.clone()) else {
            return CallbackResult::Continue;
        };
        let buffer_layout = buffer.buffer_layout();

        let pipeline =
            render::RenderPipelineBuilder::new(shader.clone(), self.pipeline_layout.clone())
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
