use super::ArcShaderModule;
use crate::Context;

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
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn build_inner_err_2(
        &self,
        device: &wgpu::Device,
    ) -> Result<wgpu::ShaderModule, wgpu::Error> {
        device.push_error_scope(wgpu::ErrorFilter::Validation);
        let shader = self.build_inner_2(device);
        pollster::block_on(async {
            if let Some(err) = device.pop_error_scope().await {
                Err(err)
            } else {
                Ok(shader)
            }
        })
    }

    pub(crate) fn build_inner_2(&self, device: &wgpu::Device) -> wgpu::ShaderModule {
        let mut shader_code = String::with_capacity(self.source.len());

        shader_code.push_str(&self.source);

        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: self.label.as_deref(),
            source: wgpu::ShaderSource::Wgsl(shader_code.into()),
        });

        module
    }

    pub(crate) fn build_inner(&self, device: &wgpu::Device) -> ArcShaderModule {
        let mut shader_code = String::with_capacity(self.source.len());

        shader_code.push_str(&self.source);

        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: self.label.as_deref(),
            source: wgpu::ShaderSource::Wgsl(shader_code.into()),
        });

        ArcShaderModule::new(module)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn build_inner_err(
        &self,
        device: &wgpu::Device,
    ) -> Result<ArcShaderModule, wgpu::Error> {
        device.push_error_scope(wgpu::ErrorFilter::Validation);
        let shader = self.build_inner(device);
        pollster::block_on(async {
            if let Some(err) = device.pop_error_scope().await {
                Err(err)
            } else {
                Ok(shader)
            }
        })
    }

    /// Create shader module
    ///
    /// panics if source is invalid
    pub fn build(&self, ctx: &Context) -> ArcShaderModule {
        self.build_inner(&ctx.render.device)
    }

    /// Create shader module
    ///
    /// Not supported on WASM (blocking call)
    #[cfg(not(target_arch = "wasm32"))]
    pub fn build_err(&self, ctx: &mut Context) -> Result<ArcShaderModule, wgpu::Error> {
        self.build_inner_err(&ctx.render.device)
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
