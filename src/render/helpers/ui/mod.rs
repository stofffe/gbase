mod fonts;
mod shapes;
mod widget;
use std::hash::{DefaultHasher, Hash, Hasher};

use crate::collision::Quad;
use crate::render::{ArcBindGroup, ArcRenderPipeline};
use crate::{filesystem, input, render, time, Context};
pub use fonts::*;
use glam::{vec2, vec3, Vec2};
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

fn create_camera(screen_size: winit::dpi::PhysicalSize<u32>) -> render::Camera {
    render::Camera::new(CameraProjection::orthographic(
        screen_size.width as f32,
        screen_size.height as f32,
    ))
    .pos(vec3(
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
            bindgroup,
            camera,
            camera_buffer,

            w_now: vec![widget::root_widget(ctx)],
            widgets_last: vec![],
            hot_this_frame: String::new(),
            hot_last_frame: String::new(),
            active: String::new(),
        }
    }

    // TODO use existing render pass instead?
    pub fn render(&mut self, ctx: &Context, screen_view: &wgpu::TextureView) {
        // NOTE: widgets_now should be constructed from user calls

        // layout
        self.auto_layout(widget::root_index());
        // self.auto_layout(ctx);

        // render
        for widget in self.w_now.clone().iter().skip(1) {
            widget.inner_render(self);
        }

        self.debug(ctx);

        // clear state
        if input::mouse_button_released(ctx, input::MouseButton::Left) {
            self.clear_active();
        }
        self.hot_last_frame = self.hot_this_frame.clone();
        self.hot_this_frame = String::new();
        self.widgets_last = self.w_now.clone();

        //
        // Rendering
        //

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
                render_pass.set_bind_group(0, Some(self.bindgroup.as_ref()), &[]);
                render_pass.draw_indexed(0..self.indices.len(), 0, 0..1);
            });

        queue.submit(Some(encoder.finish()));

        // Clear for next frame
        self.dynamic_vertices.clear();
        self.dynamic_indices.clear();
        self.w_now = vec![widget::root_widget(ctx)];
    }

    pub fn resize(&mut self, ctx: &Context, new_size: winit::dpi::PhysicalSize<u32>) {
        self.camera = create_camera(new_size);
        self.camera_buffer.write(ctx, &self.camera.uniform(ctx));
    }

    // widgets
    fn create_widget(&mut self, widget: Widget) -> usize {
        let index = self.w_now.len();

        // add to parent's children
        self.get_widget(widget.parent).children.push(index);

        // add to render tree
        self.w_now.push(widget);

        index
    }

    #[inline]
    pub(crate) fn get_widget(&mut self, index: usize) -> &mut Widget {
        &mut self.w_now[index]
    }
    #[inline]
    fn get_widget_parent(&self, index: usize) -> &Widget {
        &self.w_now[self.w_now[index].parent]
    }
}

// logic

impl GUIRenderer {
    // PRE
    // Pixels
    fn auto_layout_1(&mut self, index: usize) {
        let parent = self.get_widget_parent(index);
        let main_axis = parent.direction.main_axis();
        let cross_axis = parent.direction.cross_axis();

        let this = self.get_widget(index);

        if index != widget::root_index() {
            if let SizeKind::Pixels(px) = this.size_main {
                this.computed_size[main_axis] = px;
            }
            if let SizeKind::Pixels(px) = this.size_cross {
                this.computed_size[cross_axis] = px;
            }
        }

        // children
        for i in 0..self.w_now[index].children.len() {
            self.auto_layout_1(self.w_now[index].children[i]);
        }
    }

    // PRE
    // Percent
    fn auto_layout_2(&mut self, index: usize) {
        let parent = self.get_widget_parent(index);
        let parent_dir = parent.direction;
        let parent_inner_size = parent.computed_inner_size();
        let main_axis = parent_dir.main_axis();
        let cross_axis = parent_dir.cross_axis();

        let this = self.get_widget(index);

        if index != widget::root_index() {
            if let SizeKind::PercentOfParent(p) = this.size_main {
                this.computed_size[main_axis] = parent_inner_size[main_axis] * p;
            }
            if let SizeKind::PercentOfParent(p) = this.size_cross {
                this.computed_size[cross_axis] = parent_inner_size[cross_axis] * p;
            }
        }

        // children
        for i in 0..this.children.len() {
            self.auto_layout_2(self.w_now[index].children[i]);
        }
    }

