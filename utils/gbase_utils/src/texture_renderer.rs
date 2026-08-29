use gbase::{
    asset::{
        self, AssetCache, MeshGpuConverter, MeshGpuConverterSettings, NamedInserter,
        ShaderGpuConverter, ShaderGpuConverterSettings,
    },
    render::{self, ArcShaderModule, ArcTextureView, GpuMesh, Mesh, Shader},
    wgpu, Context,
};

use crate::CameraUniform;

pub struct TextureRenderer {
    shader_gpu_handle: asset::AssetHandle<ArcShaderModule>,
    shader_depth_gpu_handle: asset::AssetHandle<ArcShaderModule>,
    fullscreen_mesh_handle: asset::AssetHandle<Mesh>,
    fullscreen_mesh_gpu_handle: asset::AssetHandle<GpuMesh>,
    sampler: render::ArcSampler,
}

impl TextureRenderer {
    pub fn all_assets_just_loaded(&self, cache: &mut AssetCache) -> bool {
        // at least one just loaded
        let one_just_loaded = cache.handle_just_loaded(&self.shader_gpu_handle)
            || cache.handle_just_loaded(&self.shader_depth_gpu_handle)
            || cache.handle_just_loaded(&self.fullscreen_mesh_handle)
            || cache.handle_just_loaded(&self.fullscreen_mesh_gpu_handle);

        if !one_just_loaded {
            return false;
        }

        // all loaded
        cache.handle_successfully_loaded(&self.shader_gpu_handle)
            && cache.handle_successfully_loaded(&self.shader_depth_gpu_handle)
            && cache.handle_successfully_loaded(&self.fullscreen_mesh_handle)
            && cache.handle_successfully_loaded(&self.fullscreen_mesh_gpu_handle)
    }

    pub fn new(ctx: &mut Context, cache: &mut AssetCache) -> Self {
        // let shader_handle = cache.load_asset::<ShaderLoader>(&ShaderLoaderSettings::new(
        //     "../../utils/gbase_utils/assets/shaders/texture.wgsl",
        // ));
        // let shader_depth_handle = cache.load_asset::<ShaderLoader>(&ShaderLoaderSettings::new(
        //     "../../utils/gbase_utils/assets/shaders/texture_depth.wgsl",
        // ));

        let shader_handle = cache.insert_asset::<Shader, NamedInserter>(
            "texture renderer shader",
            render::Shader::new(include_str!("../assets/shaders/texture.wgsl")),
        );

        let shader_depth_handle = cache.insert_asset::<Shader, NamedInserter>(
            "texture renderer depth shader",
            render::Shader::new(include_str!("../assets/shaders/texture_depth.wgsl")),
        );

        let shader_gpu_handle = cache.convert_asset::<ShaderGpuConverter>(
            &ShaderGpuConverterSettings::new(shader_handle.clone()),
        );
        let shader_depth_gpu_handle = cache.convert_asset::<ShaderGpuConverter>(
            &ShaderGpuConverterSettings::new(shader_depth_handle.clone()),
        );

        let sampler = render::SamplerBuilder::new()
            .mip_map_filer(wgpu::FilterMode::Nearest)
            .min_mag_filter(wgpu::FilterMode::Nearest, wgpu::FilterMode::Nearest)
            .build(ctx);

        let fullscreen_mesh_handle = cache.insert_asset::<_, NamedInserter>(
            "texture renderer fullscreen mesh",
            render::Mesh::new(wgpu::PrimitiveTopology::TriangleList)
                .with_attribute(
                    render::VertexAttributeId::Position,
                    render::VertexAttributeValues::Float32x3(vec![
                        [-1.0, -1.0, 0.0],
                        [1.0, -1.0, 0.0],
                        [1.0, 1.0, 0.0],
                        [-1.0, -1.0, 0.0],
                        [1.0, 1.0, 0.0],
                        [-1.0, 1.0, 0.0],
                    ]),
                )
                .with_attribute(
                    render::VertexAttributeId::Uv(0),
                    render::VertexAttributeValues::Float32x2(vec![
                        [0.0, 1.0],
                        [1.0, 1.0],
                        [1.0, 0.0],
                        [0.0, 1.0],
                        [1.0, 0.0],
                        [0.0, 0.0],
                    ]),
                ),
        );

        let fullscreen_mesh_gpu_handle = cache.convert_asset::<MeshGpuConverter>(
            &MeshGpuConverterSettings::new(fullscreen_mesh_handle.clone()),
        );

        Self {
            shader_gpu_handle,
            shader_depth_gpu_handle,
            fullscreen_mesh_handle,
            fullscreen_mesh_gpu_handle,
            sampler,
        }
    }

