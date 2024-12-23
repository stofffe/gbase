use super::ArcShaderModule;
use crate::{render, Context};

//
// Shader Builder
//

#[derive(Clone, Eq, PartialEq, Hash)]
pub struct ShaderBuilder {
    source: String,
    label: Option<String>,
}

impl ShaderBuilder {
    pub fn new(source: String) -> Self {
        Self {
            source,
            label: None,
        }
    }

    pub fn build_uncached(&self, ctx: &Context) -> ArcShaderModule {
        let device = render::device(ctx);
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: self.label.as_deref(),
            source: wgpu::ShaderSource::Wgsl(self.source.clone().into()),
        });

        ArcShaderModule::new(module)
    }

    pub fn build(&self, ctx: &mut Context) -> ArcShaderModule {
        if let Some(shader) = ctx.render.cache.shaders.get(self) {
            log::info!("Fetch cached shader");
            return shader.clone();
        }

        log::info!("Create cached shader");
        let shader = self.build_uncached(ctx);
        ctx.render
            .cache
            .shaders
            .insert(self.clone(), shader.clone());
        shader
    }
}

impl ShaderBuilder {
    pub fn label(mut self, value: String) -> Self {
        self.label = Some(value);
        self
    }
}
