use crate::VertexAttributeId;

use super::CameraUniform;
use gbase::{
    glam::{vec4, Vec3, Vec4Swizzles},
    render::{
        self, ArcBindGroup, ArcRenderPipeline, RenderPipelineBuilder, ShaderBuilder, VertexColor,
    },
    wgpu, Context,
};
use std::{collections::BTreeMap, f32::consts::PI};

pub struct GizmoRenderer {
    dynamic_vertex_buffer: Vec<VertexColor>,
    dynamic_index_buffer: Vec<u32>,

    vertex_buffer: render::VertexBuffer<VertexColor>,
    index_buffer: render::IndexBuffer,
    bindgroup: ArcBindGroup,
    pipeline: ArcRenderPipeline,

    depth_buffer: render::DepthBuffer,

    resolution: u32,
}

const GIZMO_MAX_VERTICES: usize = 10000;
const GIZMO_MAX_INDICES: usize = 10000;
const GIZMO_RESOLUTION: u32 = 16;
impl GizmoRenderer {
    pub fn new(
        ctx: &mut Context,
        output_format: wgpu::TextureFormat,
        camera_buffer: &render::UniformBuffer<CameraUniform>,
    ) -> Self {
        let dynamic_vertex_buffer = Vec::with_capacity(GIZMO_MAX_VERTICES);
        let dynamic_index_buffer = Vec::with_capacity(GIZMO_MAX_INDICES);
        let vertex_buffer = render::VertexBufferBuilder::new(render::VertexBufferSource::Size(
            GIZMO_MAX_VERTICES as u64,
        ))
        .build(ctx);
        let index_buffer = render::IndexBufferBuilder::new(render::IndexBufferSource::Empty(
            GIZMO_MAX_INDICES as u64,
        ))
        .build(ctx);

        let bindgroup_layout = render::BindGroupLayoutBuilder::new()
            .entries(vec![
                // camera
                render::BindGroupLayoutEntry::new().vertex().uniform(),
            ])
            .build(ctx);
        let bindgroup = render::BindGroupBuilder::new(bindgroup_layout.clone())
            .entries(vec![
                // camera
                render::BindGroupEntry::Buffer(camera_buffer.buffer()),
            ])
            .build(ctx);

        let depth_buffer = render::DepthBufferBuilder::new()
            .screen_size(ctx)
            .build(ctx);

        let shader = ShaderBuilder::new(include_str!("../assets/shaders/gizmo.wgsl")).build(ctx);
        let pipeline_layout = render::PipelineLayoutBuilder::new()
            .bind_groups(vec![bindgroup_layout])
            .build(ctx);
        let pipeline = RenderPipelineBuilder::new(shader, pipeline_layout)
            .buffers(vec![vertex_buffer.desc()])
            .single_target(render::ColorTargetState::new().format(output_format))
            .depth_stencil(depth_buffer.depth_stencil_state())
            .topology(wgpu::PrimitiveTopology::LineList)
            .build(ctx);

        Self {
            dynamic_vertex_buffer,
            dynamic_index_buffer,
            vertex_buffer,
            index_buffer,
            pipeline,
            depth_buffer,
            bindgroup,
            resolution: GIZMO_RESOLUTION,
        }
    }
    pub fn render(&mut self, ctx: &Context, view: &wgpu::TextureView) {
        self.vertex_buffer.write(ctx, &self.dynamic_vertex_buffer);
        self.index_buffer.write(ctx, &self.dynamic_index_buffer);

        render::RenderPassBuilder::new()
            .color_attachments(&[Some(render::RenderPassColorAttachment::new(view))])
            .depth_stencil_attachment(self.depth_buffer.depth_render_attachment_clear())
            .build_run_submit(ctx, |mut pass| {
                pass.set_pipeline(&self.pipeline);
                pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                pass.set_index_buffer(self.index_buffer.slice(..), self.index_buffer.format());
                pass.set_bind_group(0, Some(self.bindgroup.as_ref()), &[]);
                pass.draw_indexed(0..self.index_buffer.len(), 0, 0..1);
            });

        self.dynamic_vertex_buffer.clear();
        self.dynamic_index_buffer.clear();
    }

    pub fn resize(&mut self, ctx: &Context, width: u32, height: u32) {
        self.depth_buffer.resize(ctx, width, height);
    }

    // TODO: builder instead?
    pub fn set_resolution(&mut self, resolution: u32) {
        self.resolution = resolution;
    }
}

impl GizmoRenderer {
    /// Draw line
    pub fn draw_line(&mut self, from: Vec3, to: Vec3, color: Vec3) {
        let vertex_start = self.dynamic_vertex_buffer.len();
        self.dynamic_vertex_buffer.push(VertexColor {
            position: from.to_array(),
            color: color.to_array(),
        });
        self.dynamic_vertex_buffer.push(VertexColor {
            position: to.to_array(),
            color: color.to_array(),
        });
        self.dynamic_index_buffer.push(vertex_start as u32);
        self.dynamic_index_buffer.push(vertex_start as u32 + 1);
    }