    pub fn render(
        &self,
        ctx: &mut Context,
        cache: &mut gbase::asset::AssetCache,
        in_texture: ArcTextureView,
        out_texture: &wgpu::TextureView,
        out_texture_format: wgpu::TextureFormat,
    ) {
        let bindgroup_layout = render::BindGroupLayoutBuilder::new()
            .entries(vec![
                // texture
                render::BindGroupLayoutEntry::new()
                    .texture_float_filterable()
                    .fragment(),
                // sampler
                render::BindGroupLayoutEntry::new()
                    .sampler_filtering()
                    .fragment(),
            ])
            .build(ctx);

        let pipeline_layout = render::PipelineLayoutBuilder::new()
            .bind_groups(vec![bindgroup_layout.clone()])
            .build(ctx);
        let bindgroup = render::BindGroupBuilder::new(bindgroup_layout.clone())
            .entries(vec![
                // texture
                render::BindGroupEntry::Texture(in_texture),
                // sampler
                render::BindGroupEntry::Sampler(self.sampler.clone()),
            ])
            .build(ctx);

        let Ok(shader) = cache.get_asset(&self.shader_gpu_handle).cloned() else {
            return;
        };
        let Ok(mesh) = cache.get_asset(&self.fullscreen_mesh_handle) else {
            return;
        };

        let pipeline = render::RenderPipelineBuilder::new(shader, pipeline_layout.clone())
            .single_target(render::ColorTargetState::new().format(out_texture_format))
            .buffers(mesh.buffer_layout())
            .build(ctx);

        let Ok(mesh_gpu) = cache.get_asset(&self.fullscreen_mesh_gpu_handle) else {
            return;
        };

        let mut encoder = render::EncoderBuilder::new().build_new(ctx);
        render::RenderPassBuilder::new()
            .label("texture renderer")
            .color_attachments(&[Some(
                render::RenderPassColorAttachment::new(out_texture).load(),
            )])
            .build_run(ctx, &mut encoder, |_ctx, mut render_pass| {
                render_pass.set_pipeline(&pipeline);

                render_pass.set_bind_group(0, Some(bindgroup.as_ref()), &[]);
                mesh_gpu.bind_to_render_pass(&mut render_pass);
                mesh_gpu.draw_in_render_pass(&mut render_pass);
            });
        encoder.submit(ctx);
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render_depth(
        &self,
        ctx: &mut Context,
        cache: &mut gbase::asset::AssetCache,
        in_texture: ArcTextureView,
        out_texture: &wgpu::TextureView,
        out_texture_format: wgpu::TextureFormat,
        camera: &render::UniformBuffer<CameraUniform>,
        viewport: Option<ViewPort>,
    ) {
        if !cache.handle_successfully_loaded(&self.shader_depth_gpu_handle) {
            return;
        }

        let bindgroup_layout = render::BindGroupLayoutBuilder::new()
            .entries(vec![
                // texture
                render::BindGroupLayoutEntry::new()
                    .texture_depth()
                    .fragment(),
                // sampler
                render::BindGroupLayoutEntry::new()
                    .sampler_nonfiltering()
                    .fragment(),
                // camera
                render::BindGroupLayoutEntry::new().uniform().fragment(),
            ])
            .build(ctx);

        let pipeline_layout = render::PipelineLayoutBuilder::new()
            .bind_groups(vec![bindgroup_layout.clone()])
            .build(ctx);
        let bindgroup = render::BindGroupBuilder::new(bindgroup_layout.clone())
            .entries(vec![
                // texture
                render::BindGroupEntry::Texture(in_texture),
                // sampler
                render::BindGroupEntry::Sampler(self.sampler.clone()),
                // camera
                render::BindGroupEntry::Buffer(camera.buffer()),
            ])
            .build(ctx);

        let Ok(shader) = cache.get_asset(&self.shader_depth_gpu_handle).cloned() else {
            return;
        };
        let Ok(mesh) = cache.get_asset(&self.fullscreen_mesh_handle) else {
            return;
        };

        let pipeline = render::RenderPipelineBuilder::new(shader, pipeline_layout.clone())
            .single_target(render::ColorTargetState::new().format(out_texture_format))
            .buffers(mesh.buffer_layout())
            .build(ctx);

        let Ok(mesh_gpu) = cache.get_asset(&self.fullscreen_mesh_gpu_handle) else {
            return;
        };

        let mut encoder = render::EncoderBuilder::new().build_new(ctx);
        render::RenderPassBuilder::new()
            .label("texture renderer")
            .color_attachments(&[Some(
                render::RenderPassColorAttachment::new(out_texture).load(),
            )])
            .build_run(ctx, &mut encoder, |_ctx, mut render_pass| {
                if let Some(viewport) = viewport {
                    render_pass
                        .set_viewport(viewport.x, viewport.y, viewport.w, viewport.h, 0.0, 1.0);
                }

                render_pass.set_pipeline(&pipeline);
                render_pass.set_bind_group(0, Some(bindgroup.as_ref()), &[]);

                mesh_gpu.bind_to_render_pass(&mut render_pass);
                mesh_gpu.draw_in_render_pass(&mut render_pass);
            });
        encoder.submit(ctx);
    }
}

pub struct ViewPort {
    x: f32,
    y: f32,
    h: f32,
    w: f32,
}

impl ViewPort {
    pub fn new_pixels(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, h, w }
    }
    pub fn new_ndc(ctx: &Context, x: f32, y: f32, w: f32, h: f32) -> Self {
        let screen_size = render::surface_size(ctx);
        Self {
            x: x * screen_size.width as f32,
            y: y * screen_size.height as f32,
            w: w * screen_size.width as f32,
            h: h * screen_size.height as f32,
        }
    }
}