    // PRE
    // Grow
    fn auto_layout_3(&mut self, index: usize) {
        let parent = self.get_widget_parent(index);
        let parent_inner_size = parent.computed_inner_size();
        let gap = parent.gap;
        let neighbour_count = parent.children.len();
        let main_axis = parent.direction.main_axis();
        let cross_axis = parent.direction.cross_axis();

        let this = self.get_widget(index);
        let size_main = this.size_main;
        let size_cross = this.size_cross;
        // TODO marging padding

        if index != widget::root_index() {
            if let SizeKind::Grow = size_main {
                let mut neighbours_space = 0.0;
                for i in 0..neighbour_count {
                    let neighbout_i = self.get_widget_parent(index).children[i];
                    let neighbour_size = self.get_widget(neighbout_i).computed_size[main_axis];
                    neighbours_space += neighbour_size;
                }

                let gap_space = (neighbour_count - 1) as f32 * gap;

                let total_space_taken = neighbours_space + gap_space;

                let space_left = parent_inner_size[main_axis] - total_space_taken;
                self.get_widget(index).computed_size[main_axis] = space_left;
            }

            if let SizeKind::Grow = size_cross {
                self.get_widget(index).computed_size[cross_axis] = parent_inner_size[cross_axis];
            }
        }

        // children
        for i in 0..self.w_now[index].children.len() {
            self.auto_layout_3(self.w_now[index].children[i]);
        }
    }

    // PRE
    fn auto_layout_4(&mut self, index: usize) {
        // let parent_index = self.w_now[index].parent;

        // SOLVE VIOLATIONS

        // children
        for i in 0..self.w_now[index].children.len() {
            self.auto_layout_4(self.w_now[index].children[i]);
        }
    }

    // PRE
    // Relative pos
    fn auto_layout_5(&mut self, index: usize) {
        let this = self.get_widget(index);
        let children_count = this.children.len();
        let inner_pos = this.computed_inner_pos();
        let main_axis = this.direction.main_axis();

        if index != widget::root_index() {
            let mut offset = 0.0;

            // main axis
            for i in 0..children_count {
                let child_index = self.get_widget(index).children[i];

                self.get_widget(child_index).computed_pos = inner_pos;
                self.get_widget(child_index).computed_pos[main_axis] += offset;

                offset += self.get_widget(child_index).computed_size[main_axis];
                offset += self.get_widget(index).gap;
            }
        }

        // children
        for i in 0..self.w_now[index].children.len() {
            self.auto_layout_5(self.w_now[index].children[i]);
        }
    }

    // Algorithm
    // https://www.rfleury.com/p/ui-part-1-the-interaction-medium?s=w
    // https://www.rfleury.com/p/ui-part-2-build-it-every-frame-immediate
    // https://www.rfleury.com/p/ui-part-3-the-widget-building-language
    fn auto_layout(&mut self, index: usize) {
        // 1. Fixed sizes (PRE/POST)
        self.auto_layout_1(index);
        // 2. Parent dependent sizes (PRE)
        self.auto_layout_2(index);
        // 3. Grow dependent sizes (POST)
        self.auto_layout_3(index);
        // 4. Solve violations (PRE)
        self.auto_layout_4(index);
        // 5. Parent dependent sizes (PRE)
        self.auto_layout_5(index);
        // dbg!(&self.w_now);
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
        self.text(
            &format!("fps: {}", time::fps(ctx)),
            Quad::new(vec2(0.0, 0.0), vec2(0.5, 0.05)),
            0.05,
            BLACK,
            false,
        );
        self.text(
            &format!("hot: {}", self.hot_this_frame),
            Quad::new(vec2(0.0, 0.05), vec2(0.5, 0.05)),
            0.05,
            BLACK,
            false,
        );
        self.text(
            &format!("hot last: {}", self.hot_last_frame),
            Quad::new(vec2(0.0, 0.1), vec2(0.5, 0.05)),
            0.05,
            BLACK,
            false,
        );
        self.text(
            &format!("active: {}", self.active),
            Quad::new(vec2(0.0, 0.15), vec2(0.5, 0.05)),
            0.05,
            BLACK,
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
