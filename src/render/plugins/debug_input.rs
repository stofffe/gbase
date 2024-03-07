#![allow(clippy::new_without_default)]

use encase::ShaderType;
use winit::keyboard::KeyCode;

use crate::{input, render, Context};

/// Debug information for use in shaders
pub struct DebugInput {
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
    buffer: wgpu::Buffer,
}

impl DebugInput {
    pub fn new(ctx: &Context) -> Self {
        let device = render::device(ctx);
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("debug input buffer"),
            size: u64::from(DebugInputUniform::min_size()),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("debug input bg layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX
                    | wgpu::ShaderStages::FRAGMENT
                    | wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("debug input bg"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });

        Self {
            bind_group_layout,
            bind_group,
            buffer,
        }
    }

    #[rustfmt::skip]
    pub fn update_buffer(&mut self, ctx: &Context) {
        let mut uniform = DebugInputUniform::default();
        // Update input
        if input::key_pressed(ctx, KeyCode::F1) { uniform.btn1 = 1; }
        if input::key_pressed(ctx, KeyCode::F2) { uniform.btn2 = 1; }
        if input::key_pressed(ctx, KeyCode::F3) { uniform.btn3 = 1; }
        if input::key_pressed(ctx, KeyCode::F4) { uniform.btn4 = 1; }
        if input::key_pressed(ctx, KeyCode::F5) { uniform.btn5 = 1; }
        if input::key_pressed(ctx, KeyCode::F6) { uniform.btn6 = 1; }
        if input::key_pressed(ctx, KeyCode::F7) { uniform.btn7 = 1; }
        if input::key_pressed(ctx, KeyCode::F8) { uniform.btn8 = 1; }
        if input::key_pressed(ctx, KeyCode::F9) { uniform.btn9 = 1; }

        // Update buffer
        let queue = render::queue(ctx);
        let mut buffer = encase::UniformBuffer::new(Vec::new());
        buffer
            .write(&uniform)
            .expect("could not write to debug input buffer");
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

#[derive(ShaderType, Default)]
pub struct DebugInputUniform {
    btn1: u32,
    btn2: u32,
    btn3: u32,
    btn4: u32,
    btn5: u32,
    btn6: u32,
    btn7: u32,
    btn8: u32,
    btn9: u32,
}
