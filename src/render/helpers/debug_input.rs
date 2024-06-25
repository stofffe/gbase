use crate::{
    input,
    render::{self, ArcBindGroup, ArcBindGroupLayout, ArcBuffer},
    Context,
};
use encase::ShaderType;
use winit::keyboard::KeyCode;

/// Debug information for use in shaders
pub struct DebugInput {
    buffer: render::UniformBuffer,
    bindgroup_layout: ArcBindGroupLayout,
    bindgroup: ArcBindGroup,
}

impl DebugInput {
    pub fn new(ctx: &mut Context) -> Self {
        let buffer = render::UniformBufferBuilder::new()
            .usage(wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST)
            .build(ctx, DebugInputUniform::min_size());

        let bindgroup_layout = render::BindGroupLayoutBuilder::new()
            .entries(vec![render::BindGroupLayoutEntry::new()
                .vertex()
                .fragment()
                .compute()
                .uniform()])
            .build(ctx);
        let bindgroup = render::BindGroupBuilder::new(bindgroup_layout.clone())
            .entries(vec![render::BindGroupEntry::Buffer(buffer.buffer())])
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
    pub fn buffer(&self) -> ArcBuffer {
        self.buffer.buffer()
    }
    pub fn buffer_ref(&self) -> &wgpu::Buffer {
        self.buffer.buffer_ref()
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
