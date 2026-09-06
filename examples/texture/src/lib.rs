use gbase::{
    asset::{
        self, AssetHandle, ImageGpuLoader, ImageGpuLoaderSettings, ImageLoader,
        ImageLoaderSettings, MeshGpuLoader, MeshGpuLoaderSettings, NamedInserter, ShaderGpuLoader,
        ShaderGpuLoaderSettings, ShaderLoader, ShaderLoaderSettings,
    },
    render::{
        self, ArcPipelineLayout, ArcShaderModule, ArcTexture, GpuMesh, Mesh, SamplerBuilder,
        TextureViewBuilder,
    },
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

    texture_gpu_handle: AssetHandle<ArcTexture>,
    mesh_handle: AssetHandle<render::Mesh>,
    mesh_gpu_handle: AssetHandle<GpuMesh>,

    shader_gpu_handle: AssetHandle<ArcShaderModule>,
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

        let shader_handle = cache
            .load_asset::<ShaderLoader>(&ShaderLoaderSettings::from_path("shaders/texture.wgsl"));
        let shader_gpu_handle =
            cache.load_asset::<ShaderGpuLoader>(&ShaderGpuLoaderSettings::new(shader_handle));

        let texture_handle = cache
            .load_asset::<ImageLoader>(&ImageLoaderSettings::from_path("textures/texture.jpeg"));
        let texture_gpu_handle =
            cache.load_asset::<ImageGpuLoader>(&ImageGpuLoaderSettings::new(texture_handle));

        let mesh = render::MeshBuilder::quad()
            .build()
            .with_extracted_attributes([
                render::VertexAttributeId::Position,
                render::VertexAttributeId::Uv(0),
            ]);
        let mesh_handle = cache.insert_asset::<Mesh, NamedInserter>("quad mesh", mesh);
        let mesh_gpu_handle =
            cache.load_asset::<MeshGpuLoader>(&MeshGpuLoaderSettings::new(mesh_handle.clone()));

        Self {
            pipeline_layout,
            bindgroup_layout,

            texture_gpu_handle,
            mesh_handle,
            mesh_gpu_handle,
            shader_gpu_handle,
        }
    }

    #[no_mangle]
    fn render(
        &mut self,
        ctx: &mut Context,
        cache: &mut gbase::asset::AssetCache,
        screen_view: &wgpu::TextureView,
    ) -> CallbackResult {
        let Ok(gpu_mesh) = cache.get_asset_cloned(&self.mesh_gpu_handle) else {
            return CallbackResult::Continue;
        };

        let Ok(shader) = cache.get_asset_cloned(&self.shader_gpu_handle) else {
            return CallbackResult::Continue;
        };

        let Ok(texture) = cache.get_asset_cloned(&self.texture_gpu_handle) else {
            return CallbackResult::Continue;
        };

        let view = TextureViewBuilder::new(texture.clone()).build(ctx);
        let sampler = SamplerBuilder::new().build(ctx);
        let bindgroup = render::BindGroupBuilder::new(self.bindgroup_layout.clone())
            .entries(vec![
                // texture
                render::BindGroupEntry::Texture(view),
                // sampler
                render::BindGroupEntry::Sampler(sampler),
            ])
            .build(ctx);

        // TODO: place this on gpumesh instead?
        let buffer_layout = asset::get_asset(cache, self.mesh_handle.clone())
            .unwrap()
            .buffer_layout();
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

                gpu_mesh.bind_to_render_pass(&mut render_pass);

                render_pass.set_bind_group(0, Some(bindgroup.as_ref()), &[]);
                render_pass.draw_indexed(0..gpu_mesh.index_count.unwrap(), 0, 0..1);
            });

        CallbackResult::Continue
    }
}
