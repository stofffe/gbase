use super::ArcShaderModule;
use crate::{render, Context};

//
// Shader Builder
//

#[derive(Clone, Eq, PartialEq, Hash)]
pub struct ShaderBuilder {
    pub label: Option<String>,
    pub source: String,
}

impl ShaderBuilder {
    pub fn new(source: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            label: None,
        }
    }

    /// Create shader module
    ///
    /// panics if source is invalid
    pub fn build(&self, ctx: &Context) -> ArcShaderModule {
        let mut shader_code = String::with_capacity(self.source.len());

        shader_code.push_str(&self.source);

        let device = render::device(ctx);
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: self.label.as_deref(),
            source: wgpu::ShaderSource::Wgsl(shader_code.into()),
        });

        ArcShaderModule::new(module)
    }

    /// Create shader module
    ///
    /// Not supported on WASM (blocking call)
    #[cfg(not(target_arch = "wasm32"))]
    pub fn build_err(&self, ctx: &mut Context) -> Result<ArcShaderModule, wgpu::Error> {
        let device = render::device(ctx);
        device.push_error_scope(wgpu::ErrorFilter::Validation);
        let shader = self.build(ctx);
        pollster::block_on(async {
            let device = render::device(ctx);
            if let Some(err) = device.pop_error_scope().await {
                Err(err)
            } else {
                Ok(shader)
            }
        })
    }
}

impl ShaderBuilder {
    pub fn label(mut self, value: String) -> Self {
        self.label = Some(value);
        self
    }
    pub fn source(mut self, value: String) -> Self {
        self.source = value;
        self
    }
}
