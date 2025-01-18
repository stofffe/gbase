use crate::{render, Context};
use render::{
    ArcBindGroupLayout, ArcComputePipeline, ArcPipelineLayout, ArcRenderPipeline, ArcShaderModule,
};

//
// Pipeline layout builder
//

// TODO: add all options
#[derive(Clone, Eq, PartialEq, Hash)]
pub struct PipelineLayoutBuilder {
    label: Option<String>,
    bind_groups: Vec<ArcBindGroupLayout>,
    push_constants: Vec<wgpu::PushConstantRange>,
}

impl PipelineLayoutBuilder {
    pub fn new() -> Self {
        Self {
            label: None,
            bind_groups: Vec::new(),
            push_constants: Vec::new(),
        }
    }

    pub fn build_uncached(&self, ctx: &Context) -> ArcPipelineLayout {
        let device = render::device(ctx);

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: self.label.as_deref(),
            bind_group_layouts: &self
                .bind_groups
                .iter()
                .map(|b| b.as_ref())
                .collect::<Vec<_>>(),
            push_constant_ranges: &self.push_constants,
        });

        ArcPipelineLayout::new(layout)
    }
    pub fn build(&self, ctx: &mut Context) -> ArcPipelineLayout {
        if let Some(pipeline_layout) = ctx.render.cache.pipeline_layouts.get(self) {
            log::info!("Fetch cached pipeline layout");
            return pipeline_layout.clone();
        }

        log::info!("Create cached pipeline layout");
        let pipeline_layout = self.build_uncached(ctx);
        ctx.render
            .cache
            .pipeline_layouts
            .insert(self.clone(), pipeline_layout.clone());
        pipeline_layout
    }
}

impl PipelineLayoutBuilder {
    pub fn label(mut self, value: String) -> Self {
        self.label = Some(value);
        self
    }
    pub fn bind_groups(mut self, value: Vec<ArcBindGroupLayout>) -> Self {
        self.bind_groups = value;
        self
    }
    pub fn push_constants(mut self, value: Vec<wgpu::PushConstantRange>) -> Self {
        self.push_constants = value;
        self
    }
}

//
// Render Pipeline Builder
//

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct ColorTargetState {
    format: wgpu::TextureFormat,
    blend: Option<wgpu::BlendState>,
    write_mask: wgpu::ColorWrites,
}

impl ColorTargetState {
    pub fn new() -> Self {
        Self {
            format: wgpu::TextureFormat::Rgba8Unorm,
            blend: None,
            write_mask: wgpu::ColorWrites::ALL,
        }
    }
    pub fn from_current_screen(ctx: &Context) -> Self {
        Self {
            format: render::surface_config(ctx).format,
            blend: None,
            write_mask: wgpu::ColorWrites::ALL,
        }
    }
    pub fn format(mut self, value: wgpu::TextureFormat) -> Self {
        self.format = value;
        self
    }
    pub fn blend(mut self, value: wgpu::BlendState) -> Self {
        self.blend = Some(value);
        self
    }
    pub fn write_mask(mut self, value: wgpu::ColorWrites) -> Self {
        self.write_mask = value;
        self
    }
}

impl Into<wgpu::ColorTargetState> for ColorTargetState {
    fn into(self) -> wgpu::ColorTargetState {
        wgpu::ColorTargetState {
            format: self.format,
            blend: self.blend,
            write_mask: self.write_mask,
        }
    }
}

#[derive(Clone, Eq, PartialEq, Hash)]
pub struct RenderPipelineBuilder {
    layout: ArcPipelineLayout,
    label: Option<String>,
    shader: ArcShaderModule,
    buffers: Vec<wgpu::VertexBufferLayout<'static>>,
    targets: Vec<Option<ColorTargetState>>,
    topology: wgpu::PrimitiveTopology,
    polygon_mode: wgpu::PolygonMode,
    cull_mode: Option<wgpu::Face>,
    depth_stencil: Option<wgpu::DepthStencilState>,
    vertex_entry_point: Option<String>,
    fragment_entry_point: Option<String>,
}

impl RenderPipelineBuilder {
    pub fn new(shader: ArcShaderModule, layout: ArcPipelineLayout) -> Self {
        Self {
            layout,
            shader,
            buffers: Vec::new(),
            targets: Vec::new(),
            topology: wgpu::PrimitiveTopology::TriangleList,
            polygon_mode: wgpu::PolygonMode::Fill,
            cull_mode: None,
            depth_stencil: None,
            label: None,
            vertex_entry_point: None,
            fragment_entry_point: None,
        }
    }

