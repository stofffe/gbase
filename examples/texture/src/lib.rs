use std::{fs, io::Read};

use gbase::{
    bytemuck::bytes_of,
    filesystem, input,
    render::{self, ArcBindGroup, ArcRenderPipeline, VertexBufferBuilder, VertexBufferSource},
    wgpu, Callbacks, Context,
};
use gbase_utils::{
    image::{self, GenericImageView},
    AssetHandle, Assets, Image,
};

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub async fn run() {
    gbase::run::<App>().await;
}

struct App {
    vertex_buffer: render::VertexBuffer<render::VertexUV>,
    pipeline: ArcRenderPipeline,

    assets: Assets,
    texture_handle: AssetHandle<Image>,
    bindgroup_layout: render::ArcBindGroupLayout,
}

impl Callbacks for App {
    fn new(ctx: &mut Context) -> Self {
        let mut assets = Assets::new();
        let vertex_buffer =
            VertexBufferBuilder::new(VertexBufferSource::Data(QUAD_VERTICES.to_vec())).build(ctx);

        let texture = gbase_utils::texture_builder_from_image_bytes(
            &filesystem::load_b!("textures/texture.jpeg").unwrap(),
        )
        .unwrap();
        let sampler = render::SamplerBuilder::new();
        let image = Image { texture, sampler };
        let texture_handle = assets.allocate_image_data(image);

        assets.watch_image("assets/textures/city.jpg".into(), texture_handle.clone());

        let shader_str = filesystem::load_s!("shaders/texture.wgsl").unwrap();
        let shader = render::ShaderBuilder::new(shader_str).build(ctx);

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
        let pipeline = render::RenderPipelineBuilder::new(shader, pipeline_layout)
            .single_target(render::ColorTargetState::from_current_screen(ctx))
            .buffers(vec![vertex_buffer.desc()])
            .build(ctx);

        Self {
            vertex_buffer,
            pipeline,
            bindgroup_layout,
            assets,
            texture_handle,
        }
    }
    fn render(&mut self, ctx: &mut Context, screen_view: &wgpu::TextureView) -> bool {
        if input::key_just_pressed(ctx, input::KeyCode::F1) {
            let image = self.assets.get_image_mut(self.texture_handle.clone());
            image.texture.format = wgpu::TextureFormat::Rgba8UnormSrgb;
        }

        self.assets.check_watch_images();

        let texture = self.assets.get_image_gpu(ctx, self.texture_handle.clone());
        let bindgroup = render::BindGroupBuilder::new(self.bindgroup_layout.clone())
            .entries(vec![
                // texture
                render::BindGroupEntry::Texture(texture.view()),
                // sampler
                render::BindGroupEntry::Sampler(texture.sampler()),
            ])
            .build(ctx);

        let mut encoder = render::EncoderBuilder::new().build(ctx);
        let queue = render::queue(ctx);
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("render pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: screen_view,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLUE),
                    store: wgpu::StoreOp::Store,
                },
                resolve_target: None,
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_bind_group(0, Some(bindgroup.as_ref()), &[]);
        render_pass.draw(0..self.vertex_buffer.len(), 0..1);

        drop(render_pass);
        queue.submit(Some(encoder.finish()));

        false
    }
}

#[rustfmt::skip]
const QUAD_VERTICES: &[render::VertexUV] = &[
    render::VertexUV { position: [-0.5, -0.5, 0.0], uv: [0.0, 1.0] }, // bottom left
    render::VertexUV { position: [ 0.5,  0.5, 0.0], uv: [1.0, 0.0] }, // top right
    render::VertexUV { position: [-0.5,  0.5, 0.0], uv: [0.0, 0.0] }, // top left

    render::VertexUV { position: [-0.5, -0.5, 0.0], uv: [0.0, 1.0] }, // bottom left
    render::VertexUV { position: [ 0.5, -0.5, 0.0], uv: [1.0, 1.0] }, // bottom right
    render::VertexUV { position: [ 0.5,  0.5, 0.0], uv: [1.0, 0.0] }, // top right
];
