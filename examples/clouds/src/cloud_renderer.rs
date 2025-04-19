use crate::noise::{generate_blue_noise, generate_cloud_noise, generate_weather_map};
use crate::CloudParameters;
use gbase::render::SamplerBuilder;
use gbase::wgpu;
use gbase::{filesystem, render, Context};

pub struct CloudRenderer {
    vertices: render::VertexBuffer<render::VertexUV>,
    shader: render::ArcShaderModule,
    pipeline_layout: render::ArcPipelineLayout,
    bindgroup_layout: render::ArcBindGroupLayout,

    noise_texture: render::TextureWithView,
    weather_map_texture: render::TextureWithView,
    blue_noise_texture: render::TextureWithView,
    noise_sampler: render::ArcSampler,
    app_info: gbase_utils::AppInfo, // TODO: global or passed in render?
}

impl CloudRenderer {
    pub fn new(ctx: &mut Context) -> Result<Self, wgpu::Error> {
        let noise_texture = generate_cloud_noise(ctx)?;
        let weather_map_texture = generate_weather_map(ctx);
        let blue_noise_texture = generate_blue_noise(ctx);

        let app_info = gbase_utils::AppInfo::new(ctx);
        let noise_sampler = SamplerBuilder::new()
            .min_mag_filter(wgpu::FilterMode::Linear, wgpu::FilterMode::Linear)
            .build(ctx);
        let vertices = render::VertexBufferBuilder::new(render::VertexBufferSource::Data(
            QUAD_VERTICES.to_vec(),
        ))
        .build(ctx);

        let shader =
            render::ShaderBuilder::new(filesystem::load_s!("shaders/clouds.wgsl").unwrap());

        #[cfg(not(target_arch = "wasm32"))]
        let shader = shader.build_err(ctx)?;

        #[cfg(target_arch = "wasm32")]
        let shader = shader.build(ctx);

        let bindgroup_layout = render::BindGroupLayoutBuilder::new()
            .entries(vec![
                // App info
                render::BindGroupLayoutEntry::new()
                    .uniform()
                    .vertex()
                    .fragment(),
                // Camera
                render::BindGroupLayoutEntry::new()
                    .uniform()
                    .vertex()
                    .fragment(),
                // Parameters
                render::BindGroupLayoutEntry::new().uniform().fragment(),
                // Noise texture
                render::BindGroupLayoutEntry::new()
                    .ty(wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D3,
                        multisampled: false,
                    })
                    .fragment(),
                // Weather map
                render::BindGroupLayoutEntry::new()
                    .texture_float_filterable()
                    .fragment(),
                // Blue noise
                render::BindGroupLayoutEntry::new()
                    .texture_float_filterable()
                    .fragment(),
                // Noise sampler
                render::BindGroupLayoutEntry::new()
                    .sampler_filtering()
                    .fragment(),
            ])
            .build(ctx);
        let pipeline_layout = render::PipelineLayoutBuilder::new()
            .bind_groups(vec![bindgroup_layout.clone()])
            .build(ctx);

        Ok(Self {
            app_info,
            vertices,
            pipeline_layout,
            bindgroup_layout,
            shader,

            noise_texture,
            weather_map_texture,
            blue_noise_texture,
            noise_sampler,
        })
    }

    pub fn render(
        &mut self,
        ctx: &mut Context,
        view: &wgpu::TextureView,
        depth_buffer: &render::DepthBuffer,
        framebuffer: &render::FrameBuffer, // TODO: remove
        camera: &render::UniformBuffer<gbase_utils::CameraUniform>,
        parameters: &render::UniformBuffer<CloudParameters>,
    ) {
        self.app_info.update_buffer(ctx);

        let bindgroup = render::BindGroupBuilder::new(self.bindgroup_layout.clone())
            .entries(vec![
                // App info
                render::BindGroupEntry::Buffer(self.app_info.buffer()),
                // Camera
                render::BindGroupEntry::Buffer(camera.buffer()),
                // Parameters
                render::BindGroupEntry::Buffer(parameters.buffer()),
                // Noise texture
                render::BindGroupEntry::Texture(self.noise_texture.view()),
                // Weather map
                render::BindGroupEntry::Texture(self.weather_map_texture.view()),
                // Blue noise
                render::BindGroupEntry::Texture(self.blue_noise_texture.view()),
                // Noise sampler
                render::BindGroupEntry::Sampler(self.noise_sampler.clone()),
            ])
            .build(ctx);
        let pipeline =
            render::RenderPipelineBuilder::new(self.shader.clone(), self.pipeline_layout.clone())
                .buffers(vec![self.vertices.desc()])
                .single_target(framebuffer.target_blend(wgpu::BlendState::ALPHA_BLENDING))
                .depth_stencil(depth_buffer.depth_stencil_state())
                .build(ctx);

        let mut encoder = render::EncoderBuilder::new().build(ctx);
        render::RenderPassBuilder::new()
            .color_attachments(&[Some(render::RenderPassColorAttachment::new(view))])
            .depth_stencil_attachment(depth_buffer.depth_render_attachment_load())
            .build_run(&mut encoder, |mut rp| {
                rp.set_pipeline(&pipeline);
                rp.set_vertex_buffer(0, self.vertices.slice(..));
                rp.set_bind_group(0, Some(bindgroup.as_ref()), &[]);
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
