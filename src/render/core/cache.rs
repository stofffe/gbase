use crate::render::{
    self, ArcBindGroup, ArcBindGroupLayout, ArcComputePipeline, ArcPipelineLayout,
    ArcRenderPipeline, ArcSampler, ArcShaderModule, BindGroupBuilder, BindGroupLayoutBuilder,
    ComputePipelineBuilder, PipelineLayoutBuilder, RenderPipelineBuilder, SamplerBuilder,
    ShaderBuilder, TextureBuilder,
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
    pub textures: HashMap<TextureBuilder, render::Texture>,
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
        }
    }
}
