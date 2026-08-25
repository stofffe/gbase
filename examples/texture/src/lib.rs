use gbase::{
    asset::{
        self, AssetHandle, ImageGpuConverter, ImageGpuConverterOptions, ImageLoader,
        ImageLoaderSettings, MeshGpuConverter, MeshGpuConverterSettings, ShaderGpuConverter,
        ShaderGpuConverterOptions, ShaderLoader, ShaderLoaderSettings,
    },
    render::{self, ArcHandle, ArcPipelineLayout, Image},
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
    mesh_handle: AssetHandle<render::Mesh>,

    shader_handle: AssetHandle<render::Shader>,
    shader_gpu: AssetHandle<ArcHandle<wgpu::ShaderModule>>,
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

        let shader_handle = asset::AssetBuilder::load::<ShaderLoader>()
            .build(cache, ShaderLoaderSettings::new("shaders/texture.wgsl"));
        let texture_handle = asset::AssetBuilder::load::<ImageLoader>()
            .build(cache, ImageLoaderSettings::new("textures/texture.jpeg"));

        let mesh = render::MeshBuilder::quad()
            .build()
            .with_extracted_attributes([
                render::VertexAttributeId::Position,
                render::VertexAttributeId::Uv(0),
            ]);
        let mesh_handle = asset::AssetBuilder::insert(mesh).build(cache);

        let shader_gpu = asset::convert_asset_new::<ShaderGpuConverter>(
            cache,
            ShaderGpuConverterOptions::new(shader_handle.clone()),
        );

        Self {
            pipeline_layout,
            bindgroup_layout,

            texture_handle,
            shader_handle,
            mesh_handle,

            shader_gpu,
        }
    }

    #[no_mangle]
    fn render(
        &mut self,
        ctx: &mut Context,
        cache: &mut gbase::asset::AssetCache,
        screen_view: &wgpu::TextureView,
    ) -> CallbackResult {
        let asset::GetAssetResult::Success(mesh) = asset::get_or_convert_asset::<MeshGpuConverter>(
            cache,
            MeshGpuConverterSettings::new(self.mesh_handle.clone()),
        ) else {
            return CallbackResult::Continue;
        };
        let mesh = mesh.clone();

        let asset::GetAssetResult::Success(shader) = asset::get(cache, self.shader_gpu.clone())
        else {
            return CallbackResult::Continue;
        };
        let shader = shader.clone();

        // NOTE: alternative way of loading shader
        // let asset::GetAssetResult::Success(shader) =
        // asset::get_or_convert_asset::<ShaderGpuConverter>(
        //     cache,
        //     ShaderGpuConverterSettings::new(self.shader_handle.clone()),
        // ) else {
        //     return CallbackResult::Continue;
        // };
        // let shader = mesh.clone();

        let asset::GetAssetResult::Success(texture) =
            asset::get_or_convert_asset::<ImageGpuConverter>(
                cache,
                ImageGpuConverterOptions::new(self.texture_handle.clone()),
            )
        else {
            return CallbackResult::Continue;
        };
        let texture = texture.clone();

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
