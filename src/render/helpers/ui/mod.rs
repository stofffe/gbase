mod auto_layout;
mod fonts;
mod shapes;
mod widget;
use std::hash::{DefaultHasher, Hash, Hasher};

use crate::collision::Quad;
use crate::render::{ArcBindGroup, ArcRenderPipeline};
use crate::{filesystem, input, render, time, Context};
pub use fonts::*;
use glam::{vec2, vec3};
use render::VertexTrait;
pub use widget::*;

use super::CameraProjection;

//

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
    w_now: Vec<Widget>,
    widgets_last: Vec<Widget>,

    layout_stack: Vec<usize>,

    hot_this_frame: String,
    hot_last_frame: String,
    active: String,

    // render
    dynamic_vertices: Vec<VertexUI>,
    dynamic_indices: Vec<u32>,
    vertices: render::VertexBuffer<VertexUI>,
    indices: render::IndexBuffer,
    pipeline: ArcRenderPipeline,
    font_atlas: FontAtlas,
    bindgroup: ArcBindGroup,

    camera: render::Camera,
    camera_buffer: render::UniformBuffer<render::CameraUniform>,
}

// TODO: working?
fn create_camera(screen_size: winit::dpi::PhysicalSize<u32>) -> render::Camera {
    render::Camera::new(CameraProjection::orthographic(screen_size.height as f32)).pos(vec3(
        screen_size.width as f32 / 2.0,
        -(screen_size.height as f32 / 2.0),
        1.0,
    ))
}

impl GUIRenderer {
    pub fn new(
        ctx: &mut Context,
        output_format: wgpu::TextureFormat,
        max_quads: usize,
        font_bytes: &[u8],
        supported_chars: &str,
    ) -> Self {
        let max_vertices = max_quads * 4;
        let max_indices = max_quads * 6;

        let dynamic_vertices = Vec::with_capacity(max_vertices);
        let dynamic_indices = Vec::with_capacity(max_indices);

        let vertices = render::VertexBufferBuilder::new(render::VertexBufferSource::Empty(
            max_vertices as u64,
        ))
        .build(ctx);
        let indices =
            render::IndexBufferBuilder::new(render::IndexBufferSource::Empty(max_indices as u64))
                .build(ctx);

        let sampler = render::SamplerBuilder::new().build(ctx);
        let font_atlas = FontAtlas::new(ctx, font_bytes, supported_chars);

        let camera = create_camera(render::surface_size(ctx));
        let camera_buffer = render::UniformBufferBuilder::new(render::UniformBufferSource::Data(
            camera.uniform(ctx),
        ))
        .build(ctx);

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
                // camera
                render::BindGroupLayoutEntry::new().uniform().vertex(),
            ])
            .build(ctx);
        let bindgroup = render::BindGroupBuilder::new(bindgroup_layout.clone())
            .entries(vec![
                // texture atlas
                render::BindGroupEntry::Texture(font_atlas.texture_atlas.texture().view()),
                // sampler
                render::BindGroupEntry::Sampler(sampler),
                // camera
                render::BindGroupEntry::Buffer(camera_buffer.buffer()),
            ])
            .build(ctx);

        let pipeline_layout = render::PipelineLayoutBuilder::new()
            .bind_groups(vec![bindgroup_layout])
            .build(ctx);
        let pipeline = render::RenderPipelineBuilder::new(shader, pipeline_layout)
            .buffers(vec![vertices.desc()])
            .single_target(
                render::ColorTargetState::new()
                    .format(output_format)
                    .blend(wgpu::BlendState::ALPHA_BLENDING),
            )
            .build(ctx);

        Self {
            dynamic_vertices,
            dynamic_indices,
            vertices,
            indices,
            pipeline,
            font_atlas,
            bindgroup,
            camera,
            camera_buffer,

            w_now: vec![widget::root_widget(ctx)],
            widgets_last: vec![],
            layout_stack: vec![widget::root_index()],

            hot_this_frame: String::new(),
            hot_last_frame: String::new(),
            active: String::new(),
        }
    }

    // TODO use existing render pass instead?
    pub fn render(&mut self, ctx: &Context, screen_view: &wgpu::TextureView) {
        // NOTE: widgets_now should be constructed from user calls

        // run auto layout algorithm on widgets this frame
        self.auto_layout(widget::root_index());

        // render widgets after layout
        for widget in self.w_now.clone().iter() {
            widget.inner_render(self);
        }

        //
        // Rendering
        //

        self.vertices.write(ctx, &self.dynamic_vertices);
        self.indices.write(ctx, &self.dynamic_indices);

        let queue = render::queue(ctx);
        let mut encoder = render::EncoderBuilder::new().build(ctx);

        render::RenderPassBuilder::new()
            .color_attachments(&[Some(render::RenderPassColorAttachment::new(screen_view))])
            .build_run(&mut encoder, |mut render_pass| {
                render_pass.set_pipeline(&self.pipeline);
                render_pass.set_vertex_buffer(0, self.vertices.slice(..));
                render_pass.set_index_buffer(self.indices.slice(..), self.indices.format());
                render_pass.set_bind_group(0, Some(self.bindgroup.as_ref()), &[]);
                render_pass.draw_indexed(0..self.indices.len(), 0, 0..1);
            });

        queue.submit(Some(encoder.finish()));

        //
        // Clear for next frame
        //
        if input::mouse_button_released(ctx, input::MouseButton::Left) {
            self.clear_active();
        }
        self.hot_last_frame = self.hot_this_frame.clone();
        self.hot_this_frame = String::new();
        self.widgets_last = self.w_now.clone();
        self.dynamic_vertices.clear();
        self.dynamic_indices.clear();
        self.w_now = vec![widget::root_widget(ctx)];
        self.layout_stack = vec![widget::root_index()];
    }

    pub fn resize(&mut self, ctx: &Context, new_size: winit::dpi::PhysicalSize<u32>) {
        self.camera = create_camera(new_size);
        self.camera_buffer.write(ctx, &self.camera.uniform(ctx));
    }

    // widgets
    fn create_widget(&mut self, widget: Widget) -> usize {
        let index = self.w_now.len();

        // add to parent's children
        self.get_widget_mut(widget.parent).children.push(index);

        // add to render tree
        self.w_now.push(widget);

        index
    }

    pub(crate) fn push_layout(&mut self, index: usize) {
        self.layout_stack.push(index);
    }
    pub(crate) fn pop_layout(&mut self) {
        self.layout_stack
            .pop()
            .expect("trying to pop layout when empty");
    }
    pub(crate) fn get_layout(&mut self) -> usize {
        *self
            .layout_stack
            .last()
            .expect("trying to get layout when empty")
    }

    pub(crate) fn get_widget_last_frame(&self, id: &str) -> Option<Widget> {
        self.widgets_last.iter().find(|w| w.label == id).cloned()
    }

    #[inline]
    pub(crate) fn get_widget(&self, index: usize) -> &Widget {
        &self.w_now[index]
    }
    #[inline]
    pub(crate) fn get_widget_mut(&mut self, index: usize) -> &mut Widget {
        &mut self.w_now[index]
    }
    #[inline]
    fn get_widget_parent(&self, index: usize) -> &Widget {
        &self.w_now[self.w_now[index].parent]
    }

    fn children_size(&self, index: usize, axis: usize) -> f32 {
        let children = &self.w_now[index].children;
        if children.is_empty() {
            return 0.0;
        }

        let mut children_sum = 0.0;
        for &child_i in children {
            let child = &self.w_now[child_i];
            children_sum += child.computed_size[axis];
        }

        let gap_sum = (children.len() - 1) as f32 * self.w_now[index].gap;

        children_sum + gap_sum
    }

    fn children_max(&self, index: usize, axis: usize) -> f32 {
        let children = &self.w_now[index].children;

        let mut children_max = 0.0f32;
        for &child_i in children {
            let child = &self.w_now[child_i];
            children_max = children_max.max(child.computed_size[axis]);
        }

        children_max
    }
}

