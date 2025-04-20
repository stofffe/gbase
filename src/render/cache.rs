use crate::render::{
    self, ArcBindGroup, ArcBindGroupLayout, ArcComputePipeline, ArcPipelineLayout,
    ArcRenderPipeline, ArcSampler, BindGroupBuilder, BindGroupLayoutBuilder,
    ComputePipelineBuilder, PipelineLayoutBuilder, RenderPipelineBuilder, SamplerBuilder,
    TextureViewBuilder,
};
use crate::Context;
use std::collections::HashMap;

pub struct RenderCache {
    pub bindgroup_layouts: HashMap<BindGroupLayoutBuilder, ArcBindGroupLayout>,
    pub bindgroups: HashMap<BindGroupBuilder, ArcBindGroup>,
    pub pipeline_layouts: HashMap<PipelineLayoutBuilder, ArcPipelineLayout>,
    pub render_pipelines: HashMap<RenderPipelineBuilder, ArcRenderPipeline>,
    pub compute_pipelines: HashMap<ComputePipelineBuilder, ArcComputePipeline>,
    pub samplers: HashMap<SamplerBuilder, ArcSampler>,
    pub texture_views: HashMap<TextureViewBuilder, render::ArcTextureView>,
}

impl RenderCache {
    pub fn empty() -> Self {
        Self {
            bindgroup_layouts: HashMap::new(),
            bindgroups: HashMap::new(),
            pipeline_layouts: HashMap::new(),
            render_pipelines: HashMap::new(),
            compute_pipelines: HashMap::new(),
            samplers: HashMap::new(),
            texture_views: HashMap::new(),
        }
    }
}

/// Clear all caches
pub fn clear_cache(ctx: &mut Context) {
    ctx.render.cache = RenderCache::empty();
}
