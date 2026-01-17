use crate::{
    render::{
        ArcHandle, BindGroupBuilder, BindGroupLayoutBuilder, ComputePipelineBuilder,
        PipelineLayoutBuilder, RenderPipelineBuilder, SamplerBuilder, TextureViewBuilder,
    },
    Context,
};
use rustc_hash::FxHashMap;
use std::sync::Arc;

pub struct RenderCache {
    pub bindgroup_layouts: FxHashMap<BindGroupLayoutBuilder, ArcHandle<wgpu::BindGroupLayout>>,
    pub bindgroups: FxHashMap<BindGroupBuilder, ArcHandle<wgpu::BindGroup>>,
    pub pipeline_layouts: FxHashMap<PipelineLayoutBuilder, ArcHandle<wgpu::PipelineLayout>>,
    pub render_pipelines: FxHashMap<RenderPipelineBuilder, ArcHandle<wgpu::RenderPipeline>>,
    pub compute_pipelines: FxHashMap<ComputePipelineBuilder, ArcHandle<wgpu::ComputePipeline>>,
    pub samplers: FxHashMap<SamplerBuilder, ArcHandle<wgpu::Sampler>>,
    pub texture_views: FxHashMap<TextureViewBuilder, ArcHandle<wgpu::TextureView>>,

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

    pub fn clear_all(&mut self) {
        self.bindgroup_layouts.clear();
        self.bindgroups.clear();
        self.pipeline_layouts.clear();
        self.render_pipelines.clear();
        self.compute_pipelines.clear();
        self.samplers.clear();
        self.texture_views.clear();
    }

    pub fn clear_unused(&mut self) {
        self.bindgroup_layouts
            .retain(|_, handle| Arc::strong_count(&handle.handle) > 1);
        self.bindgroups
            .retain(|_, handle| Arc::strong_count(&handle.handle) > 1);
        self.pipeline_layouts
            .retain(|_, handle| Arc::strong_count(&handle.handle) > 1);
        self.render_pipelines
            .retain(|_, handle| Arc::strong_count(&handle.handle) > 1);
        self.compute_pipelines
            .retain(|_, handle| Arc::strong_count(&handle.handle) > 1);
        self.samplers
            .retain(|_, handle| Arc::strong_count(&handle.handle) > 1);
        self.texture_views
            .retain(|_, handle| Arc::strong_count(&handle.handle) > 1);
    }

    pub(crate) fn next_id(&mut self) -> u64 {
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
