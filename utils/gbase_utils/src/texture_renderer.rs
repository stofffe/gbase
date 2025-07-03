use gbase::{
    asset,
    render::{self, ArcTextureView, ShaderBuilder, VertexUV},
    wgpu, Context,
};

use crate::CameraUniform;

pub struct TextureRenderer {
    shader_handle: asset::AssetHandle<ShaderBuilder>,
    shader_depth_handle: asset::AssetHandle<ShaderBuilder>,
    sampler: render::ArcSampler,
    vertices: render::VertexBuffer<VertexUV>,
    indices: render::IndexBuffer,
    vertices_depth: render::VertexBuffer<VertexUV>,
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

        let vertices = render::VertexBufferBuilder::new(render::VertexBufferSource::Data(
            CENTERED_QUAD_VERTICES.to_vec(),
        ))
        .build(ctx);
        let indices = render::IndexBufferBuilder::new(render::IndexBufferSource::Data(
            CENTERED_QUAD_INDICES.to_vec(),
        ))
        .build(ctx);

        let vertices_depth = render::VertexBufferBuilder::new(render::VertexBufferSource::Data(
            CENTERED_QUAD_VERTICES_DEPTH.to_vec(),
        ))
        .build(ctx);

        Self {
            vertices,
            indices,
            vertices_depth,
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
            .buffers(vec![self.vertices.desc()])
            .build(ctx);

        render::RenderPassBuilder::new()
            .label("texture renderer")
            .color_attachments(&[Some(
                render::RenderPassColorAttachment::new(out_texture).load(),
            )])
            .build_run_submit(ctx, |mut render_pass| {
                render_pass.set_pipeline(&pipeline);
                render_pass.set_vertex_buffer(0, self.vertices.slice(..));
                render_pass.set_index_buffer(self.indices.slice(..), self.indices.format());
                render_pass.set_bind_group(0, Some(bindgroup.as_ref()), &[]);
                render_pass.draw_indexed(0..self.indices.len(), 0, 0..1);
            });
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
            .buffers(vec![self.vertices_depth.desc()])
            .build(ctx);

        render::RenderPassBuilder::new()
            .label("texture renderer")
            .color_attachments(&[Some(
                render::RenderPassColorAttachment::new(out_texture).load(),
            )])
            .build_run_submit(ctx, |mut render_pass| {
                render_pass.set_pipeline(&pipeline);
                render_pass.set_vertex_buffer(0, self.vertices_depth.slice(..));
                render_pass.set_index_buffer(self.indices.slice(..), self.indices.format());
                render_pass.set_bind_group(0, Some(bindgroup.as_ref()), &[]);
                render_pass.draw_indexed(0..self.indices.len(), 0, 0..1);
            });
    }
}

#[rustfmt::skip]
const CENTERED_QUAD_VERTICES: &[render::VertexUV] = &[
    render::VertexUV { position: [-1.0, -1.0, 0.0], uv: [0.0, 1.0] }, // bottom left
    render::VertexUV { position: [ 1.0, -1.0, 0.0], uv: [1.0, 1.0] }, // bottom right
    render::VertexUV { position: [ 1.0,  1.0, 0.0], uv: [1.0, 0.0] }, // top right

    render::VertexUV { position: [-1.0, -1.0, 0.0], uv: [0.0, 1.0] }, // bottom left
    render::VertexUV { position: [ 1.0,  1.0, 0.0], uv: [1.0, 0.0] }, // top right
    render::VertexUV { position: [-1.0,  1.0, 0.0], uv: [0.0, 0.0] }, // top left
];

#[rustfmt::skip]
const CENTERED_QUAD_VERTICES_DEPTH: &[render::VertexUV] = &[
    render::VertexUV { position: [0.25, 0.25, 0.0], uv: [0.0, 1.0] }, // bottom left
    render::VertexUV { position: [1.0,  0.25, 0.0], uv: [1.0, 1.0] }, // bottom right
    render::VertexUV { position: [1.0,  1.0,  0.0], uv: [1.0, 0.0] }, // top right

    render::VertexUV { position: [0.25, 0.25, 0.0], uv: [0.0, 1.0] }, // bottom left
    render::VertexUV { position: [1.0,  1.0,  0.0], uv: [1.0, 0.0] }, // top right
    render::VertexUV { position: [0.25, 1.0,  0.0], uv: [0.0, 0.0] }, // top left
];

#[rustfmt::skip]
const CENTERED_QUAD_INDICES: &[u32] = &[
    0, 1, 2,
    3, 4, 5
];
