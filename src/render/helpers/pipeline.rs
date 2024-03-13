use crate::{render, Context};

use super::{ArcHandle, ArcShaderModule, BindGroupLayout};

//
// Render Pipeline Builder
//

pub type RenderPipeline = ArcHandle<wgpu::RenderPipeline>;

#[derive(Hash)]
pub struct RenderPipelineBuilder<'a> {
    label: Option<&'a str>,

    shader: ArcShaderModule,
    bind_groups: &'a [BindGroupLayout],
    buffers: &'a [wgpu::VertexBufferLayout<'a>],

    targets: &'a [Option<wgpu::ColorTargetState>],

    topology: wgpu::PrimitiveTopology,
    polygon_mode: wgpu::PolygonMode,
    cull_mode: Option<wgpu::Face>,

    depth_stencil: Option<wgpu::DepthStencilState>,
}

impl<'a> RenderPipelineBuilder<'a> {
    pub fn new(shader: ArcShaderModule) -> Self {
        Self {
            shader,
            label: None,
            bind_groups: &[],
            buffers: &[],
            targets: &[],
            topology: wgpu::PrimitiveTopology::TriangleList,
            polygon_mode: wgpu::PolygonMode::Fill,
            cull_mode: None,
            depth_stencil: None,
        }
    }

    pub fn build(self, ctx: &Context) -> RenderPipeline {
        let device = render::device(ctx);

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: self.label,
            bind_group_layouts: &self
                .bind_groups
                .iter()
                .map(|r| r.as_ref())
                .collect::<Vec<_>>(),
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: self.label,
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &self.shader,
                entry_point: "vs_main",
                buffers: self.buffers,
            },
            fragment: Some(wgpu::FragmentState {
                module: &self.shader,
                entry_point: "fs_main",
                targets: self.targets,
            }),
            primitive: wgpu::PrimitiveState {
                topology: self.topology,
                polygon_mode: self.polygon_mode,
                cull_mode: None,
                front_face: wgpu::FrontFace::Ccw, // Right handed coordinate system
                strip_index_format: None,
                conservative: false,
                unclipped_depth: false,
            },
            depth_stencil: self.depth_stencil,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        ArcHandle::new(pipeline)
    }
}

impl<'a> RenderPipelineBuilder<'a> {
    pub fn label(mut self, value: &'a str) -> Self {
        self.label = Some(value);
        self
    }
    pub fn bind_groups(mut self, value: &'a [BindGroupLayout]) -> Self {
        self.bind_groups = value;
        self
    }
    pub fn buffers(mut self, value: &'a [wgpu::VertexBufferLayout<'a>]) -> Self {
        self.buffers = value;
        self
    }
    pub fn targets(mut self, value: &'a [Option<wgpu::ColorTargetState>]) -> Self {
        self.targets = value;
        self
    }
    pub fn depth_stencil(mut self, value: wgpu::DepthStencilState) -> Self {
        self.depth_stencil = Some(value);
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
    pub fn cull_mode(mut self, value: wgpu::Face) -> Self {
        self.cull_mode = Some(value);
        self
    }

    pub fn default_target(ctx: &Context) -> Option<wgpu::ColorTargetState> {
        let surface_config = render::surface_config(ctx);
        Some(wgpu::ColorTargetState {
            format: surface_config.format,
            blend: None,
            write_mask: wgpu::ColorWrites::ALL,
        })
    }
}

//
// Compute Pipeline Builder
//

pub type ComputePipeline = ArcHandle<wgpu::ComputePipeline>;

#[derive(Hash)]
pub struct ComputePipelineBuilder<'a> {
    label: Option<&'a str>,

    shader: ArcShaderModule,
    bind_groups: &'a [BindGroupLayout],
}

impl<'a> ComputePipelineBuilder<'a> {
    pub fn new(shader: ArcShaderModule) -> Self {
        Self {
            shader,
            label: None,
            bind_groups: &[],
        }
    }

    pub fn build(self, ctx: &Context) -> ComputePipeline {
        let device = render::device(ctx);

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: self.label,
            bind_group_layouts: &self
                .bind_groups
                .iter()
                .map(|r| r.as_ref())
                .collect::<Vec<_>>(),
            push_constant_ranges: &[],
        });

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: self.label,
            layout: Some(&layout),
            module: &self.shader,
            entry_point: "cs_main",
        });

        ArcHandle::new(pipeline)
    }
}

impl<'a> ComputePipelineBuilder<'a> {
    pub fn label(mut self, value: &'a str) -> Self {
        self.label = Some(value);
        self
    }
    pub fn bind_groups(mut self, value: &'a [BindGroupLayout]) -> Self {
        self.bind_groups = value;
        self
    }
}
