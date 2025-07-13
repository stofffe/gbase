use crate::noise::generate_cloud_noise;
use crate::CloudParameters;
use gbase::render::{GpuImage, GpuMesh, Image, Mesh};
use gbase::{asset, tracing};
use gbase::{
    render::{self, ShaderBuilder},
    wgpu, Context,
};
use std::collections::BTreeSet;

pub struct CloudRenderer {
    mesh_handle: asset::AssetHandle<Mesh>,
    shader_handle: asset::AssetHandle<ShaderBuilder>,
    weather_map_handle: asset::AssetHandle<Image>,
    blue_noise_handle: asset::AssetHandle<Image>,

    pipeline_layout: render::ArcPipelineLayout,
    bindgroup_layout: render::ArcBindGroupLayout,

    noise_texture: render::GpuImage,
    app_info: gbase_utils::AppInfo, // TODO: global or passed in render?
}

impl CloudRenderer {
    pub fn new(
        ctx: &mut Context,
        cache: &mut gbase::asset::AssetCache,
    ) -> Result<Self, wgpu::Error> {
        let noise_texture = generate_cloud_noise(ctx)?;
        let weather_map_texture =
            asset::AssetBuilder::load("assets/textures/clouds_weather_map.png")
                .watch(cache)
                // TODO:
                // .on_load(|img: &mut Image| {
                //     img.sampler.set_address_mode(wgpu::AddressMode::Repeat);
                //     img.texture.set_format(wgpu::TextureFormat::Rgba8Unorm);
                // })
                .build(cache);
        let blue_noise_texture = asset::AssetBuilder::load("assets/textures/blue_noise.png")
            .watch(cache)
            // TODO:
            // .on_load(|img: &mut Image| {
            //     img.sampler.set_address_mode(wgpu::AddressMode::Repeat);
            //     img.texture.set_format(wgpu::TextureFormat::Rgba8Unorm);
            // })
            .build(cache);

        let app_info = gbase_utils::AppInfo::new(ctx);
        let mesh = render::MeshBuilder::fullscreen_quad()
            .build()
            .with_extracted_attributes(BTreeSet::from([
                render::VertexAttributeId::Position,
                render::VertexAttributeId::Uv(0),
            ]));
        let mesh_handle = asset::AssetBuilder::insert(mesh).build(cache);

        let shader_handle = asset::AssetBuilder::load("assets/shaders/clouds.wgsl")
            .watch(cache)
            .build(cache);

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
            weather_map_handle: weather_map_texture,
            blue_noise_handle: blue_noise_texture,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        ctx: &mut Context,
        cache: &mut gbase::asset::AssetCache,
        view: &wgpu::TextureView,
        depth_buffer: &render::DepthBuffer,
        framebuffer: &render::FrameBuffer, // TODO: remove
        camera: &render::UniformBuffer<gbase_utils::CameraUniform>,
        parameters: &render::UniformBuffer<CloudParameters>,
    ) {
        if !asset::handle_loaded(cache, self.shader_handle.clone())
            || !asset::handle_loaded(cache, self.mesh_handle.clone())
            || !asset::handle_loaded(cache, self.weather_map_handle.clone())
            || !asset::handle_loaded(cache, self.blue_noise_handle.clone())
        {
            tracing::warn!("all cloud asset not loaded, skipping render");
            return;
        }

        self.app_info.update_buffer(ctx);

        let weather_map =
            asset::convert_asset::<GpuImage>(ctx, cache, self.weather_map_handle.clone(), &())
                .unwrap();
        let blue_noise =
            asset::convert_asset::<GpuImage>(ctx, cache, self.blue_noise_handle.clone(), &())
                .unwrap();
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

        let shader = asset::convert_asset(ctx, cache, self.shader_handle.clone(), &()).unwrap();
        let mesh = asset::get(cache, self.mesh_handle.clone()).unwrap();
        let pipeline = render::RenderPipelineBuilder::new(shader, self.pipeline_layout.clone())
            .label("cloud renderer")
            .buffers(mesh.buffer_layout())
            .single_target(framebuffer.target_blend(wgpu::BlendState::ALPHA_BLENDING))
            .depth_stencil(depth_buffer.depth_stencil_state())
            .build(ctx);

        let mesh_gpu =
            asset::convert_asset::<GpuMesh>(ctx, cache, self.mesh_handle.clone(), &()).unwrap();
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

    pub fn reload_noise(&mut self, ctx: &mut Context) {
        self.noise_texture = generate_cloud_noise(ctx).unwrap();
    }
}
