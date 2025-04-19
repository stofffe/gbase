mod auto_layout;
mod fonts;
mod shapes;
mod widget;

pub use fonts::*;
pub use widget::*;

use crate::{app_info, camera, AppInfoUniform, CameraProjection};
use gbase::{
    glam::{vec2, vec3},
    input,
    render::{self, ArcBindGroup, ArcPipelineLayout, ArcRenderPipeline, ArcShaderModule, VertexUV},
    time,
    wgpu::{self},
    winit, Context,
};
use std::mem;

pub struct GUIRenderer {
    // logic
    widgets: Vec<Widget>,
    widgets_cache: Vec<Widget>,

    layout_stack: Vec<usize>,

    hot_this_frame: String,
    hot_last_frame: String,
    active: String,

    // render
    vertices: render::VertexBuffer<VertexUV>,
    indices: render::IndexBuffer,
    instances: Vec<WidgetInstance>,
    instance_buffer: render::RawBuffer<WidgetInstance>,

    shader: ArcShaderModule,
    pipeline_layout: ArcPipelineLayout,
    bindgroup: ArcBindGroup,
    font_atlas: FontAtlas,

    camera: camera::Camera,
    camera_buffer: render::UniformBuffer<camera::CameraUniform>,

    app_info_buffer: render::UniformBuffer<app_info::AppInfoUniform>,
}

