use super::{
    BindGroupCombinedBuilder, BindGroupCombinedEntry, DepthBuffer, DynamicIndexBuffer,
    DynamicIndexBufferBuilder, DynamicVertexBuffer, DynamicVertexBufferBuilder, EncoderBuilder,
    PerspectiveCamera, PerspectiveCameraUniform, RenderPipelineBuilder, ShaderBuilder, Transform,
    UniformBuffer, UniformBufferBuilder, VertexColor,
};
use crate::{render, Context};
use encase::ShaderType;
use glam::{vec3, Quat, Vec2, Vec3, Vec4Swizzles};
use std::f32::consts::PI;

pub struct GizmoRenderer {
    vertex_buffer: DynamicVertexBuffer<VertexColor>,
    index_buffer: DynamicIndexBuffer,
    bindgroup: wgpu::BindGroup,
    pipeline: wgpu::RenderPipeline,

    camera_buffer: UniformBuffer,
    depth_buffer: DepthBuffer,
}

const GIZMO_MAX_VERTICES: usize = 10000;
const GIZMO_MAX_INDICES: usize = 10000;
const GIZMO_RESOLUTION: u32 = 16;
impl GizmoRenderer {
    pub fn new(ctx: &Context) -> Self {
        let vertex_buffer = DynamicVertexBufferBuilder::new(GIZMO_MAX_VERTICES).build(ctx);
        let index_buffer = DynamicIndexBufferBuilder::new(GIZMO_MAX_INDICES).build(ctx);

        let camera_buffer = UniformBufferBuilder::new()
            .usage(wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST)
            .build(ctx, PerspectiveCameraUniform::min_size());
        let (bindgroup_layout, bindgroup) = BindGroupCombinedBuilder::new()
            .entries(&[
                BindGroupCombinedEntry::new(camera_buffer.buf().as_entire_binding())
                    .uniform()
                    .visibility(wgpu::ShaderStages::VERTEX),
            ])
            .build(ctx);

        let shader = ShaderBuilder::new(include_str!("../../../assets/gizmo.wgsl")).build(ctx);
        let pipeline = RenderPipelineBuilder::new(&shader)
            .buffers(&[vertex_buffer.desc()])
            .targets(&[RenderPipelineBuilder::default_target(ctx)])
            .depth_stencil(DepthBuffer::depth_stencil_state())
            .bind_groups(&[&bindgroup_layout])
            .topology(wgpu::PrimitiveTopology::LineList)
            .build(ctx);

        let depth_buffer = DepthBuffer::new(ctx);

        Self {
            vertex_buffer,
            index_buffer,
            pipeline,
            depth_buffer,
            camera_buffer,
            bindgroup,
        }
    }
    pub fn render(
        &mut self,
        ctx: &Context,
        view: &wgpu::TextureView,
        camera: &mut PerspectiveCamera,
    ) {
        self.vertex_buffer.update_buffer(ctx);
        self.index_buffer.update_buffer(ctx);
        self.camera_buffer.write(ctx, &camera.uniform(ctx));

        let mut encoder = EncoderBuilder::new().build(ctx);
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(self.depth_buffer.depth_stencil_attachment_clear()),
            label: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        pass.set_pipeline(&self.pipeline);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.set_index_buffer(self.index_buffer.slice(..), self.index_buffer.format());
        pass.set_bind_group(0, &self.bindgroup, &[]);
        pass.draw_indexed(0..self.index_buffer.len(), 0, 0..1);
        drop(pass);

        let queue = render::queue(ctx);
        queue.submit(Some(encoder.finish()));

        self.vertex_buffer.clear();
        self.index_buffer.clear();
    }
    pub fn resize(&mut self, ctx: &Context) {
        self.depth_buffer.resize(ctx);
    }
}

