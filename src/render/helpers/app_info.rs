use encase::ShaderType;

use crate::{render, time, Context};

/// Holds information about the app
///
/// Can easily be sent as uniform to shaders
pub struct AppInfo {
    bindgroup_layout: render::ArcBindGroupLayout,
    bindgroup: render::ArcBindGroup,
    buffer: render::UniformBuffer,
}

impl AppInfo {
    pub fn new(ctx: &mut Context) -> Self {
        let buffer = render::UniformBufferBuilder::new()
            .usage(wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST)
            .build(ctx, AppInfoUniform::min_size());

        let bindgroup_layout = render::BindGroupLayoutBuilder::new()
            .entries(vec![
                //
                render::BindGroupLayoutEntry::new()
                    .vertex()
                    .fragment()
                    .compute()
                    .uniform(),
            ])
            .build(ctx);
        let bindgroup = render::BindGroupBuilder::new(bindgroup_layout.clone())
            .entries(vec![render::BindGroupEntry::Buffer(buffer.buffer())])
            .build(ctx);

        Self {
            bindgroup_layout,
            bindgroup,
            buffer,
        }
    }

    pub fn update_buffer(&mut self, ctx: &Context) {
        let uniform = AppInfoUniform {
            time_passed: time::time_since_start(ctx),
        };
        self.buffer.write(ctx, &uniform);
    }

    pub fn buffer(&self) -> render::ArcBuffer {
        self.buffer.buffer()
    }
    pub fn buffer_ref(&self) -> &wgpu::Buffer {
        self.buffer.buffer_ref()
    }
    pub fn bindgroup_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bindgroup_layout
    }
    pub fn bindgroup(&self) -> &wgpu::BindGroup {
        &self.bindgroup
    }
}

#[derive(ShaderType)]
pub struct AppInfoUniform {
    time_passed: f32,
}