    pub fn build_uncached(&self, ctx: &Context) -> ArcRenderPipeline {
        let device = render::device(ctx);
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: self.label.as_deref(),
            layout: Some(&self.layout),
            vertex: wgpu::VertexState {
                module: &self.shader,
                entry_point: self.vertex_entry_point.as_deref(),
                buffers: &self.buffers,
                compilation_options: wgpu::PipelineCompilationOptions::default(), // TODO look into these options
            },
            fragment: Some(wgpu::FragmentState {
                module: &self.shader,
                entry_point: self.fragment_entry_point.as_deref(),
                targets: &self
                    .targets
                    .iter()
                    .map(|state| state.clone().map(|a| a.into()))
                    .collect::<Vec<_>>(),
                compilation_options: wgpu::PipelineCompilationOptions::default(), // TODO look into these options
            }),
            primitive: wgpu::PrimitiveState {
                topology: self.topology,
                polygon_mode: self.polygon_mode,
                cull_mode: self.cull_mode,
                front_face: wgpu::FrontFace::Ccw, // Right handed coordinate system
                strip_index_format: None,
                conservative: false,
                unclipped_depth: false,
            },
            depth_stencil: self.depth_stencil.clone(),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        ArcRenderPipeline::new(pipeline)
    }
    pub fn build(&self, ctx: &mut Context) -> ArcRenderPipeline {
        if let Some(render_pipeline) = ctx.render.cache.render_pipelines.get(self) {
            log::info!("Fetch cached render pipeline");
            return render_pipeline.clone();
        }

        log::info!("Create cached render pipeline");
        let render_pipeline = self.build_uncached(ctx);
        ctx.render
            .cache
            .render_pipelines
            .insert(self.clone(), render_pipeline.clone());
        render_pipeline
    }
}

impl RenderPipelineBuilder {
    pub fn label(mut self, value: String) -> Self {
        self.label = Some(value);
        self
    }
    pub fn buffers(mut self, value: Vec<wgpu::VertexBufferLayout<'static>>) -> Self {
        self.buffers = value;
        self
    }
    // TODO: make this custom type? (with empty)
    pub fn multiple_targets(mut self, value: Vec<Option<ColorTargetState>>) -> Self {
        self.targets = value;
        self
    }
    pub fn single_target(mut self, value: ColorTargetState) -> Self {
        self.targets = vec![Some(value)];
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
    pub fn vertex_entry_point(mut self, value: String) -> Self {
        self.vertex_entry_point = Some(value);
        self
    }
    pub fn fragment_entry_point(mut self, value: String) -> Self {
        self.fragment_entry_point = Some(value);
        self
    }

    // TODO if targets empty use this instead
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

#[derive(Clone, Eq, PartialEq, Hash)]
pub struct ComputePipelineBuilder {
    layout: ArcPipelineLayout,
    shader: ArcShaderModule,
    label: Option<String>,
    entry_point: Option<String>,
}

impl ComputePipelineBuilder {
    pub fn new(shader: ArcShaderModule, layout: ArcPipelineLayout) -> Self {
        Self {
            layout,
            shader,
            label: None,
            entry_point: None,
        }
    }

    pub fn build_uncached(&self, ctx: &Context) -> ArcComputePipeline {
        let device = render::device(ctx);

        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: self.label.as_deref(),
            layout: Some(&self.layout),
            module: &self.shader,
            entry_point: self.entry_point.as_deref(),
            compilation_options: wgpu::PipelineCompilationOptions::default(), // TODO look into these options
            cache: None,
        });

        ArcComputePipeline::new(pipeline)
    }

    pub fn build(&self, ctx: &mut Context) -> ArcComputePipeline {
        if let Some(compute_pipeline) = ctx.render.cache.compute_pipeline.get(self) {
            log::info!("Fetch cached compute pipeline");
            return compute_pipeline.clone();
        }

        log::info!("Create cached compute pipeline");
        let compute_pipeline = self.build_uncached(ctx);
        ctx.render
            .cache
            .compute_pipeline
            .insert(self.clone(), compute_pipeline.clone());
        compute_pipeline
    }
}

impl ComputePipelineBuilder {
    pub fn label(mut self, value: String) -> Self {
        self.label = Some(value);
        self
    }
    pub fn entry_point(mut self, value: impl Into<String>) -> Self {
        self.entry_point = Some(value.into());
        self
    }
}
