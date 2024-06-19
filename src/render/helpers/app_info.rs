use encase::ShaderType;

use crate::{render, time, Context};

/// Holds information about the app
///
/// Can easily be sent as uniform to shaders
pub struct AppInfo {
    bindgroup_layout: wgpu::BindGroupLayout,
    bindgroup: wgpu::BindGroup,
    buffer: render::UniformBuffer,
}

impl AppInfo {
    pub fn new(ctx: &Context) -> Self {
        let buffer = render::UniformBufferBuilder::new()
            .usage(wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST)
            .build(ctx, AppInfoUniform::min_size());

        let (bindgroup_layout, bindgroup) = render::BindGroupCombinedBuilder::new()
            .entries(&[
                render::BindGroupCombinedEntry::new(buffer.buf().as_entire_binding())
                    .visibility(wgpu::ShaderStages::VERTEX_FRAGMENT | wgpu::ShaderStages::COMPUTE)
                    .uniform(),
            ])
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

    pub fn buffer(&self) -> &wgpu::Buffer {
        self.buffer.buf()
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
