use gbase::{
    filesystem,
    render::{self, ArcPipelineLayout, GpuImage, GpuMesh, Image, Mesh, SamplerBuilder},
    wgpu::{self},
    Callbacks, Context,
};
use gbase_utils::{AssetCache, AssetHandle};

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub async fn run() {
    gbase::run::<App>().await;
}

struct App {
    pipeline_layout: ArcPipelineLayout,
    bindgroup_layout: render::ArcBindGroupLayout,

    mesh_cache: AssetCache<Mesh, GpuMesh>,
    mesh_handle: AssetHandle<Mesh>,

    texture_cache: AssetCache<Image, GpuImage>,
    texture_handle: AssetHandle<Image>,

    shader_cache: AssetCache<render::ShaderBuilder, wgpu::ShaderModule>,
    shader_handle: AssetHandle<render::ShaderBuilder>,
}

impl Callbacks for App {
    fn init_ctx() -> gbase::ContextBuilder {
        gbase::ContextBuilder::new().vsync(false)
    }
    fn new(ctx: &mut Context) -> Self {
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

        let mut shader_cache = AssetCache::new();
        let shader_descriptor =
            render::ShaderBuilder::new(filesystem::load_s!("shaders/texture.wgsl").unwrap());
        let shader_handle =
            shader_cache.allocate_reload(shader_descriptor, "assets/shaders/texture.wgsl".into());

        let mut texture_cache = AssetCache::new();
        let image = Image {
            texture: gbase_utils::texture_builder_from_image_bytes(
                &filesystem::load_b!("textures/texture.jpeg").unwrap(),
            )
            .unwrap(),
            sampler: SamplerBuilder::new(),
        };
        let texture_handle =
            texture_cache.allocate_reload(image, "assets/textures/texture.jpeg".into());

        let mut mesh_cache = AssetCache::new();
        let mesh = render::MeshBuilder::quad().build().extract_attributes([
            render::VertexAttributeId::Position,
            render::VertexAttributeId::Uv(0),
        ]);
        let mesh_handle = mesh_cache.allocate(mesh);

        Self {
            pipeline_layout,
            bindgroup_layout,

            mesh_cache,
            mesh_handle,

            shader_handle,
            shader_cache,

            texture_cache,
            texture_handle,
        }
    }
    fn render(&mut self, ctx: &mut Context, screen_view: &wgpu::TextureView) -> bool {
        self.shader_cache.check_watch(ctx);
        self.texture_cache.check_watch(ctx);

        // clear
        render::RenderPassBuilder::new()
            .color_attachments(&[Some(
                render::RenderPassColorAttachment::new(screen_view).clear(wgpu::Color::BLACK),
            )])
            .build_run_submit(ctx, |_| {});

        let mesh = self.mesh_cache.get_gpu(ctx, self.mesh_handle.clone());
        let shader = self.shader_cache.get_gpu(ctx, self.shader_handle.clone());
        let texture = self.texture_cache.get_gpu(ctx, self.texture_handle.clone());
        let bindgroup = render::BindGroupBuilder::new(self.bindgroup_layout.clone())
            .entries(vec![
                // texture
                render::BindGroupEntry::Texture(texture.view()),
                // sampler
                render::BindGroupEntry::Sampler(texture.sampler()),
            ])
            .build(ctx);

        let buffer_layout = self
            .mesh_cache
            .get(self.mesh_handle.clone())
            .buffer_layout();
        let pipeline = render::RenderPipelineBuilder::new(shader, self.pipeline_layout.clone())
            .single_target(render::ColorTargetState::from_current_screen(ctx))
            .buffers(buffer_layout)
            .build(ctx);

        render::RenderPassBuilder::new()
            .color_attachments(&[Some(render::RenderPassColorAttachment::new(screen_view))])
            .build_run_submit(ctx, |mut render_pass| {
                render_pass.set_pipeline(&pipeline);

                mesh.bind_to_render_pass(&mut render_pass);

                render_pass.set_bind_group(0, Some(bindgroup.as_ref()), &[]);
                render_pass.draw_indexed(0..mesh.index_count.unwrap(), 0, 0..1);
            });

        false
    }
}
