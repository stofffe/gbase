use crate::{input, Context};
use encase::ShaderType;
use winit::keyboard::KeyCode;

/// Debug information for use in shaders
pub struct DebugInput {
    buffer: super::UniformBuffer,
    bindgroup_layout: wgpu::BindGroupLayout,
    bindgroup: wgpu::BindGroup,
}

impl DebugInput {
    pub fn new(ctx: &Context) -> Self {
        let buffer = super::UniformBufferBuilder::new()
            .usage(wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST)
            .build(ctx, DebugInputUniform::min_size());

        let (bindgroup_layout, bindgroup) = super::BindGroupCombinedBuilder::new()
            .entries(&[
                super::BindGroupCombinedEntry::new(buffer.buf().as_entire_binding())
                    .visibility(wgpu::ShaderStages::VERTEX_FRAGMENT | wgpu::ShaderStages::COMPUTE)
                    .uniform(),
            ])
            .build(ctx);
        Self {
            buffer,
            bindgroup_layout,
            bindgroup,
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
        self.buffer.write(ctx, &uniform);
    }

    pub fn bindgroup_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bindgroup_layout
    }
    pub fn bindgroup(&self) -> &wgpu::BindGroup {
        &self.bindgroup
    }
    pub fn buffer(&self) -> &wgpu::Buffer {
        self.buffer.buf()
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
