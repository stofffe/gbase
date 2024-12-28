use crate::noise::generate_noise;
use gbase::render::SamplerBuilder;
use gbase::wgpu;
use gbase::{
    collision, filesystem,
    render::{self, CameraUniform},
    Context,
};

pub struct CloudRenderer {
    vertices: render::VertexBuffer<render::VertexUV>,
    pipeline: render::ArcRenderPipeline,
    bindgroup: render::ArcBindGroup,

    noise_texture: render::Texture,
    app_info: render::AppInfo,
}

impl CloudRenderer {
    pub fn new(
        ctx: &mut Context,
        framebuffer: &render::FrameBuffer,
        depth_buffer: &render::DepthBuffer,
        camera: &render::UniformBuffer<CameraUniform>,
        bounding_box: &render::UniformBuffer<collision::Box3D>,
    ) -> Self {
        let noise_texture = generate_noise(ctx);
        let app_info = render::AppInfo::new(ctx);
        let noise_sampler = SamplerBuilder::new()
            .min_mag_filter(wgpu::FilterMode::Linear, wgpu::FilterMode::Linear)
            .build(ctx);
        let vertices = render::VertexBufferBuilder::new(render::VertexBufferSource::Data(
            QUAD_VERTICES.to_vec(),
        ))
        .build(ctx);
        let shader_str = filesystem::load_s!("shaders/clouds.wgsl").unwrap();
        let shader = render::ShaderBuilder::new(shader_str).build(ctx);
        let bindgroup_layout = render::BindGroupLayoutBuilder::new()
            .entries(vec![
                // App info
                render::BindGroupLayoutEntry::new()
                    .uniform()
                    .vertex()
                    .fragment(),
                // Camera
                render::BindGroupLayoutEntry::new().uniform().vertex(),
                // Cloud BB
                render::BindGroupLayoutEntry::new()
                    .uniform()
                    .vertex()
                    .fragment(),
                // Noise texture
                render::BindGroupLayoutEntry::new()
                    .texture_float_filterable()
                    .fragment(),
                // Noise sampler
                render::BindGroupLayoutEntry::new()
                    .sampler_filtering()
                    .fragment(),
            ])
            .build(ctx);
        let bindgroup = render::BindGroupBuilder::new(bindgroup_layout.clone())
            .entries(vec![
                // App info
                render::BindGroupEntry::Buffer(app_info.buffer()),
                // Camera
                render::BindGroupEntry::Buffer(camera.buffer()),
                // Cloud BB
                render::BindGroupEntry::Buffer(bounding_box.buffer()),
                // Noise texture
                render::BindGroupEntry::Texture(noise_texture.view()),
                // Noise sampler
                render::BindGroupEntry::Sampler(noise_sampler.clone()),
            ])
            .build(ctx);
        let pipeline_layout = render::PipelineLayoutBuilder::new()
            .bind_groups(vec![bindgroup_layout])
            .build(ctx);
        let pipeline = render::RenderPipelineBuilder::new(shader, pipeline_layout)
            .buffers(vec![vertices.desc()])
            .targets(vec![Some(framebuffer.target())])
            .depth_stencil(depth_buffer.depth_stencil_state())
            .build(ctx);

        Self {
            app_info,
            vertices,
            pipeline,
            bindgroup,
            noise_texture,
        }
    }

    pub fn render(
        &mut self,
        ctx: &mut Context,
        view: &wgpu::TextureView,
        depth_buffer: &render::DepthBuffer,
    ) {
        self.app_info.update_buffer(ctx);

        let mut encoder = render::EncoderBuilder::new().build(ctx);
        render::RenderPassBuilder::new()
            .color_attachments(&[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })])
            .depth_stencil_attachment(depth_buffer.depth_render_attachment_load())
            .build_run(&mut encoder, |mut rp| {
                rp.set_pipeline(&self.pipeline);
                rp.set_vertex_buffer(0, self.vertices.slice(..));
                rp.set_bind_group(0, Some(self.bindgroup.as_ref()), &[]);
                rp.draw(0..self.vertices.len(), 0..1);
            });

        let queue = render::queue(ctx);
        queue.submit(Some(encoder.finish()));
    }
}

#[rustfmt::skip]
const QUAD_VERTICES: &[render::VertexUV] = &[
    render::VertexUV { position: [-1.0, -1.0, 0.0], uv: [0.0, 1.0] }, // bottom left
    render::VertexUV { position: [ 1.0,  1.0, 0.0], uv: [1.0, 0.0] }, // top right
    render::VertexUV { position: [-1.0,  1.0, 0.0], uv: [0.0, 0.0] }, // top left

    render::VertexUV { position: [-1.0, -1.0, 0.0], uv: [0.0, 1.0] }, // bottom left
    render::VertexUV { position: [ 1.0, -1.0, 0.0], uv: [1.0, 1.0] }, // bottom right
    render::VertexUV { position: [ 1.0,  1.0, 0.0], uv: [1.0, 0.0] }, // top right
];
