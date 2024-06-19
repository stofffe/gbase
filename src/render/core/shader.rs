use crate::{render, Context};

//
// Shader Builder
//

pub struct ShaderBuilder<'a> {
    label: Option<&'a str>,
}

impl<'a> ShaderBuilder<'a> {
    pub fn new() -> Self {
        Self { label: None }
    }

    pub fn build(&self, ctx: &Context, source: &'a str) -> wgpu::ShaderModule {
        let device = render::device(ctx);
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: self.label,
            source: wgpu::ShaderSource::Wgsl(source.into()),
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
