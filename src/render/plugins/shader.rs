use crate::{filesystem, render, Context};
use std::path::Path;

pub struct ShaderBuilder<'a> {
    label: Option<String>,
    source: String,
    vs_entry: String,
    fs_entry: String,
    cs_entry: String,
    buffers: Vec<wgpu::VertexBufferLayout<'static>>,
    targets: Vec<Option<wgpu::ColorTargetState>>,
    bind_group_layouts: Vec<&'a wgpu::BindGroupLayout>,
}

impl<'a> ShaderBuilder<'a> {
    pub fn new(source: impl Into<String>) -> Self {
        Self {
            label: None,
            source: source.into(),
            vs_entry: "vs_main".to_string(),
            fs_entry: "fs_main".to_string(),
            cs_entry: "cs_main".to_string(),
            buffers: Vec::new(),
            targets: Vec::new(),
            bind_group_layouts: Vec::new(),
        }
    }
    pub async fn build(self, ctx: &'a Context) -> Shader {
        let device = render::device(ctx);
        let shader_str = filesystem::load_string(ctx, Path::new(&self.source))
            .await
            .unwrap();
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: self.label.as_deref(),
            source: wgpu::ShaderSource::Wgsl(shader_str.into()),
        });
        Shader {
            module,
            vs_entry: self.vs_entry,
            fs_entry: self.fs_entry,
            cs_entry: self.cs_entry,
            buffers: self.buffers,
            targets: self.targets,
            bind_group_layouts: self.bind_group_layouts,
        }
    }

    pub fn label(mut self, value: impl Into<String>) -> Self {
        self.label = Some(value.into());
        self
    }
    pub fn vs_entry(mut self, value: impl Into<String>) -> Self {
        self.vs_entry = value.into();
        self
    }
    pub fn fs_entry(mut self, value: impl Into<String>) -> Self {
        self.fs_entry = value.into();
        self
    }
    pub fn cs_entry(mut self, value: impl Into<String>) -> Self {
        self.cs_entry = value.into();
        self
    }
    pub fn buffers(mut self, value: Vec<wgpu::VertexBufferLayout<'static>>) -> Self {
        self.buffers = value;
        self
    }
    pub fn targets(mut self, value: Vec<Option<wgpu::ColorTargetState>>) -> Self {
        self.targets = value;
        self
    }
    pub fn default_target(mut self, surface_config: &wgpu::SurfaceConfiguration) -> Self {
        self.targets = vec![Some(wgpu::ColorTargetState {
            format: surface_config.format,
            blend: None,
            write_mask: wgpu::ColorWrites::ALL,
        })];
        self
    }
    pub fn bind_group_layouts(mut self, value: Vec<&'a wgpu::BindGroupLayout>) -> Self {
        self.bind_group_layouts = value;
        self
    }
}

pub struct Shader<'a> {
    module: wgpu::ShaderModule,

    vs_entry: String,
    buffers: Vec<wgpu::VertexBufferLayout<'static>>,

    fs_entry: String,
    targets: Vec<Option<wgpu::ColorTargetState>>,

    cs_entry: String,

    bind_group_layouts: Vec<&'a wgpu::BindGroupLayout>,
}

impl<'a> Shader<'a> {
    pub fn module(&self) -> &wgpu::ShaderModule {
        &self.module
    }
    pub fn bind_group_layouts(&self) -> &[&wgpu::BindGroupLayout] {
        &self.bind_group_layouts
    }
    pub fn vertex(&self) -> wgpu::VertexState<'_> {
        wgpu::VertexState {
            module: &self.module,
            entry_point: &self.vs_entry,
            buffers: &self.buffers,
        }
    }
    pub fn fragment(&self) -> Option<wgpu::FragmentState> {
        Some(wgpu::FragmentState {
            module: &self.module,
            entry_point: &self.fs_entry,
            targets: &self.targets,
        })
    }
    pub fn cs_entry(&self) -> &str {
        &self.cs_entry
    }
}
