use encase::ShaderType;
use glam::{Mat4, Quat, Vec3};

use crate::Context;

pub struct Transform {
    pub pos: Vec3,
    pub rot: Quat,
    pub scale: Vec3,

    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
    pub buffer: wgpu::Buffer,
}

impl Transform {
    pub fn new(device: &wgpu::Device) -> Self {
        let pos = Vec3::ZERO;
        let rotation = Quat::IDENTITY;
        let scale = Vec3::ONE;

        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("transform buffer"),
            size: u64::from(TransformUniform::min_size()),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("transform bg layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("transform bg"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });
        Self {
            pos,
            rot: rotation,
            scale,

            buffer,
            bind_group,
            bind_group_layout,
        }
    }

    pub fn uniform(&self) -> TransformUniform {
        let matrix = Mat4::from_scale_rotation_translation(self.scale, self.rot, self.pos);
        TransformUniform { matrix }
    }

    pub fn update_buffer(&mut self, ctx: &Context) {
        let queue = ctx.render.queue.clone();
        let mut buffer = encase::UniformBuffer::new(Vec::new());
        buffer
            .write(&self.uniform())
            .expect("could not write to transform buffer");
        queue.write_buffer(&self.buffer, 0, &buffer.into_inner());
    }

    pub fn pos(mut self, pos: Vec3) -> Self {
        self.pos = pos;
        self
    }
    pub fn rotation(mut self, rotation: Quat) -> Self {
        self.rot = rotation;
        self
    }
    pub fn scale(mut self, scale: Vec3) -> Self {
        self.scale = scale;
        self
    }
}

#[derive(ShaderType)]
pub struct TransformUniform {
    matrix: Mat4,
}