impl GizmoRenderer {
    pub fn draw_line(&mut self, start: Vec3, end: Vec3, color: Vec3) {
        let vertex_start = self.vertex_buffer.len();
        self.vertex_buffer.add(VertexColor {
            position: start.to_array(),
            color: color.to_array(),
        });
        self.vertex_buffer.add(VertexColor {
            position: end.to_array(),
            color: color.to_array(),
        });
        self.index_buffer.add(vertex_start);
        self.index_buffer.add(vertex_start + 1);
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
            self.vertex_buffer.add(VertexColor {
                position: pos.to_array(),
                color: color.to_array(),
            });
            self.index_buffer.add(vertex_start + i);
            self.index_buffer.add(vertex_start + (i + 1) % n);
        }
        for i in 0..n {
            let p = i as f32 / n as f32;
            let angle = p * 2.0 * PI;
            let pos = Quat::from_rotation_x(PI / 2.0)
                * vec3(radius * angle.cos(), radius * angle.sin(), 0.0);
            let pos = (transform * pos.extend(1.0)).xyz();
            self.vertex_buffer.add(VertexColor {
                position: pos.to_array(),
                color: color.to_array(),
            });
            self.index_buffer.add(vertex_start + n + i);
            self.index_buffer.add(vertex_start + n + (i + 1) % n);
        }
        for i in 0..n {
            let p = i as f32 / n as f32;
            let angle = p * 2.0 * PI;
            let pos = Quat::from_rotation_y(PI / 2.0)
                * vec3(radius * angle.cos(), radius * angle.sin(), 0.0);
            let pos = (transform * pos.extend(1.0)).xyz();
            self.vertex_buffer.add(VertexColor {
                position: pos.to_array(),
                color: color.to_array(),
            });
            self.index_buffer.add(vertex_start + 2 * n + i);
            self.index_buffer.add(vertex_start + 2 * n + (i + 1) % n);
        }
    }

    pub fn draw_cube(&mut self, dimensions: Vec3, transform: &Transform, color: Vec3) {
        let d = dimensions;
        let t = transform.matrix();
        let vertex_start = self.vertex_buffer.len();

        let lbl = vec3(-d.x * 0.5, -d.y * 0.5, -d.z * 0.5); // lower bottom left
        let lbr = vec3(d.x * 0.5, -d.y * 0.5, -d.z * 0.5); // lower bottom right
        let ltr = vec3(d.x * 0.5, -d.y * 0.5, d.z * 0.5); // lower top right
        let ltl = vec3(-d.x * 0.5, -d.y * 0.5, d.z * 0.5); // lower top left

        let ubl = vec3(-d.x * 0.5, d.y * 0.5, -d.z * 0.5); // upper bottom left
        let ubr = vec3(d.x * 0.5, d.y * 0.5, -d.z * 0.5); // upper bottom right
        let utr = vec3(d.x * 0.5, d.y * 0.5, d.z * 0.5); // upper top right
        let utl = vec3(-d.x * 0.5, d.y * 0.5, d.z * 0.5); // upper top left

        // Bottom
        self.vertex_buffer.add(VertexColor {
            position: (t * lbl.extend(1.0)).xyz().to_array(),
            color: color.to_array(),
        });
        self.vertex_buffer.add(VertexColor {
            position: (t * lbr.extend(1.0)).xyz().to_array(),
            color: color.to_array(),
        });
        self.vertex_buffer.add(VertexColor {
            position: (t * ltr.extend(1.0)).xyz().to_array(),
            color: color.to_array(),
        });
        self.vertex_buffer.add(VertexColor {
            position: (t * ltl.extend(1.0)).xyz().to_array(),
            color: color.to_array(),
        });

        // Top
        self.vertex_buffer.add(VertexColor {
            position: (t * ubl.extend(1.0)).xyz().to_array(),
            color: color.to_array(),
        });
        self.vertex_buffer.add(VertexColor {
            position: (t * ubr.extend(1.0)).xyz().to_array(),
            color: color.to_array(),
        });
        self.vertex_buffer.add(VertexColor {
            position: (t * utr.extend(1.0)).xyz().to_array(),
            color: color.to_array(),
        });
        self.vertex_buffer.add(VertexColor {
            position: (t * utl.extend(1.0)).xyz().to_array(),
            color: color.to_array(),
        });

        // Bottom
        self.index_buffer.add(vertex_start);
        self.index_buffer.add(vertex_start + 1);

        self.index_buffer.add(vertex_start + 1);
        self.index_buffer.add(vertex_start + 2);

        self.index_buffer.add(vertex_start + 2);
        self.index_buffer.add(vertex_start + 3);

        self.index_buffer.add(vertex_start + 3);
        self.index_buffer.add(vertex_start);

        // Top
        self.index_buffer.add(vertex_start + 4);
        self.index_buffer.add(vertex_start + 5);

        self.index_buffer.add(vertex_start + 5);
        self.index_buffer.add(vertex_start + 6);

        self.index_buffer.add(vertex_start + 6);
        self.index_buffer.add(vertex_start + 7);

        self.index_buffer.add(vertex_start + 7);
        self.index_buffer.add(vertex_start + 4);

        // Connections
        self.index_buffer.add(vertex_start);
        self.index_buffer.add(vertex_start + 4);

        self.index_buffer.add(vertex_start + 1);
        self.index_buffer.add(vertex_start + 5);

        self.index_buffer.add(vertex_start + 2);
        self.index_buffer.add(vertex_start + 6);

        self.index_buffer.add(vertex_start + 3);
        self.index_buffer.add(vertex_start + 7);
    }

    pub fn draw_quad(&mut self, dimensions: Vec2, transform: &Transform, color: Vec3) {
        let vertex_start = self.vertex_buffer.len();
        let d = dimensions;
        let t = transform.matrix();

        let bl = vec3(-d.x * 0.5, -d.y * 0.5, 0.0);
        let br = vec3(d.x * 0.5, -d.y * 0.5, 0.0);
        let tr = vec3(d.x * 0.5, d.y * 0.5, 0.0);
        let tl = vec3(-d.x * 0.5, d.y * 0.5, 0.0);

        self.vertex_buffer.add(VertexColor {
            position: (t * bl.extend(1.0)).xyz().to_array(),
            color: color.to_array(),
        });
        self.vertex_buffer.add(VertexColor {
            position: (t * br.extend(1.0)).xyz().to_array(),
            color: color.to_array(),
        });
        self.vertex_buffer.add(VertexColor {
            position: (t * tr.extend(1.0)).xyz().to_array(),
            color: color.to_array(),
        });
        self.vertex_buffer.add(VertexColor {
            position: (t * tl.extend(1.0)).xyz().to_array(),
            color: color.to_array(),
        });

        self.index_buffer.add(vertex_start);
        self.index_buffer.add(vertex_start + 1);

        self.index_buffer.add(vertex_start + 1);
        self.index_buffer.add(vertex_start + 2);

        self.index_buffer.add(vertex_start + 2);
        self.index_buffer.add(vertex_start + 3);

        self.index_buffer.add(vertex_start + 3);
        self.index_buffer.add(vertex_start);
    }
    pub fn draw_circle(&mut self, radius: f32, transform: &Transform, color: Vec3) {
        let n = GIZMO_RESOLUTION;
        let t = transform.matrix();

        let vertex_start = self.vertex_buffer.len();

        for i in 0..n {
            let p = i as f32 / n as f32;
            let angle = p * 2.0 * PI;
            let pos = vec3(radius * angle.cos(), radius * angle.sin(), 0.0);
            self.vertex_buffer.add(VertexColor {
                position: (t * pos.extend(1.0)).xyz().to_array(),
                color: color.to_array(),
            });
        }

        for i in 0..n {
            self.index_buffer.add(vertex_start + i);
            self.index_buffer.add(vertex_start + (i + 1) % n);
        }
    }
}
