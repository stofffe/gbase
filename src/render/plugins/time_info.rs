use encase::ShaderType;

pub struct TimeInfo {
    pub time_passed: f32, // TODO getters and setters?

    pub bind_group_layout: wgpu::BindGroupLayout,
    pub bind_group: wgpu::BindGroup,
    pub buffer: wgpu::Buffer,
}

impl TimeInfo {
    pub fn new(device: &wgpu::Device) -> Self {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("app info buffer"),
            size: u64::from(AppInfoUniform::min_size()),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("app info bind group layout"),
            entries: &[
                // app info
                wgpu::BindGroupLayoutEntry {
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
                },
            ],
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("app info bindgroup"),
            layout: &bind_group_layout,
            entries: &[
                // app info
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer.as_entire_binding(),
                },
            ],
        });
        Self {
            time_passed: 0.0,
            bind_group_layout,
            bind_group,
            buffer,
        }
    }

    pub fn update_buffer(&mut self, queue: &wgpu::Queue) {
        let mut buffer = encase::UniformBuffer::new(Vec::new());
        buffer
            .write(&self.uniform())
            .expect("could not write to camera buffer");
        queue.write_buffer(&self.buffer, 0, &buffer.into_inner());
    }

    fn uniform(&self) -> AppInfoUniform {
        AppInfoUniform {
            time_passed: self.time_passed,
        }
    }
}

#[derive(ShaderType)]
pub struct AppInfoUniform {
    time_passed: f32,
}
