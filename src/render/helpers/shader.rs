use crate::{render, Context};

//
// Shader Builder
//

pub struct ShaderBuilder<'a> {
    source: &'a str,
    label: Option<&'a str>,
}

impl<'a> ShaderBuilder<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            source,
            label: None,
        }
    }

    pub fn build(&self, ctx: &Context) -> wgpu::ShaderModule {
        let device = render::device(ctx);
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: self.label,
            source: wgpu::ShaderSource::Wgsl(self.source.into()),
        });

        module
    }
}

impl<'a> ShaderBuilder<'a> {
    pub fn label(mut self, value: &'a str) -> Self {
        self.label = Some(value);
        self
    }
}