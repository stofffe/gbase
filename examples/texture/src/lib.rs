use std::path::Path;

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

        let shader_handle = asset::load_watch(ctx, Path::new("assets/shaders/texture.wgsl"), false);
        let texture_handle = asset::load_watch::<render::Image>(
            ctx,
            Path::new("assets/textures/texture.jpeg"),
            true,
        );
        let texture = asset::get_mut(ctx, texture_handle.clone()).unwrap();
        texture.texture = texture
            .texture
            .clone()
            .format(wgpu::TextureFormat::Rgba8Unorm);

        let mesh = render::MeshBuilder::quad().build().extract_attributes([
            render::VertexAttributeId::Position,
            render::VertexAttributeId::Uv(0),
        ]);
        let mesh_handle = asset::insert(ctx, mesh);

        Self {
            pipeline_layout,
            bindgroup_layout,

            texture_handle,
            shader_handle,
            mesh_handle,
        }
    }
    fn render(&mut self, ctx: &mut Context, screen_view: &wgpu::TextureView) -> bool {
        let mesh =
            asset::convert_asset::<render::GpuMesh>(ctx, self.mesh_handle.clone(), &()).unwrap();
        let shader =
            asset::convert_asset::<wgpu::ShaderModule>(ctx, self.shader_handle.clone(), &())
                .unwrap()
                .clone();
        let texture =
            asset::convert_asset::<GpuImage>(ctx, self.texture_handle.clone(), &()).unwrap();

        let bindgroup = render::BindGroupBuilder::new(self.bindgroup_layout.clone())
            .entries(vec![
                // texture
                render::BindGroupEntry::Texture(texture.view()),
                // sampler
                render::BindGroupEntry::Sampler(texture.sampler()),
            ])
            .build(ctx);

        // TODO: place this on gpumesh instead?
        let buffer_layout = asset::get(ctx, self.mesh_handle.clone())
            .unwrap()
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

        false
    }
}
