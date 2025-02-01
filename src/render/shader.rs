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
    pub fn new(source: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            label: None,
        }
    }

    /// Create shader module
    ///
    /// panics if source is invalid
    pub fn build_uncached(&self, ctx: &Context) -> ArcShaderModule {
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
    /// Checks cache before creating new
    ///
    /// panics if source is invalid
    pub fn build(self, ctx: &mut Context) -> ArcShaderModule {
        if let Some(shader) = ctx.render.cache.shaders.get(&self) {
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

    /// Create shader module
    ///
    /// Checks cache before creating new
    ///
    /// Not supported on WASM (blocking call)
    #[cfg(not(target_arch = "wasm32"))]
    pub fn build_err(&self, ctx: &mut Context) -> Result<ArcShaderModule, wgpu::Error> {
        if let Some(shader) = ctx.render.cache.shaders.get(self) {
            log::info!("Fetch cached shader");
            return Ok(shader.clone());
        }

        log::info!("Create cached shader");
        let shader = self.build_unchached_err(ctx)?;
        ctx.render
            .cache
            .shaders
            .insert(self.clone(), shader.clone());
        Ok(shader)
    }

    /// Create shader module
    ///
    /// Not supported on WASM (blocking call)
    #[cfg(not(target_arch = "wasm32"))]
    pub fn build_unchached_err(&self, ctx: &mut Context) -> Result<ArcShaderModule, wgpu::Error> {
        let device = render::device(ctx);
        device.push_error_scope(wgpu::ErrorFilter::Validation);
        let shader = self.build_uncached(ctx);
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
}
