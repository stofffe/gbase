use crate::{
    filesystem,
    render::{
        self, ArcBindGroup, ArcRenderPipeline, EncoderBuilder, RenderPipelineBuilder,
        ShaderBuilder, Transform, VertexColor,
    },
    Context,
};
use glam::{vec3, vec4, Quat, Vec2, Vec3, Vec4Swizzles};
use std::f32::consts::PI;

use super::CameraUniform;

pub struct GizmoRenderer {
    dynamic_vertex_buffer: Vec<VertexColor>,
    dynamic_index_buffer: Vec<u32>,

    vertex_buffer: render::VertexBuffer<VertexColor>,
    index_buffer: render::IndexBuffer,
    bindgroup: ArcBindGroup,
    pipeline: ArcRenderPipeline,

    depth_buffer: render::DepthBuffer,
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
        let vertex_buffer = render::VertexBufferBuilder::new(render::VertexBufferSource::Empty(
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

        let shader_str = filesystem::load_s!("shaders/gizmo.wgsl").expect("could not load shader");
        let shader = ShaderBuilder::new(shader_str).build(ctx);
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
        }
    }
    pub fn render(&mut self, ctx: &Context, view: &wgpu::TextureView) {
        self.vertex_buffer.write(ctx, &self.dynamic_vertex_buffer);
        self.index_buffer.write(ctx, &self.dynamic_index_buffer);

        let mut encoder = EncoderBuilder::new().build(ctx);
        render::RenderPassBuilder::new()
            .color_attachments(&[Some(render::RenderPassColorAttachment::new(view))])
            .depth_stencil_attachment(self.depth_buffer.depth_render_attachment_clear())
            .build_run(&mut encoder, |mut pass| {
                pass.set_pipeline(&self.pipeline);
                pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                pass.set_index_buffer(self.index_buffer.slice(..), self.index_buffer.format());
                pass.set_bind_group(0, Some(self.bindgroup.as_ref()), &[]);
                pass.draw_indexed(0..self.index_buffer.len(), 0, 0..1);
            });

        let queue = render::queue(ctx);
        queue.submit(Some(encoder.finish()));

        self.dynamic_vertex_buffer.clear();
        self.dynamic_index_buffer.clear();
    }

    pub fn resize(&mut self, ctx: &Context, width: u32, height: u32) {
        self.depth_buffer.resize(ctx, width, height);
    }
    pub fn resize_screen(&mut self, ctx: &Context) {
        self.depth_buffer.resize_screen(ctx);
    }
}

impl GizmoRenderer {
    pub fn draw_line(&mut self, start: Vec3, end: Vec3, color: Vec3) {
        let vertex_start = self.dynamic_vertex_buffer.len();
        self.dynamic_vertex_buffer.push(VertexColor {
            position: start.to_array(),
            color: color.to_array(),
        });
        self.dynamic_vertex_buffer.push(VertexColor {
            position: end.to_array(),
            color: color.to_array(),
        });
        self.dynamic_index_buffer.push(vertex_start as u32);
        self.dynamic_index_buffer.push(vertex_start as u32 + 1);
    }

    pub fn draw_sphere(&mut self, radius: f32, transform: &Transform, color: Vec3) {
        let n = GIZMO_RESOLUTION;
        let vertex_start = self.vertex_buffer.len();
        let transform = transform.matrix();

        for i in 0..n {
            let p = i as f32 / n as f32;
            let angle = p * 2.0 * PI;
            let pos = vec3(radius * angle.cos(), radius * angle.sin(), 0.0);
            let pos = (transform * pos.extend(1.0)).xyz();
            self.dynamic_vertex_buffer.push(VertexColor {
                position: pos.to_array(),
                color: color.to_array(),
            });
            self.dynamic_index_buffer.push(vertex_start + i);
            self.dynamic_index_buffer.push(vertex_start + (i + 1) % n);
        }
        for i in 0..n {
            let p = i as f32 / n as f32;
            let angle = p * 2.0 * PI;
            let pos = Quat::from_rotation_x(PI / 2.0)
                * vec3(radius * angle.cos(), radius * angle.sin(), 0.0);
            let pos = (transform * pos.extend(1.0)).xyz();
            self.dynamic_vertex_buffer.push(VertexColor {
                position: pos.to_array(),
                color: color.to_array(),
            });
            self.dynamic_index_buffer.push(vertex_start + n + i);
            self.dynamic_index_buffer
                .push(vertex_start + n + (i + 1) % n);
        }
        for i in 0..n {
            let p = i as f32 / n as f32;
            let angle = p * 2.0 * PI;
            let pos = Quat::from_rotation_y(PI / 2.0)
                * vec3(radius * angle.cos(), radius * angle.sin(), 0.0);
            let pos = (transform * pos.extend(1.0)).xyz();
            self.dynamic_vertex_buffer.push(VertexColor {
                position: pos.to_array(),
                color: color.to_array(),
            });
            self.dynamic_index_buffer.push(vertex_start + 2 * n + i);
            self.dynamic_index_buffer
                .push(vertex_start + 2 * n + (i + 1) % n);
        }
    }