    /// Draw quad with side 1
    pub fn draw_quad(&mut self, transform: &crate::Transform3D, color: Vec3) {
        let t = transform.matrix();

        let tl = (t * vec4(-0.5, -0.5, 0.0, 1.0)).xyz();
        let tr = (t * vec4(0.5, -0.5, 0.0, 1.0)).xyz();
        let br = (t * vec4(0.5, 0.5, 0.0, 1.0)).xyz();
        let bl = (t * vec4(-0.5, 0.5, 0.0, 1.0)).xyz();

        self.draw_line(tl, tr, color);
        self.draw_line(tr, br, color);
        self.draw_line(br, bl, color);
        self.draw_line(bl, tl, color);
    }

    /// Draw circle with diameter 1
    pub fn draw_circle(&mut self, transform: &crate::Transform3D, color: Vec3) {
        let n = self.resolution;
        let t = transform.matrix();

        for i in 0..n {
            let angle1 = (i as f32 / n as f32) * 2.0 * PI;
            let angle2 = ((i + 1) as f32 / n as f32) * 2.0 * PI;
            let p1 = (t * vec4(0.5 * angle1.cos(), 0.5 * angle1.sin(), 0.0, 1.0)).xyz();
            let p2 = (t * vec4(0.5 * angle2.cos(), 0.5 * angle2.sin(), 0.0, 1.0)).xyz();
            self.draw_line(p1, p2, color);
        }
    }

    /// Draw cube with side 1
    pub fn draw_cube(&mut self, transform: &crate::Transform3D, color: Vec3) {
        let t = transform.matrix();

        let lbl = (t * vec4(-0.5, -0.5, -0.5, 1.0)).xyz(); // lower bottom left
        let lbr = (t * vec4(0.5, -0.5, -0.5, 1.0)).xyz(); // lower bottom right
        let ltr = (t * vec4(0.5, -0.5, 0.5, 1.0)).xyz(); // lower top right
        let ltl = (t * vec4(-0.5, -0.5, 0.5, 1.0)).xyz(); // lower top left

        let ubl = (t * vec4(-0.5, 0.5, -0.5, 1.0)).xyz(); // upper bottom left
        let ubr = (t * vec4(0.5, 0.5, -0.5, 1.0)).xyz(); // upper bottom right
        let utr = (t * vec4(0.5, 0.5, 0.5, 1.0)).xyz(); // upper top right
        let utl = (t * vec4(-0.5, 0.5, 0.5, 1.0)).xyz(); // upper top left

        self.draw_line(lbl, lbr, color);
        self.draw_line(lbr, ltr, color);
        self.draw_line(ltr, ltl, color);
        self.draw_line(ltl, lbl, color);

        self.draw_line(ubl, ubr, color);
        self.draw_line(ubr, utr, color);
        self.draw_line(utr, utl, color);
        self.draw_line(utl, ubl, color);

        self.draw_line(lbl, ubl, color);
        self.draw_line(lbr, ubr, color);
        self.draw_line(ltr, utr, color);
        self.draw_line(ltl, utl, color);
    }

    /// Draw sphere with diameter 1
    pub fn draw_sphere(&mut self, transform: &crate::Transform3D, color: Vec3) {
        let n = self.resolution;
        let t = transform.matrix();

        // xy
        for i in 0..n {
            let angle1 = (i as f32 / n as f32) * 2.0 * PI;
            let angle2 = ((i + 1) as f32 / n as f32) * 2.0 * PI;
            let p1 = (t * vec4(0.5 * angle1.cos(), 0.5 * angle1.sin(), 0.0, 1.0)).xyz();
            let p2 = (t * vec4(0.5 * angle2.cos(), 0.5 * angle2.sin(), 0.0, 1.0)).xyz();
            self.draw_line(p1, p2, color);
        }

        // yz
        for i in 0..n {
            let angle1 = (i as f32 / n as f32) * 2.0 * PI;
            let angle2 = ((i + 1) as f32 / n as f32) * 2.0 * PI;
            let p1 = (t * vec4(0.0, 0.5 * angle1.sin(), 0.5 * angle1.cos(), 1.0)).xyz();
            let p2 = (t * vec4(0.0, 0.5 * angle2.sin(), 0.5 * angle2.cos(), 1.0)).xyz();
            self.draw_line(p1, p2, color);
        }

        // xz
        for i in 0..n {
            let angle1 = (i as f32 / n as f32) * 2.0 * PI;
            let angle2 = ((i + 1) as f32 / n as f32) * 2.0 * PI;
            let p1 = (t * vec4(0.5 * angle1.cos(), 0.0, 0.5 * angle1.sin(), 1.0)).xyz();
            let p2 = (t * vec4(0.5 * angle2.cos(), 0.0, 0.5 * angle2.sin(), 1.0)).xyz();
            self.draw_line(p1, p2, color);
        }
    }
}
