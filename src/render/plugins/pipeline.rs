use crate::{render, Context};

// TODO more references?
pub struct RenderPipelineBuilder<'a> {
    shader: &'a render::Shader<'a>,

    label: Option<String>,
    topology: wgpu::PrimitiveTopology,
    cull_mode: Option<wgpu::Face>,
    polygon_mode: wgpu::PolygonMode,
    depth_buffer: Option<wgpu::DepthStencilState>,
}

impl<'a> RenderPipelineBuilder<'a> {
    pub fn label(mut self, value: impl Into<String>) -> Self {
        self.label = Some(value.into());
        self
    }
    pub fn topology(mut self, value: wgpu::PrimitiveTopology) -> Self {
        self.topology = value;
        self
    }
    pub fn polygon_mode(mut self, value: wgpu::PolygonMode) -> Self {
        self.polygon_mode = value;
        self
    }
    pub fn cull_mode(mut self, value: Option<wgpu::Face>) -> Self {
        self.cull_mode = value;
        self
    }
    pub fn depth_buffer(mut self, value: wgpu::DepthStencilState) -> Self {
        self.depth_buffer = Some(value);
        self
    }

    pub fn new(shader: &'a render::Shader) -> Self {
        Self {
            label: None,
            topology: wgpu::PrimitiveTopology::TriangleList,
            shader,
            cull_mode: None,
            polygon_mode: wgpu::PolygonMode::Fill,
            depth_buffer: None,
        }
    }

    pub fn build(self, ctx: &Context) -> RenderPipeline {
        let device = render::device(ctx);
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: self.label.as_deref(),
            bind_group_layouts: self.shader.bind_group_layouts(),
            push_constant_ranges: &[], // TODO
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: self.label.as_deref(),
            layout: Some(&pipeline_layout),
            vertex: self.shader.vertex(),
            fragment: self.shader.fragment(),
            primitive: wgpu::PrimitiveState {
                topology: self.topology,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw, // right handed coordinate system
                cull_mode: self.cull_mode,
                polygon_mode: self.polygon_mode,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: self.depth_buffer,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });
        RenderPipeline { pipeline }
    }
}

pub struct RenderPipeline {
    pipeline: wgpu::RenderPipeline,
}

impl RenderPipeline {
    pub fn pipeline(&self) -> &wgpu::RenderPipeline {
        &self.pipeline
    }
}

pub struct ComputePipelineBuilder<'a> {
    shader: &'a render::Shader<'a>,

    label: Option<String>,
}

impl<'a> ComputePipelineBuilder<'a> {
    pub fn new(shader: &'a render::Shader) -> Self {
        Self {
            shader,
            label: None,
        }
    }

    pub fn build(self, ctx: &Context) -> ComputePipeline {
        let device = render::device(ctx);
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: self.label.as_deref(),
            bind_group_layouts: self.shader.bind_group_layouts(),
            push_constant_ranges: &[],
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: self.label.as_deref(),
            layout: Some(&layout),
            module: self.shader.module(),
            entry_point: self.shader.cs_entry(),
        });
        ComputePipeline { pipeline }
    }

    pub fn label(mut self, value: &str) -> Self {
        self.label = Some(value.to_string());
        self
    }
}

pub struct ComputePipeline {
    pipeline: wgpu::ComputePipeline,
}

impl ComputePipeline {
    pub fn pipeline(&self) -> &wgpu::ComputePipeline {
        &self.pipeline
    }
}
