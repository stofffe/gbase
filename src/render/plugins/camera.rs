use crate::{input, Context};
use encase::ShaderType;
use glam::{vec3, Mat4, Vec3};
use std::f32::consts::PI;

pub struct PerspectiveCamera {
    pub pos: Vec3,
    pub yaw: f32,
    pub pitch: f32,

    pub fov: f32,
    pub znear: f32,
    pub zfar: f32,

    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
    buffer: wgpu::Buffer,
}

#[derive(ShaderType)]
pub struct PerspectiveCameraUniform {
    view_proj: Mat4,
    pos: Vec3,
    btn: u32,
}

impl PerspectiveCamera {
    pub fn new(device: &wgpu::Device) -> Self {
        let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("camera buffer"),
            size: u64::from(PerspectiveCameraUniform::min_size()),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST, // OPTION
            mapped_at_creation: false,
        });

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("camera bg layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT | wgpu::ShaderStages::COMPUTE, // OPTION
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("camera bg"),
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        Self {
            pos: Vec3::ZERO,
            yaw: 0.0,
            pitch: 0.0,
            fov: PI / 2.0,
            znear: 0.1,
            zfar: 1000.0,
            bind_group_layout: camera_bind_group_layout,
            bind_group: camera_bind_group,
            buffer: camera_buffer,
        }
    }

    pub fn world_up(&self) -> Vec3 {
        vec3(0.0, 1.0, 0.0)
    }

    // TODO not sure about these
    pub fn forward(&self) -> Vec3 {
        vec3(
            self.yaw.sin() * self.pitch.cos(),
            -self.pitch.sin(),
            self.yaw.cos() * self.pitch.cos(),
        )
        .normalize()
    }
    pub fn right(&self) -> Vec3 {
        vec3(self.yaw.cos(), 0.0, -self.yaw.sin()).normalize()
    }
    pub fn up(&self) -> Vec3 {
        vec3(
            self.yaw.sin() * self.pitch.sin(),
            self.pitch.cos(),
            self.yaw.cos() * self.pitch.sin(),
        )
        .normalize()
    }

    pub fn uniform(&mut self, ctx: &Context) -> PerspectiveCameraUniform {
        const MIN_PITCH: f32 = -PI / 2.0 + 0.1;
        const MAX_PITCH: f32 = PI / 2.0 - 0.1;
        const MIN_FOV: f32 = 0.1;
        const MAX_FOV: f32 = PI - 0.1;

        // left handed coords
        self.pitch = self.pitch.clamp(MIN_PITCH, MAX_PITCH);
        self.fov = self.fov.clamp(MIN_FOV, MAX_FOV);
        let view = Mat4::look_to_lh(self.pos, self.forward(), self.up());
        let aspect = ctx.render.aspect_ratio();
        let proj = Mat4::perspective_lh(self.fov, aspect, self.znear, self.zfar);

        let view_proj = proj * view;

        // TODO DEBUG
        let mut btn = 0;
        if input::key_pressed(ctx, winit::keyboard::KeyCode::KeyL) {
            btn = 1;
        }

        let pos = self.pos;

        PerspectiveCameraUniform {
            view_proj,
            pos,
            btn,
        }
    }

    pub fn update_buffer(&mut self, ctx: &Context) {
        let queue = ctx.render.queue.clone();
        let mut buffer = encase::UniformBuffer::new(Vec::new());
        buffer
            .write(&self.uniform(ctx))
            .expect("could not write to camera buffer");
        queue.write_buffer(&self.buffer, 0, &buffer.into_inner());
    }

    pub fn pos(mut self, pos: Vec3) -> Self {
        self.pos = pos;
        self
    }
    pub fn yaw(mut self, yaw: f32) -> Self {
        self.yaw = yaw;
        self
    }
    pub fn pitch(mut self, pitch: f32) -> Self {
        self.pitch = pitch;
        self
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

// pub fn set_pos(&mut self, pos: Vec3) {
//     self.pos = pos
// }
// pub fn set_yaw(&mut self, yaw: f32) {
//     self.yaw = yaw;
// }
// pub fn set_pitch(&mut self, pitch: f32) {
//     self.pitch = pitch.clamp(-89.0, 89.0);
// }
// pub fn add_pos(&mut self, pos: Vec3) {
//     self.pos += pos
// }
// pub fn add_yaw(&mut self, yaw: f32) {
//     self.yaw += yaw;
// }
// pub fn add_pitch(&mut self, pitch: f32) {
//     self.pitch += pitch;
// }
