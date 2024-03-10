use crate::Context;
use encase::ShaderType;
use glam::{Mat4, Quat, Vec3};

//
// Transform
//

#[derive(Debug)]
pub struct Transform {
    pub pos: Vec3,
    pub rot: Quat,
    pub scale: Vec3,
}

impl Transform {
    pub fn new(pos: Vec3, rot: Quat, scale: Vec3) -> Self {
        Self { pos, rot, scale }
    }
}

impl Default for Transform {
    fn default() -> Self {
        Self {
            pos: Vec3::ZERO,
            rot: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }
}

//
// Transform GPU
//

pub struct TransformGPU {
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
    buffer: wgpu::Buffer,
}

impl TransformGPU {
    pub fn new(device: &wgpu::Device) -> Self {
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
            buffer,
            bind_group,
            bind_group_layout,
        }
    }

    pub fn update_buffer(&mut self, ctx: &Context, transform: &Transform) {
        // Create uniform
        let uniform = TransformUniform {
            matrix: Mat4::from_scale_rotation_translation(
                transform.scale,
                transform.rot,
                transform.pos,
            ),
        };

        // Upload data to gpu
        let queue = ctx.render.queue.clone();
        let mut buffer = encase::UniformBuffer::new(Vec::new());
        buffer
            .write(&uniform)
            .expect("could not write to transform buffer");
        queue.write_buffer(&self.buffer, 0, &buffer.into_inner());
    }

    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }
}
#[derive(ShaderType)]
pub struct TransformUniform {
    matrix: Mat4,
}

// Re-export transform function
// impl TransformGPU {
//     pub fn pos(mut self, pos: Vec3) -> Self {
//         self.transform.pos = pos;
//         self
//     }
//     pub fn rotation(mut self, rotation: Quat) -> Self {
//         self.transform.rot = rotation;
//         self
//     }
//     pub fn scale(mut self, scale: Vec3) -> Self {
//         self.transform.scale = scale;
//         self
//     }
// }
