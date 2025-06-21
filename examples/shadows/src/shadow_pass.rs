use gbase::{
    asset, bytemuck,
    glam::{vec3, Mat4, Vec3},
    render::{self, GpuMesh},
    wgpu, Context,
};

pub struct ShadowPass {
    pipeline_layout: render::ArcPipelineLayout,
    bindgroup_layout: render::ArcBindGroupLayout,
    instances: render::RawBuffer<Instance>,

    shader_handle: asset::AssetHandle<render::ShaderBuilder>,
    pub shadow_map: render::DepthBuffer,

    light_transform: Mat4,
    pub light_transform_buffer: render::UniformBuffer<Mat4>,
}

impl ShadowPass {
    pub fn new(ctx: &mut Context) -> Self {
        let shader_handle = asset::AssetBuilder::load("assets/shaders/shadow_pass.wgsl")
            .watch(ctx)
            .build(ctx);
        let bindgroup_layout = render::BindGroupLayoutBuilder::new()
            .entries(vec![
                // camera
                render::BindGroupLayoutEntry::new().uniform().vertex(),
                // instances
                render::BindGroupLayoutEntry::new()
                    .storage_readonly()
                    .vertex(),
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
        let instances = render::RawBufferBuilder::new(render::RawBufferSource::Size(
            10000 * std::mem::size_of::<Instance>() as u64,
        ))
        .label("instances")
        .usage(wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE)
        .build(ctx);

        let light_transform = Mat4::orthographic_rh(-10.0, 10.0, -10.0, 10.0, 0.1, 100.0)
            * Mat4::look_at_rh(
                vec3(-2.0, 4.0, -1.0),
                vec3(0.0, 0.0, 0.0),
                vec3(0.0, 1.0, 0.0),
            );
        let light_transform_buffer =
            render::UniformBufferBuilder::new(render::UniformBufferSource::Data(light_transform))
                .build(ctx);

        Self {
            pipeline_layout,
            bindgroup_layout,
            shader_handle,
            shadow_map,
            instances,
            light_transform,
            light_transform_buffer,
        }
    }

    pub fn render(
        &mut self,
        ctx: &mut Context,
        meshes: Vec<(asset::AssetHandle<render::Mesh>, gbase_utils::Transform3D)>,
        camera_pos: Vec3,
        main_light_dir: Vec3,
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

        // culling?

        // batch meshes
        let mut sorted_meshes = meshes;
        sorted_meshes.sort_by_key(|(mesh, ..)| mesh.clone());

        let mut instances = Vec::new();
        let mut draws = Vec::new();
        let mut ranges = Vec::new();
        let mut prev_mesh: Option<asset::AssetHandle<render::Mesh>> = None;
        for (index, (mesh_handle, transform)) in sorted_meshes.iter().enumerate() {
            instances.push(Instance {
                model: transform.matrix().to_cols_array_2d(),
            });

            if let Some(prev) = &prev_mesh {
                if prev == mesh_handle {
                    continue;
                }
            }
            prev_mesh = Some(mesh_handle.clone());

            let gpu_mesh = asset::convert_asset::<GpuMesh>(ctx, mesh_handle.clone(), &()).unwrap();
            draws.push(gpu_mesh);
            ranges.push(index);
        }
        ranges.push(sorted_meshes.len());

        let light_cam_dist = 10.0;
        let light_cam_width_height = 50.0;
        let light_cam_range = 100.0;
        let light_cam_pos = Vec3::ZERO - main_light_dir.normalize() * light_cam_dist;
        let light_cam_proj = Mat4::orthographic_rh(
            -light_cam_width_height,
            light_cam_width_height,
            -light_cam_width_height,
            light_cam_width_height,
            0.00,
            light_cam_range,
        );
        let light_cam_view = Mat4::look_at_rh(light_cam_pos, Vec3::ZERO, vec3(0.0, 1.0, 0.0));
        self.light_transform = light_cam_proj * light_cam_view;

        // update data
        self.instances.write(ctx, &instances);
        self.light_transform_buffer
            .write(ctx, &self.light_transform);

        // setup state
        let bindgroup = render::BindGroupBuilder::new(self.bindgroup_layout.clone())
            .label("shadow_pass")
            .entries(vec![
                // light transform
                render::BindGroupEntry::Buffer(self.light_transform_buffer.buffer()),
                // render::BindGroupEntry::Buffer(self.camera_buffer.buffer()),
                // instances
                render::BindGroupEntry::Buffer(self.instances.buffer()),
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

        // render
        let mut encoder = render::EncoderBuilder::new().build(ctx);
        render::RenderPassBuilder::new()
            .label("shadow_pass")
            .depth_stencil_attachment(self.shadow_map.depth_render_attachment_clear())
            .build_run(&mut encoder, |mut pass: wgpu::RenderPass| {
                pass.set_pipeline(&pipeline);
                pass.set_bind_group(0, Some(bindgroup.as_ref()), &[]);
                pass.set_bind_group(0, Some(bindgroup.as_ref()), &[]);

                for (i, range) in ranges.windows(2).enumerate() {
                    let (from, to) = (range[0], range[1]);
                    let mesh = draws[i].clone();

                    mesh.bind_to_render_pass(&mut pass);
                    pass.draw_indexed(0..mesh.index_count.unwrap(), 0, from as u32..to as u32);
                }
            });
        render::queue(ctx).submit([encoder.finish()]);
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable, Debug)]
pub struct Instance {
    // transform
    model: [[f32; 4]; 4],
}
