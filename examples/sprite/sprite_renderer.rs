use crate::sprite_atlas::{self, AtlasSprite};
use gbase::glam::Vec4;
use gbase::{
    collision::Quad,
    filesystem,
    render::{self, CameraUniform, VertexTrait},
    wgpu, Context,
};
use glam::{vec2, Vec2};

pub struct SpriteRenderer {
    vertices: Vec<VertexSprite>,
    indices: Vec<u32>,

    vertex_buffer: render::VertexBuffer<VertexSprite>,
    index_buffer: render::IndexBuffer,

    bindgroup_layout: render::ArcBindGroupLayout,
    pipeline: render::ArcRenderPipeline,
    // TODO: pass from user code?
    sampler: render::ArcSampler,
}

impl SpriteRenderer {
    pub fn new(ctx: &mut Context, max_sprites: u64, output_format: wgpu::TextureFormat) -> Self {
        let vertices = Vec::new();
        let indices = Vec::new();

        let vertex_buffer =
            render::VertexBufferBuilder::new(render::VertexBufferSource::Empty(max_sprites * 4))
                .build(ctx);
        let index_buffer =
            render::IndexBufferBuilder::new(render::IndexBufferSource::Empty(max_sprites * 6))
                .build(ctx);

        let shader = render::ShaderBuilder::new(
            filesystem::load_s!("shaders/sprite_renderer.wgsl")
                .expect("could not load sprite renderer shader"),
        )
        .build(ctx);

        let bindgroup_layout = render::BindGroupLayoutBuilder::new()
            .entries(vec![
                // camera
                render::BindGroupLayoutEntry::new().uniform().vertex(),
                // atlas
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
        let pipeline = render::RenderPipelineBuilder::new(shader, pipeline_layout)
            .buffers(vec![vertex_buffer.desc()])
            .single_target(
                render::ColorTargetState::new()
                    .format(output_format)
                    .blend(wgpu::BlendState::ALPHA_BLENDING),
            )
            .build(ctx);

        let sampler = render::SamplerBuilder::new().build(ctx);

        Self {
            vertices,
            indices,
            vertex_buffer,
            index_buffer,
            bindgroup_layout,
            pipeline,
            sampler,
        }
    }

    pub fn render(
        &mut self,
        ctx: &mut Context,
        output_view: &wgpu::TextureView,
        camera: &render::UniformBuffer<CameraUniform>,
        atlas: &render::TextureWithView,
    ) {
        // update buffers
        self.vertex_buffer.write(ctx, &self.vertices);
        self.index_buffer.write(ctx, &self.indices);

        // create bindgroup
        let mut encoder = render::EncoderBuilder::new().build(ctx);
        let bindgroup = render::BindGroupBuilder::new(self.bindgroup_layout.clone())
            .entries(vec![
                // camera
                render::BindGroupEntry::Buffer(camera.buffer()),
                // atlas
                render::BindGroupEntry::Texture(atlas.view()),
                // sampler
                render::BindGroupEntry::Sampler(self.sampler.clone()),
            ])
            .build(ctx);

        render::RenderPassBuilder::new()
            .color_attachments(&[Some(
                render::RenderPassColorAttachment::new(output_view).clear(wgpu::Color::BLACK),
            )])
            .build_run(&mut encoder, |mut pass| {
                pass.set_pipeline(&self.pipeline);
                pass.set_bind_group(0, bindgroup.as_ref(), &[]);
                pass.set_vertex_buffer(0, self.vertex_buffer.slice(..)); // TODO: use len of vertices?
                pass.set_index_buffer(self.index_buffer.slice(..), self.index_buffer.format());

                pass.draw_indexed(0..self.indices.len() as u32, 0, 0..1);
            });

        render::queue(ctx).submit(Some(encoder.finish()));

        self.vertices.clear();
        self.indices.clear();
    }

    #[rustfmt::skip]
    pub fn draw_quad(&mut self, quad: Quad, color: Vec4) {
        let (x, y) = (quad.pos.x ,quad.pos.y);
        let (sx, sy) = (quad.size.x, quad.size.y);
        let color = color.to_array();

        let offset = self.vertices.len() as u32; // save before pushing verts
        self.vertices.push(VertexSprite { position: [-0.5 + x,      0.5 - y,      0.0], uv: [0.0, 0.0], color, uses_texture: 0.0 }); // tl
        self.vertices.push(VertexSprite { position: [-0.5 + x + sx, 0.5 - y,      0.0], uv: [0.0, 0.0], color, uses_texture: 0.0 }); // tr
        self.vertices.push(VertexSprite { position: [-0.5 + x,      0.5 - y - sy, 0.0], uv: [0.0, 0.0], color, uses_texture: 0.0 }); // bl
        self.vertices.push(VertexSprite { position: [-0.5 + x + sx, 0.5 - y - sy, 0.0], uv: [0.0, 0.0], color, uses_texture: 0.0 }); // br
        self.indices.push(offset);     // tl
        self.indices.push(offset + 1); // bl
        self.indices.push(offset + 2); // tr
        self.indices.push(offset + 2); // tr
        self.indices.push(offset + 1); // bl
        self.indices.push(offset + 3); // br
    }

    pub fn draw_sprite(&mut self, quad: Quad, uv: Quad) {
        self.draw_sprite_with_tint(quad, uv, Vec4::ONE);
    }

    #[rustfmt::skip]
    pub fn draw_sprite_with_tint(&mut self, quad: Quad, uv: Quad, tint: Vec4) {
        let (x, y) = (quad.pos.x ,quad.pos.y);
        let (sx, sy) = (quad.size.x, quad.size.y);
        let (ux, uy) = (uv.pos.x, uv.pos.y);
        let (uw, uh) = (uv.size.x, uv.size.y);
        let color = tint.to_array();

        let offset = self.vertices.len() as u32; // save before pushing verts
        self.vertices.push(VertexSprite { position: [-0.5 + x,      -0.5 + y + sy,      0.0], uv: [ux,      uy     ], color, uses_texture: 1.0 }); // tl
        self.vertices.push(VertexSprite { position: [-0.5 + x + sx, -0.5 + y + sy,      0.0], uv: [ux + uw, uy     ], color, uses_texture: 1.0 }); // tr
        self.vertices.push(VertexSprite { position: [-0.5 + x,      -0.5 + y, 0.0], uv: [ux,      uy + uh], color, uses_texture: 1.0 }); // bl
        self.vertices.push(VertexSprite { position: [-0.5 + x + sx, -0.5 + y, 0.0], uv: [ux + uw, uy + uh], color, uses_texture: 1.0 }); // br
        self.indices.push(offset);     // tl
        self.indices.push(offset + 1); // bl
        self.indices.push(offset + 2); // tr
        self.indices.push(offset + 2); // tr
        self.indices.push(offset + 1); // bl
        self.indices.push(offset + 3); // br
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct VertexSprite {
    pub position: [f32; 3],
    pub color: [f32; 4],
    pub uv: [f32; 2],
    pub uses_texture: f32,
}

impl VertexSprite {
    const ATTRIBUTES: &'static [wgpu::VertexAttribute] = &wgpu::vertex_attr_array![
        0=>Float32x3,  // pos
        1=>Float32x4,  // color
        2=>Float32x2,  // uv
        3=>Float32,    // uses texture
    ];
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: Self::ATTRIBUTES,
        }
    }
}

impl VertexTrait for VertexSprite {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        Self::desc()
    }
}

impl AtlasSprite {
    pub fn size(&self) -> Vec2 {
        vec2(self.w as f32, self.h as f32)
    }
    pub fn uv(&self) -> Quad {
        Quad::new(
            vec2(
                self.x as f32 / sprite_atlas::ATLAS_WIDTH as f32,
                self.y as f32 / sprite_atlas::ATLAS_HEIGHT as f32,
            ),
            vec2(
                self.w as f32 / sprite_atlas::ATLAS_WIDTH as f32,
                self.h as f32 / sprite_atlas::ATLAS_HEIGHT as f32,
            ),
        )
    }
}
