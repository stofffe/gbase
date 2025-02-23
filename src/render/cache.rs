use crate::{
    render::{
        self, ArcBindGroup, ArcBindGroupLayout, ArcComputePipeline, ArcPipelineLayout,
        ArcRenderPipeline, ArcSampler, ArcShaderModule, ArcTexture, BindGroupBuilder,
        BindGroupLayoutBuilder, ComputePipelineBuilder, PipelineLayoutBuilder,
        RenderPipelineBuilder, SamplerBuilder, ShaderBuilder, TextureBuilder, TextureViewBuilder,
    },
    Context,
};
use std::collections::HashMap;

pub struct RenderCache {
    pub shaders: HashMap<ShaderBuilder, ArcShaderModule>,
    pub bindgroup_layouts: HashMap<BindGroupLayoutBuilder, ArcBindGroupLayout>,
    pub bindgroups: HashMap<BindGroupBuilder, ArcBindGroup>,
    pub pipeline_layouts: HashMap<PipelineLayoutBuilder, ArcPipelineLayout>,
    pub render_pipelines: HashMap<RenderPipelineBuilder, ArcRenderPipeline>,
    pub compute_pipeline: HashMap<ComputePipelineBuilder, ArcComputePipeline>,
    pub samplers: HashMap<SamplerBuilder, ArcSampler>,
    pub textures: HashMap<TextureBuilder, ArcTexture>,
    pub texture_views: HashMap<TextureViewBuilder, render::ArcTextureView>,
}

impl RenderCache {
    pub fn empty() -> Self {
        Self {
            shaders: HashMap::new(),
            bindgroup_layouts: HashMap::new(),
            bindgroups: HashMap::new(),
            pipeline_layouts: HashMap::new(),
            render_pipelines: HashMap::new(),
            compute_pipeline: HashMap::new(),
            samplers: HashMap::new(),
            textures: HashMap::new(),
            texture_views: HashMap::new(),
        }
    }
}

/// Clear all caches
pub fn clear_cache(ctx: &mut Context) {
    ctx.render.cache = RenderCache::empty();
}
