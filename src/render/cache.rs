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

    /// Unique id for each arc handle
    unique_arc_id: u64,
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

            unique_arc_id: 0,
        }
    }

    pub fn next_id(&mut self) -> u64 {
        let id = self.unique_arc_id;
        self.unique_arc_id += 1;
        id
    }
}

/// Clear all caches
pub fn clear_cache(ctx: &mut Context) {
    ctx.render.cache = RenderCache::empty();
}

// TODO: replace with arc::new to avoid manual creation of arcs
pub fn next_id(ctx: &mut Context) -> u64 {
    ctx.render.cache.next_id()
}
