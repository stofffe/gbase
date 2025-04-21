use crate::noise::{generate_blue_noise, generate_cloud_noise, generate_weather_map};
use crate::CloudParameters;
use gbase::filesystem;
use gbase::render::{GpuImage, GpuMesh, Image, Mesh};
use gbase::{
    render::{self, ShaderBuilder},
    wgpu, Context,
};
use gbase_utils::{AssetCache, AssetHandle};
use std::collections::BTreeSet;

pub struct CloudRenderer {
    mesh_handle: AssetHandle<Mesh>,
    shader_handle: AssetHandle<ShaderBuilder>,

    pipeline_layout: render::ArcPipelineLayout,
    bindgroup_layout: render::ArcBindGroupLayout,

    noise_texture: render::GpuImage,
    weather_map_texture: AssetHandle<Image>,
    blue_noise_texture: AssetHandle<Image>,
    app_info: gbase_utils::AppInfo, // TODO: global or passed in render?
}

impl CloudRenderer {
    pub fn new(
        ctx: &mut Context,
        shader_cache: &mut AssetCache<ShaderBuilder, wgpu::ShaderModule>,
        image_cache: &mut AssetCache<Image, render::GpuImage>,
        mesh_cache: &mut AssetCache<Mesh, GpuMesh>,
    ) -> Result<Self, wgpu::Error> {
        let noise_texture = generate_cloud_noise(ctx)?;
        let weather_map_texture = generate_weather_map(image_cache);
        let blue_noise_texture = generate_blue_noise(image_cache);

        let app_info = gbase_utils::AppInfo::new(ctx);
        let mesh = render::MeshBuilder::fullscreen_quad()
            .build()
            .extract_attributes(BTreeSet::from([
                render::VertexAttributeId::Position,
                render::VertexAttributeId::Uv(0),
            ]));
        let mesh_handle = mesh_cache.allocate(mesh);

        let shader_handle = shader_cache.allocate_reload(
            render::ShaderBuilder::new(filesystem::load_s!("shaders/clouds.wgsl").unwrap()),
            "assets/shaders/clouds.wgsl".into(),
        );

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
                // Noise sampler
                render::BindGroupLayoutEntry::new()
                    .sampler_filtering()
                    .fragment(),
                // Weather map
                render::BindGroupLayoutEntry::new()
                    .texture_float_filterable()
                    .fragment(),
                // Weather sampler
                render::BindGroupLayoutEntry::new()
                    .sampler_filtering()
                    .fragment(),
                // Blue noise
                render::BindGroupLayoutEntry::new()
                    .texture_float_filterable()
                    .fragment(),
                // Blue sampler
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
            mesh_handle,
            pipeline_layout,
            bindgroup_layout,
            shader_handle,

            noise_texture,
            weather_map_texture,
            blue_noise_texture,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        ctx: &mut Context,
        view: &wgpu::TextureView,
        shader_cache: &mut AssetCache<ShaderBuilder, wgpu::ShaderModule>,
        image_cache: &mut AssetCache<Image, GpuImage>,
        mesh_cache: &mut AssetCache<Mesh, GpuMesh>,
        depth_buffer: &render::DepthBuffer,
        framebuffer: &render::FrameBuffer, // TODO: remove
        camera: &render::UniformBuffer<gbase_utils::CameraUniform>,
        parameters: &render::UniformBuffer<CloudParameters>,
    ) {
        self.app_info.update_buffer(ctx);

        let weather_map = image_cache.get_gpu(ctx, self.weather_map_texture.clone());
        let blue_noise = image_cache.get_gpu(ctx, self.blue_noise_texture.clone());
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
                // Noise sampler
                render::BindGroupEntry::Sampler(self.noise_texture.sampler()),
                // Weather map texture
                render::BindGroupEntry::Texture(weather_map.view()),
                // Weather map sampler
                render::BindGroupEntry::Sampler(weather_map.sampler()),
                // Blue noise texture
                render::BindGroupEntry::Texture(blue_noise.view()),
                // Blue noise sampler
                render::BindGroupEntry::Sampler(blue_noise.sampler()),
            ])
            .build(ctx);

        let mesh = mesh_cache.get(self.mesh_handle.clone());
        let shader = shader_cache.get_gpu(ctx, self.shader_handle.clone());
        let pipeline = render::RenderPipelineBuilder::new(shader, self.pipeline_layout.clone())
            .label("cloud renderer")
            .buffers(mesh.buffer_layout())
            .single_target(framebuffer.target_blend(wgpu::BlendState::ALPHA_BLENDING))
            .depth_stencil(depth_buffer.depth_stencil_state())
            .build(ctx);

        let mesh_gpu = mesh_cache.get_gpu(ctx, self.mesh_handle.clone());
        let mut encoder = render::EncoderBuilder::new().build(ctx);
        render::RenderPassBuilder::new()
            .color_attachments(&[Some(render::RenderPassColorAttachment::new(view))])
            .depth_stencil_attachment(depth_buffer.depth_render_attachment_load())
            .build_run(&mut encoder, |mut render_pass| {
                render_pass.set_pipeline(&pipeline);

                mesh_gpu.bind_to_render_pass(&mut render_pass);

                render_pass.set_bind_group(0, Some(bindgroup.as_ref()), &[]);

                mesh_gpu.draw_in_render_pass(&mut render_pass);
            });

        let queue = render::queue(ctx);
        queue.submit(Some(encoder.finish()));
    }
}

// #[rustfmt::skip]
// const QUAD_VERTICES: &[render::VertexUV] = &[
//     render::VertexUV { position: [-1.0, -1.0, 0.0], uv: [0.0, 1.0] }, // bottom left
//     render::VertexUV { position: [ 1.0,  1.0, 0.0], uv: [1.0, 0.0] }, // top right
//     render::VertexUV { position: [-1.0,  1.0, 0.0], uv: [0.0, 0.0] }, // top left
//
//     render::VertexUV { position: [-1.0, -1.0, 0.0], uv: [0.0, 1.0] }, // bottom left
//     render::VertexUV { position: [ 1.0, -1.0, 0.0], uv: [1.0, 1.0] }, // bottom right
//     render::VertexUV { position: [ 1.0,  1.0, 0.0], uv: [1.0, 0.0] }, // top right
// ];
