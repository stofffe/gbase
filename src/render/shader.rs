use super::{ArcHandle, ArcShaderModule};
use crate::{arc, Context};

//
// Shader Builder
//

#[derive(Debug)]
pub struct Shader {
    pub source: String,
}

impl Shader {
    pub fn new(source: impl Into<String>) -> Self {
        Self {
            source: source.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct ShaderBuilder {
    pub label: Option<String>,
}

impl ShaderBuilder {
    pub fn new() -> Self {
        Self { label: None }
    }

    /// Create shader module without error checking
    ///
    /// Invalid wgsl code will cause a panic
    pub fn build(&self, device: &wgpu::Device, source: impl Into<String>) -> wgpu::ShaderModule {
        let source = source.into();

        let mut shader_code = String::with_capacity(source.len());

        shader_code.push_str(&source);

        device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: self.label.as_deref(),
            source: wgpu::ShaderSource::Wgsl(shader_code.into()),
        })
    }

    /// Create shader module
    pub async fn build_err(
        &self,
        device: &wgpu::Device,
        source: impl Into<String>,
    ) -> Result<wgpu::ShaderModule, wgpu::Error> {
        device.push_error_scope(wgpu::ErrorFilter::Validation);

        let module = self.build(device, source);

        if let Some(err) = device.pop_error_scope().await {
            Err(err)
        } else {
            Ok(module)
        }
    }

    /// Create shader module without error checking
    ///
    /// Automatically create a ArcHandle
    ///
    /// Invalid wgsl code will cause a panic
    pub fn build_arc_handle(&self, ctx: &Context, source: impl Into<String>) -> ArcShaderModule {
        let source = source.into();

        let mut shader_code = String::with_capacity(source.len());

        shader_code.push_str(&source);

        let device = &ctx.render.device;
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: self.label.as_deref(),
            source: wgpu::ShaderSource::Wgsl(shader_code.into()),
        });

        ArcHandle::new(arc::runtime(ctx), module)
    }

    /// Create shader module
    ///
    /// Automatically create a ArcHandle
    pub async fn build_arc_handle_err(
        &self,
        ctx: &Context,
        source: impl Into<String>,
    ) -> Result<ArcShaderModule, wgpu::Error> {
        let device = &ctx.render.device;
        device.push_error_scope(wgpu::ErrorFilter::Validation);

        let module = self.build_arc_handle(ctx, source);

        if let Some(err) = device.pop_error_scope().await {
            Err(err)
        } else {
            Ok(module)
        }
    }
}

impl ShaderBuilder {
    pub fn label(mut self, value: String) -> Self {
        self.label = Some(value);
        self
    }
}
