use gbase::{
    asset,
    render::{self, ArcTextureView, GpuMesh, ShaderBuilder},
    wgpu, Context,
};

use crate::CameraUniform;

pub struct TextureRenderer {
    shader_handle: asset::AssetHandle<ShaderBuilder>,
    shader_depth_handle: asset::AssetHandle<ShaderBuilder>,
    sampler: render::ArcSampler,
    vertices: asset::AssetHandle<render::Mesh>,
    vertices_depth: asset::AssetHandle<render::Mesh>,
}

impl TextureRenderer {
    pub fn new(ctx: &mut Context, cache: &mut gbase::asset::AssetCache) -> Self {
        let shader_handle =
            asset::AssetBuilder::load("../../utils/gbase_utils/assets/shaders/texture.wgsl")
                .watch(cache)
                .build(cache);
        let shader_depth_handle =
            asset::AssetBuilder::load("../../utils/gbase_utils/assets/shaders/texture_depth.wgsl")
                .watch(cache)
                .build(cache);

        let sampler = render::SamplerBuilder::new()
            .min_mag_filter(wgpu::FilterMode::Nearest, wgpu::FilterMode::Nearest)
            .build(ctx);

        let vertices = render::Mesh::new(wgpu::PrimitiveTopology::TriangleList)
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
            );

        let vertices_depth = render::Mesh::new(wgpu::PrimitiveTopology::TriangleList)
            .with_attribute(
                render::VertexAttributeId::Position,
                render::VertexAttributeValues::Float32x3(vec![
                    [0.25, 0.25, 0.0],
                    [1.0, 0.25, 0.0],
                    [1.0, 1.0, 0.0],
                    [0.25, 0.25, 0.0],
                    [1.0, 1.0, 0.0],
                    [0.25, 1.0, 0.0],
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
            );

        Self {
            vertices: asset::AssetBuilder::insert(vertices).build(cache),
            vertices_depth: asset::AssetBuilder::insert(vertices_depth).build(cache),
            shader_handle,
            shader_depth_handle,
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
        if !asset::handle_loaded(cache, self.shader_handle.clone()) {
            return;
        }

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

        let shader = asset::convert_asset(ctx, cache, self.shader_handle.clone(), &()).unwrap();
        let pipeline = render::RenderPipelineBuilder::new(shader, pipeline_layout.clone())
            .single_target(render::ColorTargetState::new().format(out_texture_format))
            .buffers(self.vertices.clone().get(cache).unwrap().buffer_layout())
            .build(ctx);

        let mut encoder = render::EncoderBuilder::new().build_new(ctx);
        render::RenderPassBuilder::new()
            .label("texture renderer")
            .color_attachments(&[Some(
                render::RenderPassColorAttachment::new(out_texture).load(),
            )])
            .build_run(&mut encoder, |mut render_pass| {
                render_pass.set_pipeline(&pipeline);

                let gpu_mesh = self
                    .vertices
                    .clone()
                    .convert::<GpuMesh>(ctx, cache, &())
                    .unwrap();
                render_pass.set_bind_group(0, Some(bindgroup.as_ref()), &[]);
                gpu_mesh.bind_to_render_pass(&mut render_pass);
                gpu_mesh.draw_in_render_pass(&mut render_pass);
            });
        encoder.submit(ctx);
    }

    pub fn render_depth(
        &self,
        ctx: &mut Context,
        cache: &mut gbase::asset::AssetCache,
        in_texture: ArcTextureView,
        out_texture: &wgpu::TextureView,
        out_texture_format: wgpu::TextureFormat,
        camera: &render::UniformBuffer<CameraUniform>,
    ) {
        if !asset::handle_loaded(cache, self.shader_handle.clone()) {
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
                    .sampler_filtering()
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

        let shader =
            asset::convert_asset(ctx, cache, self.shader_depth_handle.clone(), &()).unwrap();
        let pipeline = render::RenderPipelineBuilder::new(shader, pipeline_layout.clone())
            .single_target(render::ColorTargetState::new().format(out_texture_format))
            .buffers(
                self.vertices_depth
                    .clone()
                    .get(cache)
                    .unwrap()
                    .buffer_layout(),
            )
            .build(ctx);

        let mut encoder = render::EncoderBuilder::new().build_new(ctx);
        render::RenderPassBuilder::new()
            .label("texture renderer")
            .color_attachments(&[Some(
                render::RenderPassColorAttachment::new(out_texture).load(),
            )])
            .build_run(&mut encoder, |mut render_pass| {
                render_pass.set_pipeline(&pipeline);
                let gpu_mesh = self
                    .vertices_depth
                    .clone()
                    .convert::<GpuMesh>(ctx, cache, &())
                    .unwrap();
                render_pass.set_bind_group(0, Some(bindgroup.as_ref()), &[]);
                gpu_mesh.bind_to_render_pass(&mut render_pass);
                gpu_mesh.draw_in_render_pass(&mut render_pass);
            });
        encoder.submit(ctx);
    }
}
