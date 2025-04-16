use gbase::{
    filesystem, input, log,
    render::{self, ArcPipelineLayout, SamplerBuilder, ShaderBuilder},
    wgpu::{self},
    Callbacks, Context,
};
use gbase_utils::{AssetCache, AssetHandle, Assets, Image, Mesh, ShaderDescriptor, BLACK};

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub async fn run() {
    gbase::run::<App>().await;
}

struct App {
    pipeline_layout: ArcPipelineLayout,
    bindgroup_layout: render::ArcBindGroupLayout,

    assets: Assets,
    mesh_handle: AssetHandle<Mesh>,
    texture_handle: AssetHandle<Image>,
    shader_handle: AssetHandle<ShaderDescriptor>,
    shader_cache: AssetCache<ShaderDescriptor, wgpu::ShaderModule>,
}

impl Callbacks for App {
    fn init_ctx() -> gbase::ContextBuilder {
        gbase::ContextBuilder::new().vsync(false)
    }
    fn new(ctx: &mut Context) -> Self {
        let mut assets = Assets::new();

        let mesh = gbase_utils::MeshBuilder::quad().build().extract_attributes(
            &[
                gbase_utils::VertexAttributeId::Position,
                gbase_utils::VertexAttributeId::Uv(0),
            ]
            .into(),
        );

        let image = Image {
            texture: gbase_utils::texture_builder_from_image_bytes(
                &filesystem::load_b!("textures/texture.jpeg").unwrap(),
            )
            .unwrap(),
            sampler: SamplerBuilder::new(),
        };
        let texture_handle = assets.allocate_image_data(image);
        assets.watch_image(
            "assets/textures/texture.jpeg".into(),
            texture_handle.clone(),
        );

        // let shader_str = filesystem::load_s!("shaders/texture.wgsl").unwrap();
        // let shader = render::ShaderBuilder::new(shader_str);
        // let shader_handle = assets.allocate_shader_data(shader);
        // assets.watch_shader("assets/shaders/texture.wgsl".into(), shader_handle.clone());

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

        let mesh_handle = assets.allocate_mesh_data(mesh);

        let mut shader_cache = AssetCache::new();
        // let shader_str = filesystem::load_s!("shaders/texture.wgsl").unwrap();
        // let shader = render::ShaderBuilder::new(shader_str);
        let shader_descriptor = ShaderDescriptor {
            label: None,
            source: filesystem::load_s!("shaders/texture.wgsl").unwrap(),
        };
        let shader_handle =
            shader_cache.allocate_reload(shader_descriptor, "assets/shaders/texture.wgsl".into());
        // let shader_handle = shader_cache.allocate(shader);
        // shader_cache.watch("assets/shaders/texture.wgsl".into(), shader_handle.clone());

        Self {
            shader_handle,
            pipeline_layout,
            bindgroup_layout,
            assets,
            texture_handle,
            mesh_handle,
            shader_cache,
        }
    }
    fn render(&mut self, ctx: &mut Context, screen_view: &wgpu::TextureView) -> bool {
        self.shader_cache.check_watch(ctx);
        // clear
        render::RenderPassBuilder::new()
            .color_attachments(&[Some(
                render::RenderPassColorAttachment::new(screen_view).clear(wgpu::Color::BLACK),
            )])
            .build_run_submit(ctx, |_| {});

        self.assets.check_watch_images();
        self.assets.check_watch_shaders();

        match self.shader_cache.get_gpu(ctx, self.shader_handle.clone()) {
            Ok(shader) => {
                let mesh_gpu = self.assets.get_mesh_gpu(ctx, self.mesh_handle.clone());
                let texture = self.assets.get_image_gpu(ctx, self.texture_handle.clone());
                let bindgroup = render::BindGroupBuilder::new(self.bindgroup_layout.clone())
                    .entries(vec![
                        // texture
                        render::BindGroupEntry::Texture(texture.view()),
                        // sampler
                        render::BindGroupEntry::Sampler(texture.sampler()),
                    ])
                    .build(ctx);

                let buffer_layout = self
                    .assets
                    .get_mesh(self.mesh_handle.clone())
                    .buffer_layout();
                let pipeline =
                    render::RenderPipelineBuilder::new(shader, self.pipeline_layout.clone())
                        .single_target(render::ColorTargetState::from_current_screen(ctx))
                        .buffers(buffer_layout)
                        .build(ctx);

                render::RenderPassBuilder::new()
                    .color_attachments(&[Some(render::RenderPassColorAttachment::new(screen_view))])
                    .build_run_submit(ctx, |mut render_pass| {
                        render_pass.set_pipeline(&pipeline);

                        mesh_gpu.bind_to_render_pass(&mut render_pass);

                        render_pass.set_bind_group(0, Some(bindgroup.as_ref()), &[]);
                        render_pass.draw_indexed(0..mesh_gpu.index_count.unwrap(), 0, 0..1);
                    });
            }
            Err(err) => log::error!("could not compile shader: {:?}", err),
        }

        false
    }
}