impl GUIRenderer {
    pub fn new(
        ctx: &mut Context,
        max_quads: usize,
        font_bytes: &[u8],
        supported_chars: &str,
    ) -> Self {
        let vertices =
            render::VertexBufferBuilder::new(render::VertexBufferSource::Data(VERTICES.to_vec()))
                .build(ctx);
        let indices =
            render::IndexBufferBuilder::new(render::IndexBufferSource::Data(INDICES.to_vec()))
                .build(ctx);
        let instances = Vec::new();
        let instance_buffer = render::RawBufferBuilder::new(render::RawBufferSource::Size(
            (max_quads * mem::size_of::<WidgetInstance>()) as u64,
        ))
        .build(ctx);

        let sampler = render::SamplerBuilder::new().build(ctx);
        let font_atlas = FontAtlas::new(ctx, font_bytes, supported_chars);

        let camera = create_camera(render::surface_size(ctx));
        let camera_buffer = render::UniformBufferBuilder::new(render::UniformBufferSource::Data(
            camera.uniform(ctx),
        ))
        .build(ctx);

        let app_info_buffer =
            render::UniformBufferBuilder::new(render::UniformBufferSource::Empty).build(ctx);

        let shader =
            render::ShaderBuilder::new(include_str!("../../assets/shaders/ui.wgsl")).build(ctx);

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
                // app info
                render::BindGroupLayoutEntry::new().uniform().fragment(),
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
                // app info
                render::BindGroupEntry::Buffer(app_info_buffer.buffer()),
            ])
            .build(ctx);

        let pipeline_layout = render::PipelineLayoutBuilder::new()
            .bind_groups(vec![bindgroup_layout])
            .build(ctx);

        Self {
            vertices,
            indices,
            instances,
            instance_buffer,
            shader,
            pipeline_layout,
            font_atlas,
            bindgroup,
            camera,
            camera_buffer,

            widgets: vec![widget::root_widget(ctx)],
            widgets_cache: vec![],
            layout_stack: vec![widget::root_index()],

            hot_this_frame: String::new(),
            hot_last_frame: String::new(),
            active: String::new(),

            app_info_buffer,
        }
    }

    // TODO use existing render pass instead?
    pub fn render(
        &mut self,
        ctx: &mut Context,
        screen_view: &wgpu::TextureView,
        view_format: wgpu::TextureFormat,
    ) {
        //
        // Layout widgets
        //
        self.auto_layout(widget::root_index());

        for widget in self.widgets.clone().iter() {
            if let Some(color) = widget.color {
                self.quad(
                    widget.computed_pos_margin(),
                    widget.computed_size_margin(),
                    color,
                    widget.border_radius,
                );
            }

            if !widget.text.is_empty() {
                self.text(
                    &widget.text,
                    widget.computed_pos,
                    widget.computed_size,
                    widget.font_size,
                    widget.text_color,
                    widget.text_wrap,
                );
            }
        }

        self.debug(ctx);

        //
        // Rendering
        //
        let pipeline =
            render::RenderPipelineBuilder::new(self.shader.clone(), self.pipeline_layout.clone())
                .buffers(vec![self.vertices.desc(), WidgetInstance::desc()])
                .single_target(
                    render::ColorTargetState::new()
                        .format(view_format)
                        .blend(wgpu::BlendState::ALPHA_BLENDING),
                )
                .build(ctx);

        self.instance_buffer.write(ctx, &self.instances);
        self.app_info_buffer.write(ctx, &AppInfoUniform::new(ctx));
        render::RenderPassBuilder::new()
            .color_attachments(&[Some(render::RenderPassColorAttachment::new(screen_view))])
            .build_run_submit(ctx, |mut render_pass| {
                render_pass.set_pipeline(&pipeline);
                render_pass.set_vertex_buffer(0, self.vertices.slice(..));
                render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
                render_pass.set_index_buffer(self.indices.slice(..), self.indices.format());
                render_pass.set_bind_group(0, Some(self.bindgroup.as_ref()), &[]);
                render_pass.draw_indexed(0..self.indices.len(), 0, 0..self.instances.len() as u32);
            });

        //
        // Clear for next frame
        //
        if input::mouse_button_released(ctx, input::MouseButton::Left) {
            self.clear_active();
        }
        self.hot_last_frame = self.hot_this_frame.clone();
        self.hot_this_frame = String::new();
        self.widgets_cache = self.widgets.clone();
        self.widgets = vec![widget::root_widget(ctx)];
        self.layout_stack = vec![widget::root_index()];

        self.instances.clear();
    }

    pub fn resize(&mut self, ctx: &Context, new_size: winit::dpi::PhysicalSize<u32>) {
        self.camera = create_camera(new_size);
        self.camera_buffer.write(ctx, &self.camera.uniform(ctx));
    }

    /// Insert a widget into the widget tree
    fn insert_widget(&mut self, mut widget: Widget) -> usize {
        let index = self.widgets.len();

        // widget -> parent pointer
        widget.parent = self.get_layout();

        // parent -> widget pointer
        self.get_widget_mut(widget.parent).children.push(index);

        // add to render tree
        self.widgets.push(widget);

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
    pub(crate) fn get_layout(&self) -> usize {
        *self
            .layout_stack
            .last()
            .expect("trying to get layout when empty")
    }

    pub(crate) fn get_widget_cached(&self, id: &str) -> Option<Widget> {
        self.widgets_cache.iter().find(|w| w.label == id).cloned()
    }

    #[inline]
    pub(crate) fn get_widget(&self, index: usize) -> &Widget {
        &self.widgets[index]
    }
    #[inline]
    pub(crate) fn get_widget_mut(&mut self, index: usize) -> &mut Widget {
        &mut self.widgets[index]
    }
    #[inline]
    fn get_widget_parent(&self, index: usize) -> &Widget {
        &self.widgets[self.widgets[index].parent]
    }

    /// Size of children
    ///
    /// Includes gap
    fn get_children_size(&self, index: usize, axis: usize) -> f32 {
        let children = &self.widgets[index].children;
        if children.is_empty() {
            return 0.0;
        }

        let mut children_sum = 0.0;
        for &child_i in children {
            let child = &self.widgets[child_i];
            children_sum += child.computed_size[axis];
        }

        let gap_sum = (children.len() - 1) as f32 * self.widgets[index].gap;

        children_sum + gap_sum
    }

    fn get_children_max(&self, index: usize, axis: usize) -> f32 {
        let children = &self.widgets[index].children;

        let mut children_max = 0.0f32;
        for &child_i in children {
            let child = &self.widgets[child_i];
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
            vec2(0.0, 0.0),
            vec2(0.5, font_size),
            font_size,
            font_color,
            false,
        );
        self.text(
            &format!("hot: {}", self.hot_this_frame),
            vec2(0.0, font_size),
            vec2(0.5, font_size),
            font_size,
            font_color,
            false,
        );
        self.text(
            &format!("hot last: {}", self.hot_last_frame),
            vec2(0.0, font_size * 2.0),
            vec2(0.5, font_size),
            font_size,
            font_color,
            false,
        );
        self.text(
            &format!("active: {}", self.active),
            vec2(0.0, font_size * 3.0),
            vec2(0.5, font_size),
            font_size,
            font_color,
            false,
        );
    }
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct WidgetInstance {
    position: [f32; 2], // uv coordinate system, (0,0) top left and y+ is down
    scale: [f32; 2],
    atlas_offset: [f32; 2],
    atlas_scale: [f32; 2],
    color: [f32; 4],
    ty: u32,
    border_radius: [f32; 4],
}

impl WidgetInstance {
    pub fn desc() -> render::VertexBufferLayout {
        render::VertexBufferLayout::from_vertex_formats(
            wgpu::VertexStepMode::Instance,
            vec![
                wgpu::VertexFormat::Float32x2, // pos
                wgpu::VertexFormat::Float32x2, // scale
                wgpu::VertexFormat::Float32x2, // atlas offset
                wgpu::VertexFormat::Float32x2, // atlas scale
                wgpu::VertexFormat::Float32x4, // color
                wgpu::VertexFormat::Uint32,    // ty
                wgpu::VertexFormat::Float32x4, // border radius
            ],
        )
    }
}

fn create_camera(screen_size: winit::dpi::PhysicalSize<u32>) -> crate::Camera {
    crate::Camera::new(CameraProjection::orthographic(screen_size.height as f32)).pos(vec3(
        screen_size.width as f32 / 2.0,
        -(screen_size.height as f32 / 2.0),
        1.0,
    ))
}

#[rustfmt::skip]
const VERTICES: &[render::VertexUV] = &[
    render::VertexUV { position: [0.0,  0.0, 0.0], uv: [0.0, 0.0] }, // top left
    render::VertexUV { position: [0.0, -1.0, 0.0], uv: [0.0, 1.0] }, // bottom left
    render::VertexUV { position: [1.0,  0.0, 0.0], uv: [1.0, 0.0] }, // top right
    render::VertexUV { position: [1.0, -1.0, 0.0], uv: [1.0, 1.0] }, // bottom right
];

#[rustfmt::skip]
const INDICES: &[u32] = &[
    0, 1, 2,
    2, 1, 3
];

// use std::hash::{DefaultHasher, Hash, Hasher};
// #[derive(Clone, Copy, Debug, Eq, PartialEq)]
// pub struct UiID {
//     id: u64,
// }
//
// impl UiID {
//     pub const fn cleared() -> Self {
//         let id = 0;
//         Self { id }
//     }
//     pub fn new(label: &str) -> Self {
//         let mut hasher = DefaultHasher::new();
//         label.hash(&mut hasher);
//         let id = hasher.finish();
//         Self { id }
//     }
//     pub(crate) fn new_child(parent: u64, label: &str) -> Self {
//         let mut hasher = DefaultHasher::new();
//         label.hash(&mut hasher);
//         parent.hash(&mut hasher);
//         let id = hasher.finish();
//         Self { id }
//     }
// }
