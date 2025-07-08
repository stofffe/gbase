use crate::render::{
    self, ArcBindGroup, ArcBindGroupLayout, ArcComputePipeline, ArcPipelineLayout,
    ArcRenderPipeline, ArcSampler, BindGroupBuilder, BindGroupLayoutBuilder,
    ComputePipelineBuilder, PipelineLayoutBuilder, RenderPipelineBuilder, SamplerBuilder,
    TextureViewBuilder,
};
use crate::Context;
use rustc_hash::FxHashMap;

pub struct RenderCache {
    pub bindgroup_layouts: FxHashMap<BindGroupLayoutBuilder, ArcBindGroupLayout>,
    pub bindgroups: FxHashMap<BindGroupBuilder, ArcBindGroup>,
    pub pipeline_layouts: FxHashMap<PipelineLayoutBuilder, ArcPipelineLayout>,
    pub render_pipelines: FxHashMap<RenderPipelineBuilder, ArcRenderPipeline>,
    pub compute_pipelines: FxHashMap<ComputePipelineBuilder, ArcComputePipeline>,
    pub samplers: FxHashMap<SamplerBuilder, ArcSampler>,
    pub texture_views: FxHashMap<TextureViewBuilder, render::ArcTextureView>,
}

impl RenderCache {
    pub fn empty() -> Self {
        Self {
            bindgroup_layouts: FxHashMap::default(),
            bindgroups: FxHashMap::default(),
            pipeline_layouts: FxHashMap::default(),
            render_pipelines: FxHashMap::default(),
            compute_pipelines: FxHashMap::default(),
            samplers: FxHashMap::default(),
            texture_views: FxHashMap::default(),
        }
    }
}

/// Clear all caches
pub fn clear_cache(ctx: &mut Context) {
    ctx.render.cache = RenderCache::empty();
}
