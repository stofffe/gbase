use encase::ShaderType;
use gbase::{render, time, wgpu, Context};

/// Holds information about the app
///
/// Can easily be sent as uniform to shaders
pub struct AppInfo {
    bindgroup_layout: render::ArcBindGroupLayout,
    bindgroup: render::ArcBindGroup,
    buffer: render::UniformBuffer<AppInfoUniform>,
}

impl AppInfo {
    pub fn new(ctx: &mut Context) -> Self {
        let buffer = render::UniformBufferBuilder::new()
            .label("app info")
            .usage(wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST)
            .build(ctx);

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
            screen_width: render::surface_size(ctx).width,
            screen_height: render::surface_size(ctx).height,
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
    screen_width: u32,
    screen_height: u32,
}

impl AppInfoUniform {
    pub fn new(ctx: &Context) -> Self {
        Self {
            time_passed: time::time_since_start(ctx),
            screen_width: render::surface_size(ctx).width,
            screen_height: render::surface_size(ctx).height,
        }
    }
}
