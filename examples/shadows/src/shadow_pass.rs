use gbase::{
    asset,
    render::{self, GpuMesh, VertexAttributeId},
    wgpu::{self, util::RenderEncoder},
    Context,
};

pub struct ShadowPass {
    pipeline_layout: render::ArcPipelineLayout,
    bindgroup_layout: render::ArcBindGroupLayout,

    shader_handle: asset::AssetHandle<render::ShaderBuilder>,
    pub shadow_map: render::DepthBuffer,
}

impl ShadowPass {
    pub fn new(ctx: &mut Context) -> Self {
        let shader_handle = asset::AssetBuilder::load("assets/shaders/shadow_pass.wgsl")
            .watch(ctx)
            .build(ctx);
        let bindgroup_layout = render::BindGroupLayoutBuilder::new()
            .entries(vec![
                // Camera
                render::BindGroupLayoutEntry::new().uniform().vertex(),
            ])
            .build(ctx);
        let pipeline_layout = render::PipelineLayoutBuilder::new()
            .label("shadow_pass")
            .bind_groups(vec![bindgroup_layout.clone()])
            .build(ctx);
        let shadow_map = render::DepthBufferBuilder::new()
            .label("shadow_pass")
            .screen_size(ctx)
            .build(ctx);
        Self {
            pipeline_layout,
            bindgroup_layout,
            shader_handle,
            shadow_map,
        }
    }

    pub fn render(
        &mut self,
        ctx: &mut Context,
        camera: &render::UniformBuffer<gbase_utils::CameraUniform>,
        meshes: Vec<(asset::AssetHandle<render::Mesh>, gbase_utils::Transform3D)>,
    ) {
        let mut assets_loaded = true;
        assets_loaded &= asset::handle_loaded(ctx, self.shader_handle.clone());
        // could probably skip not loaded ones
        for (mesh, _) in meshes.iter() {
            assets_loaded &= asset::handle_loaded(ctx, mesh.clone());
        }
        if !assets_loaded {
            return;
        }

        let bindgroup = render::BindGroupBuilder::new(self.bindgroup_layout.clone())
            .label("shadow_pass")
            .entries(vec![
                // camera
                render::BindGroupEntry::Buffer(camera.buffer()),
            ])
            .build(ctx);
        let shader = asset::convert_asset(ctx, self.shader_handle.clone(), &()).unwrap();
        let pipeline = render::RenderPipelineBuilder::new(shader, self.pipeline_layout.clone())
            .label("shadow_pass")
            .buffers(vec![render::VertexBufferLayout::from_vertex_formats(
                gbase::wgpu::VertexStepMode::Vertex,
                vec![wgpu::VertexFormat::Float32x3], // pos
            )])
            .depth_stencil(self.shadow_map.depth_stencil_state())
            .build(ctx);

        render::RenderPassBuilder::new()
            .label("shadow_pass")
            .depth_stencil_attachment(self.shadow_map.depth_render_attachment_clear())
            .build_run_submit_inner(
                render::device_arc(ctx),
                render::queue_arc(ctx),
                |mut pass: wgpu::RenderPass| {
                    pass.set_pipeline(&pipeline);

                    for (mesh, transform) in meshes {
                        let mesh_gpu =
                            asset::convert_asset::<render::GpuMesh>(ctx, mesh.clone(), &())
                                .unwrap();
                        mesh_gpu
                            .bind_to_render_pass_specific(&mut pass, [VertexAttributeId::Position]);
                        pass.set_bind_group(0, Some(bindgroup.as_ref()), &[]);
                        pass.draw_indexed(0..mesh_gpu.index_count.unwrap(), 0, 0..1);
                    }
                },
            );
    }
}