    /// Draw unit cube
    pub fn draw_cube(&mut self, transform: &Transform, color: Vec3) {
        let t = transform.matrix();
        let vertex_start = self.dynamic_vertex_buffer.len() as u32;

        // Create unit cube
        let lbl = vec4(-0.5, -0.5, -0.5, 1.0); // lower bottom left
        let lbr = vec4(0.5, -0.5, -0.5, 1.0); // lower bottom right
        let ltr = vec4(0.5, -0.5, 0.5, 1.0); // lower top right
        let ltl = vec4(-0.5, -0.5, 0.5, 1.0); // lower top left

        let ubl = vec4(-0.5, 0.5, -0.5, 1.0); // upper bottom left
        let ubr = vec4(0.5, 0.5, -0.5, 1.0); // upper bottom right
        let utr = vec4(0.5, 0.5, 0.5, 1.0); // upper top right
        let utl = vec4(-0.5, 0.5, 0.5, 1.0); // upper top left

        // Bottom
        self.dynamic_vertex_buffer.push(VertexColor {
            position: (t * lbl).xyz().to_array(),
            color: color.to_array(),
        });
        self.dynamic_vertex_buffer.push(VertexColor {
            position: (t * lbr).xyz().to_array(),
            color: color.to_array(),
        });
        self.dynamic_vertex_buffer.push(VertexColor {
            position: (t * ltr).xyz().to_array(),
            color: color.to_array(),
        });
        self.dynamic_vertex_buffer.push(VertexColor {
            position: (t * ltl).xyz().to_array(),
            color: color.to_array(),
        });

        // Top
        self.dynamic_vertex_buffer.push(VertexColor {
            position: (t * ubl).xyz().to_array(),
            color: color.to_array(),
        });
        self.dynamic_vertex_buffer.push(VertexColor {
            position: (t * ubr).xyz().to_array(),
            color: color.to_array(),
        });
        self.dynamic_vertex_buffer.push(VertexColor {
            position: (t * utr).xyz().to_array(),
            color: color.to_array(),
        });
        self.dynamic_vertex_buffer.push(VertexColor {
            position: (t * utl).xyz().to_array(),
            color: color.to_array(),
        });

        // Bottom
        self.dynamic_index_buffer.push(vertex_start);
        self.dynamic_index_buffer.push(vertex_start + 1);

        self.dynamic_index_buffer.push(vertex_start + 1);
        self.dynamic_index_buffer.push(vertex_start + 2);

        self.dynamic_index_buffer.push(vertex_start + 2);
        self.dynamic_index_buffer.push(vertex_start + 3);

        self.dynamic_index_buffer.push(vertex_start + 3);
        self.dynamic_index_buffer.push(vertex_start);

        // Top
        self.dynamic_index_buffer.push(vertex_start + 4);
        self.dynamic_index_buffer.push(vertex_start + 5);

        self.dynamic_index_buffer.push(vertex_start + 5);
        self.dynamic_index_buffer.push(vertex_start + 6);

        self.dynamic_index_buffer.push(vertex_start + 6);
        self.dynamic_index_buffer.push(vertex_start + 7);

        self.dynamic_index_buffer.push(vertex_start + 7);
        self.dynamic_index_buffer.push(vertex_start + 4);

        // Connections
        self.dynamic_index_buffer.push(vertex_start);
        self.dynamic_index_buffer.push(vertex_start + 4);

        self.dynamic_index_buffer.push(vertex_start + 1);
        self.dynamic_index_buffer.push(vertex_start + 5);

        self.dynamic_index_buffer.push(vertex_start + 2);
        self.dynamic_index_buffer.push(vertex_start + 6);

        self.dynamic_index_buffer.push(vertex_start + 3);
        self.dynamic_index_buffer.push(vertex_start + 7);
    }

    pub fn draw_quad(&mut self, dimensions: Vec2, transform: &Transform, color: Vec3) {
        let vertex_start = self.dynamic_vertex_buffer.len() as u32;
        let d = dimensions;
        let t = transform.matrix();

        let bl = vec3(-d.x * 0.5, -d.y * 0.5, 0.0);
        let br = vec3(d.x * 0.5, -d.y * 0.5, 0.0);
        let tr = vec3(d.x * 0.5, d.y * 0.5, 0.0);
        let tl = vec3(-d.x * 0.5, d.y * 0.5, 0.0);

        self.dynamic_vertex_buffer.push(VertexColor {
            position: (t * bl.extend(1.0)).xyz().to_array(),
            color: color.to_array(),
        });
        self.dynamic_vertex_buffer.push(VertexColor {
            position: (t * br.extend(1.0)).xyz().to_array(),
            color: color.to_array(),
        });
        self.dynamic_vertex_buffer.push(VertexColor {
            position: (t * tr.extend(1.0)).xyz().to_array(),
            color: color.to_array(),
        });
        self.dynamic_vertex_buffer.push(VertexColor {
            position: (t * tl.extend(1.0)).xyz().to_array(),
            color: color.to_array(),
        });

        self.dynamic_index_buffer.push(vertex_start);
        self.dynamic_index_buffer.push(vertex_start + 1);

        self.dynamic_index_buffer.push(vertex_start + 1);
        self.dynamic_index_buffer.push(vertex_start + 2);

        self.dynamic_index_buffer.push(vertex_start + 2);
        self.dynamic_index_buffer.push(vertex_start + 3);

        self.dynamic_index_buffer.push(vertex_start + 3);
        self.dynamic_index_buffer.push(vertex_start);
    }
    pub fn draw_circle(&mut self, radius: f32, transform: &Transform, color: Vec3) {
        let n = GIZMO_RESOLUTION;
        let t = transform.matrix();

        let vertex_start = self.dynamic_vertex_buffer.len();

        for i in 0..n {
            let p = i as f32 / n as f32;
            let angle = p * 2.0 * PI;
            let pos = vec3(radius * angle.cos(), radius * angle.sin(), 0.0);
            self.dynamic_vertex_buffer.push(VertexColor {
                position: (t * pos.extend(1.0)).xyz().to_array(),
                color: color.to_array(),
            });
        }

        for i in 0..n {
            self.dynamic_index_buffer.push(vertex_start as u32 + i);
            self.dynamic_index_buffer
                .push(vertex_start as u32 + (i + 1) % n);
        }
    }
}
