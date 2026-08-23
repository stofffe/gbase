use gbase::{
    asset::{
        self, AssetHandle, ConvertAssetResult, DerivedHandle, ImageGpuConverter,
        ImageGpuConverterOptions, ImageLoader, ImageLoaderSettings, MeshGpuConverter,
        MeshGpuConverterSettings, ShaderGpuConverter, ShaderGpuConverterOptions, ShaderLoader,
        ShaderLoaderSettings,
    },
    render::{self, ArcPipelineLayout, Image},
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
    shader_handle: AssetHandle<render::Shader>,
    mesh_handle: AssetHandle<render::Mesh>,

    shader_derived: DerivedHandle<wgpu::ShaderModule>,
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

        let shader_derived = asset::convert_derived_asset::<ShaderGpuConverter>(
            cache,
            ShaderGpuConverterOptions::new(shader_handle.clone()),
        );

        Self {
            pipeline_layout,
            bindgroup_layout,

            texture_handle,
            shader_handle,
            mesh_handle,

            shader_derived,
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

        // let ConvertAssetResult::Success(shader) = asset::convert_asset::<ShaderGpuConverter>(
        //     ctx,
        //     cache,
        //     &ShaderGpuConverterOptions::new(self.shader_handle.clone()),
        // ) else {
        //     return CallbackResult::Continue;
        // };

        let shader_derived = asset::convert_derived_asset::<ShaderGpuConverter>(
            cache,
            ShaderGpuConverterOptions::new(self.shader_handle.clone()),
        );

        let asset::GetDerivedResult::Success(shader) =
            asset::get_derived_asset(cache, shader_derived.clone())
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
