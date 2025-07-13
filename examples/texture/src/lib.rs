use gbase::{
    asset::{self, AssetHandle},
    render::{self, ArcPipelineLayout, GpuImage, Image},
    wgpu::{self},
    Callbacks, Context,
};

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub async fn run() {
    gbase::run::<App>().await;
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
        gbase::ContextBuilder::new().vsync(true)
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
        let shader_handle = asset::AssetBuilder::load("assets/shaders/texture.wgsl")
            .watch(cache)
            .build(cache);
        let texture_handle =
            asset::AssetBuilder::load::<render::Image>("assets/textures/texture.jpeg")
                // TODO:
                // .on_load(|img| {
                //     img.texture = img
                //         .texture
                //         .clone()
                //         .with_format(wgpu::TextureFormat::Rgba8Unorm)
                // })
                .watch(cache)
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
    ) -> bool {
        if !asset::handle_loaded(cache, self.mesh_handle)
            || !asset::handle_loaded(cache, self.shader_handle)
            || !asset::handle_loaded(cache, self.texture_handle)
        {
            return false;
        }
        let mesh =
            asset::convert_asset::<render::GpuMesh>(ctx, cache, self.mesh_handle, &()).unwrap();
        let shader =
            asset::convert_asset::<wgpu::ShaderModule>(ctx, cache, self.shader_handle, &())
                .unwrap();

        let texture =
            asset::convert_asset::<GpuImage>(ctx, cache, self.texture_handle, &()).unwrap();

        let bindgroup = render::BindGroupBuilder::new(self.bindgroup_layout.clone())
            .entries(vec![
                // texture
                render::BindGroupEntry::Texture(texture.view()),
                // sampler
                render::BindGroupEntry::Sampler(texture.sampler()),
            ])
            .build(ctx);

        // TODO: place this on gpumesh instead?
        let buffer_layout = asset::get(cache, self.mesh_handle).unwrap().buffer_layout();
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

        false
    }
}
