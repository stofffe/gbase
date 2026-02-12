use gbase::{
    asset::{AssetCache, AssetHandle, ConvertAssetResult, ShaderLoader},
    render, wgpu,
};

pub struct UiRenderer {
    shader_handle: AssetHandle<render::ShaderBuilder>,
    bindgroup_layout: render::ArcBindGroupLayout,
    pipeline_layout: render::ArcPipelineLayout,
}

impl UiRenderer {
    pub fn new(ctx: &mut gbase::Context, cache: &mut AssetCache) -> Self {
        let shader_handle = cache
            .load_builder("assets/shaders/ui.wgsl", ShaderLoader {})
            .watch(cache)
            .build(cache);

        let bindgroup_layout = render::BindGroupLayoutBuilder::new()
            .entries(vec![])
            .build(ctx);

        let pipeline_layout = render::PipelineLayoutBuilder::new()
            .bind_groups(vec![bindgroup_layout.clone()])
            .build(ctx);

        Self {
            pipeline_layout,
            bindgroup_layout,
            shader_handle,
        }
    }

    pub fn render(
        &self,
        ctx: &mut gbase::Context,
        cache: &mut AssetCache,
        view: &wgpu::TextureView,
        view_format: wgpu::TextureFormat,
    ) {
        let ConvertAssetResult::Success(shader) = self.shader_handle.convert(ctx, cache) else {
            return;
        };

        let bindgroup = render::BindGroupBuilder::new(self.bindgroup_layout.clone())
            .entries(vec![])
            .build(ctx);

        let pipeline = render::RenderPipelineBuilder::new(shader, self.pipeline_layout.clone())
            .topology(wgpu::PrimitiveTopology::TriangleStrip)
            .single_target(render::ColorTargetState::new().format(view_format))
            .build(ctx);

        render::RenderPassBuilder::new()
            .color_attachments(&[Some(render::RenderPassColorAttachment::new(view))])
            .build_run_submit(ctx, |mut pass| {
                pass.set_pipeline(&pipeline);
                pass.set_bind_group(0, Some(bindgroup.as_ref()), &[]);
                pass.draw(0..4, 0..1);
            });
    }
}
