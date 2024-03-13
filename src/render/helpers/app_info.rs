use encase::ShaderType;

use crate::{time, Context};

use super::ArcHandle;

/// Holds information about the app
///
/// Can easily be sent as uniform to shaders
pub struct AppInfo {
    bindgroup_layout: super::BindGroupLayout,
    bindgroup: super::BindGroup,
    buffer: super::UniformBuffer,
}

impl AppInfo {
    pub fn new(ctx: &Context) -> Self {
        let buffer = super::UniformBufferBuilder::new()
            .usage(wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST)
            .build(ctx, AppInfoUniform::min_size());

        let (bindgroup_layout, bindgroup) = super::BindGroupCombinedBuilder::new()
            .entries(&[
                super::BindGroupCombinedEntry::new(buffer.buf().as_entire_binding())
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

    pub fn bindgroup_layout_handle(&self) -> ArcHandle<wgpu::BindGroupLayout> {
        self.bindgroup_layout.clone()
    }
    pub fn bindgroup_handle(&self) -> ArcHandle<wgpu::BindGroup> {
        self.bindgroup.clone()
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
