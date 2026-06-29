use super::{ArcHandle, ArcShaderModule};
use crate::{
    render::{self, next_id},
    Context,
};

//
// Shader Builder
//

#[derive(Debug)]
pub struct Shader {
    pub source: String,
    pub config: ShaderBuilder,
}

impl Shader {
    pub fn new(source: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            config: ShaderBuilder::new(),
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

    /// Create shader module
    ///
    /// panics if source is invalid
    pub fn build(&self, ctx: &mut Context, source: impl Into<String>) -> ArcShaderModule {
        ArcHandle::new(ctx, self.build_non_arc(ctx, source.into()))
    }

    /// Create shader module
    ///
    /// Not supported on WASM (blocking call)
    #[cfg(not(target_arch = "wasm32"))]
    pub fn build_err(
        &self,
        ctx: &mut Context,
        source: impl Into<String>,
    ) -> Result<ArcShaderModule, wgpu::Error> {
        self.build_err_non_arc(ctx, source.into())
            .map(|module| ArcHandle::new(ctx, module))
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn build_err_non_arc(
        &self,
        ctx: &Context,
        source: String,
    ) -> Result<wgpu::ShaderModule, wgpu::Error> {
        let device = render::device(ctx);
        device.push_error_scope(wgpu::ErrorFilter::Validation);
        let shader = self.build_non_arc(ctx, source);
        pollster::block_on(async {
            if let Some(err) = device.pop_error_scope().await {
                Err(err)
            } else {
                Ok(shader)
            }
        })
    }

    pub(crate) fn build_non_arc(&self, ctx: &Context, source: String) -> wgpu::ShaderModule {
        let device = render::device(ctx);
        let mut shader_code = String::with_capacity(source.len());

        shader_code.push_str(&source);

        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: self.label.as_deref(),
            source: wgpu::ShaderSource::Wgsl(shader_code.into()),
        });

        module
    }
}

impl ShaderBuilder {
    pub fn label(mut self, value: String) -> Self {
        self.label = Some(value);
        self
    }
}
