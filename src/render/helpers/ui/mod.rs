mod button;
mod elements;
mod fonts;

pub use button::*;
// pub use elements::*;
pub use fonts::*;

use crate::render::{ArcBindGroup, ArcRenderPipeline};
use crate::{filesystem, render, Context};
use render::VertexTrait;
use std::hash::{DefaultHasher, Hash, Hasher};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UiID {
    id: u64,
}

impl UiID {
    pub const fn cleared() -> Self {
        let id = 0;
        Self { id }
    }
    pub fn new(label: &str) -> Self {
        let mut hasher = DefaultHasher::new();
        label.hash(&mut hasher);
        let id = hasher.finish();
        Self { id }
    }
    pub fn new_child(parent: u64, label: &str) -> Self {
        let mut hasher = DefaultHasher::new();
        label.hash(&mut hasher);
        parent.hash(&mut hasher);
        let id = hasher.finish();
        Self { id }
    }
}

pub struct GUIRenderer {
    // logic
    hot: UiID,    // hover
    active: UiID, // holding click

    // render
    dynamic_vertices: Vec<VertexUI>,
    dynamic_indices: Vec<u32>,
    vertices: render::VertexBuffer<VertexUI>,
    indices: render::IndexBuffer,
    pipeline: ArcRenderPipeline,
    font_atlas: FontAtlas,
    font_atlas_bindgroup: ArcBindGroup,
}

impl GUIRenderer {
    pub async fn new(
        ctx: &mut Context,
        output_format: wgpu::TextureFormat,
        vertices_batch_size: u32,
        indices_batch_size: u32,
        font_bytes: &[u8],
        supported_chars: &str,
    ) -> Self {
        let dynamic_vertices = Vec::with_capacity(vertices_batch_size as usize);
        let dynamic_indices = Vec::with_capacity(indices_batch_size as usize);
        let vertices = render::VertexBufferBuilder::new(render::VertexBufferSource::Empty(
            vertices_batch_size as u64,
        ))
        .build(ctx);
        let indices = render::IndexBufferBuilder::new(render::IndexBufferSource::Empty(
            indices_batch_size as u64,
        ))
        .build(ctx);

        let sampler = render::SamplerBuilder::new().build(ctx);
        let font_atlas = FontAtlas::new(ctx, font_bytes, supported_chars);

        let shader_str = filesystem::load_s!("shaders/ui.wgsl").unwrap();
        let shader = render::ShaderBuilder::new(shader_str).build(ctx);

        let bindgroup_layout = render::BindGroupLayoutBuilder::new()
            .entries(vec![
                // texture atlas
                render::BindGroupLayoutEntry::new()
                    .texture_float_filterable()
                    .fragment(),
                // sampler
                render::BindGroupLayoutEntry::new()
                    .sampler_filtering()
                    .fragment(),
            ])
            .build(ctx);
        let bindgroup = render::BindGroupBuilder::new(bindgroup_layout.clone())
            .entries(vec![
                // texture atlas
                render::BindGroupEntry::Texture(font_atlas.texture_atlas.texture().view()),
                // sampler
                render::BindGroupEntry::Sampler(sampler),
            ])
            .build(ctx);

        let pipeline_layout = render::PipelineLayoutBuilder::new()
            .bind_groups(vec![bindgroup_layout])
            .build(ctx);
        let pipeline = render::RenderPipelineBuilder::new(shader, pipeline_layout)
            .buffers(vec![vertices.desc()])
            .targets(vec![Some(wgpu::ColorTargetState {
                format: output_format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })])
            .build(ctx);

        Self {
            dynamic_vertices,
            dynamic_indices,
            vertices,
            indices,
            pipeline,
            font_atlas,
            font_atlas_bindgroup: bindgroup,
            hot: UiID::cleared(),
            active: UiID::cleared(),
        }
    }

    // TODO use existing render pass instead?
    pub fn render(&mut self, ctx: &Context, screen_view: &wgpu::TextureView) {
        // TODO: have logic here?
        //
        // if input::mouse_button_released(ctx, MouseButton::Left) {
        //     self.clear_active();
        // }

        // Update buffers with current frames data

        self.vertices.write(ctx, &self.dynamic_vertices);
        self.indices.write(ctx, &self.dynamic_indices);

        // Render batch
        let queue = render::queue(ctx);
        let mut encoder = render::create_encoder(ctx, None);

        render::RenderPassBuilder::new()
            .color_attachments(&[Some(wgpu::RenderPassColorAttachment {
                view: screen_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })])
            .build_run(&mut encoder, |mut render_pass| {
                render_pass.set_pipeline(&self.pipeline);
                render_pass.set_vertex_buffer(0, self.vertices.slice(..));
                render_pass.set_index_buffer(self.indices.slice(..), self.indices.format());
                render_pass.set_bind_group(0, Some(self.font_atlas_bindgroup.as_ref()), &[]);
                render_pass.draw_indexed(0..self.indices.len(), 0, 0..1);
            });

        queue.submit(Some(encoder.finish()));

        // Clear for next frame
        self.dynamic_vertices.clear();
        self.dynamic_indices.clear();
    }

    fn set_active(&mut self, id: UiID) {
        self.active = id;
    }
    fn clear_active(&mut self) {
        self.active = UiID::cleared();
    }
    fn check_active(&self, id: UiID) -> bool {
        self.active == id
    }
    fn set_hot(&mut self, id: UiID) {
        self.hot = id;
    }
    fn clear_hot(&mut self) {
        self.hot = UiID::cleared();
    }
    fn check_hot(&self, id: UiID) -> bool {
        self.hot == id
    }
}

const VERTEX_TYPE_SHAPE: u32 = 0;
const VERTEX_TYPE_TEXT: u32 = 1;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct VertexUI {
    pub position: [f32; 3],
    pub ty: u32, // 0 shape, 1 text
    pub color: [f32; 4],
    pub uv: [f32; 2],
}

impl VertexUI {
    const ATTRIBUTES: &'static [wgpu::VertexAttribute] = &wgpu::vertex_attr_array![
        0=>Float32x3,   // pos
        1=>Uint32,      // ty
        2=>Float32x4,   // color
        3=>Float32x2,   // uv
    ];
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: Self::ATTRIBUTES,
        }
    }
}

impl VertexTrait for VertexUI {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        Self::desc()
    }
}