// active/hot

impl GUIRenderer {
    // active
    fn set_active(&mut self, id: String) {
        self.active = id;
    }
    fn clear_active(&mut self) {
        self.active = String::new();
    }
    pub fn check_active(&self, id: &str) -> bool {
        self.active == id
    }

    // hot
    fn set_hot_this_frame(&mut self, id: String) {
        self.hot_this_frame = id;
    }
    pub fn check_hot(&self, id: &str) -> bool {
        self.hot_last_frame == id && self.hot_this_frame == id
    }
    pub fn check_last_hot(&self, id: &str) -> bool {
        self.hot_last_frame == id
    }
    fn debug(&mut self, ctx: &Context) {
        //
        // debug
        //
        let font_size = 100.0;
        let font_color = WHITE;

        self.text(
            &format!("fps: {}", time::fps(ctx)),
            Quad::new(vec2(0.0, 0.0), vec2(0.5, font_size)),
            font_size,
            font_color,
            false,
        );
        self.text(
            &format!("hot: {}", self.hot_this_frame),
            Quad::new(vec2(0.0, font_size), vec2(0.5, font_size)),
            font_size,
            font_color,
            false,
        );
        self.text(
            &format!("hot last: {}", self.hot_last_frame),
            Quad::new(vec2(0.0, font_size * 2.0), vec2(0.5, font_size)),
            font_size,
            font_color,
            false,
        );
        self.text(
            &format!("active: {}", self.active),
            Quad::new(vec2(0.0, font_size * 3.0), vec2(0.5, font_size)),
            font_size,
            font_color,
            false,
        );
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
