use gbase::{
    asset::{AssetCache, AssetHandle, ConvertAssetResult, ShaderLoader},
    bytemuck, render, wgpu,
};

pub struct UIRenderer {
    shader_handle: AssetHandle<render::ShaderBuilder>,
    bindgroup_layout: render::ArcBindGroupLayout,
    pipeline_layout: render::ArcPipelineLayout,
    instance_buffer: render::RawBuffer<UIElementInstace>,
}

impl UIRenderer {
    pub fn new(ctx: &mut gbase::Context, cache: &mut AssetCache, max_elements: u64) -> Self {
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
        let instance_buffer = render::RawBufferBuilder::new(max_elements).build(ctx);

        Self {
            pipeline_layout,
            bindgroup_layout,
            shader_handle,
            instance_buffer,
        }
    }

    pub fn render(
        &self,
        ctx: &mut gbase::Context,
        cache: &mut AssetCache,
        view: &wgpu::TextureView,
        view_format: wgpu::TextureFormat,
        ui_elements: Vec<UIElementInstace>,
    ) {
        let ConvertAssetResult::Success(shader) = self.shader_handle.convert(ctx, cache) else {
            return;
        };

        self.instance_buffer.write(ctx, &ui_elements);

        let bindgroup = render::BindGroupBuilder::new(self.bindgroup_layout.clone())
            .entries(vec![])
            .build(ctx);

        let pipeline = render::RenderPipelineBuilder::new(shader, self.pipeline_layout.clone())
            .buffers(vec![UIElementInstace::desc()])
            .topology(wgpu::PrimitiveTopology::TriangleStrip)
            .single_target(render::ColorTargetState::new().format(view_format))
            .build(ctx);

        render::RenderPassBuilder::new()
            .color_attachments(&[Some(render::RenderPassColorAttachment::new(view))])
            .build_run_submit(ctx, |mut pass| {
                pass.set_pipeline(&pipeline);
                pass.set_vertex_buffer(0, self.instance_buffer.slice(..));
                pass.set_bind_group(0, Some(bindgroup.as_ref()), &[]);

                pass.draw(0..4, 0..ui_elements.len() as u32);
            });
    }
}

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct UIElementInstace {
    pub position: [f32; 2], // uv coordinate system, (0,0) top left and y+ is down
    pub size: [f32; 2],
    pub color: [f32; 4],
}

impl UIElementInstace {
    pub fn desc() -> render::VertexBufferLayout {
        render::VertexBufferLayout::from_vertex_formats(
            wgpu::VertexStepMode::Instance,
            vec![
                wgpu::VertexFormat::Float32x2, // pos
                wgpu::VertexFormat::Float32x2, // scale
                wgpu::VertexFormat::Float32x4, // color
            ],
        )
    }
}
