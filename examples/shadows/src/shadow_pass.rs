use gbase::{
    asset, bytemuck,
    glam::{vec3, vec4, Mat4, Vec3, Vec4Swizzles},
    render::{self, GpuMesh},
    wgpu, Context,
};
use gbase_utils::{Camera, GizmoRenderer};

pub struct ShadowPass {
    pipeline_layout: render::ArcPipelineLayout,
    bindgroup_layout: render::ArcBindGroupLayout,
    instances: render::RawBuffer<Instance>,

    shader_handle: asset::AssetHandle<render::ShaderBuilder>,
    pub shadow_map: render::ArcTexture,
    pub light_transform_buffers: Vec<render::UniformBuffer<Mat4>>,
}

fn calculate_light_matrix(
    ctx: &mut Context,
    main_light_dir: Vec3,
    mut camera: Camera,
    znear: f32,
    zfar: f32,
) -> gbase::glam::Mat4 {
    // get world space corners
    // change zfar to cover smaller area
    camera.znear = znear;
    camera.zfar = zfar;
    let camera_inv_view_proj = camera.uniform(ctx).inv_view_proj;

    let mut corners = Vec::new();
    for x in [-1.0, 1.0] {
        for y in [-1.0, 1.0] {
            for z in [0.0, 1.0] {
                let world_coord_homo = camera_inv_view_proj * vec4(x, y, z, 1.0);
                let world_coord = world_coord_homo / world_coord_homo.w;
                corners.push(world_coord.xyz());
            }
        }
    }

    // calc aabb (view space)
    let summed_corners = corners.iter().sum::<Vec3>();
    let center = summed_corners / corners.len() as f32;

    // view matrix
    let light_cam_view = Mat4::look_to_rh(center, main_light_dir, vec3(0.0, 1.0, 0.0));

    let mut min_light_space = Vec3::MAX;
    let mut max_light_space = Vec3::MIN;
    for corner in corners.iter() {
        let pos = light_cam_view.transform_point3(*corner);
        min_light_space = min_light_space.min(pos);
        max_light_space = max_light_space.max(pos);
    }

    // grow camera depth behind and in front of camera
    let z_mult = 10.0;
    if min_light_space.z < 0.0 {
        min_light_space.z *= z_mult;
    } else {
        min_light_space.z /= z_mult;
    }
    if max_light_space.z < 0.0 {
        max_light_space.z /= z_mult;
    } else {
        max_light_space.z *= z_mult;
    }

    // projection matrix
    let light_cam_proj = Mat4::orthographic_rh(
        min_light_space.x,
        max_light_space.x,
        min_light_space.y,
        max_light_space.y,
        min_light_space.z,
        max_light_space.z,
    );

    light_cam_proj * light_cam_view
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
        let instances = render::RawBufferBuilder::new(render::RawBufferSource::Size(
            10000 * std::mem::size_of::<Instance>() as u64, // TODO: hardocoded
        ))
        .label("instances")
        .usage(wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE)
        .build(ctx);

        let light_transform_buffers = vec![
            render::UniformBufferBuilder::new(render::UniformBufferSource::Empty).build(ctx),
            render::UniformBufferBuilder::new(render::UniformBufferSource::Empty).build(ctx),
            render::UniformBufferBuilder::new(render::UniformBufferSource::Empty).build(ctx),
        ];

        let shadow_map_new = render::TextureBuilder::new(render::TextureSource::Empty(1024, 1024))
            .label("shadow map")
            .with_format(wgpu::TextureFormat::Depth32Float)
            .usage(wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING)
            .depth_or_array_layers(3)
            .build(ctx);

        Self {
            pipeline_layout,
            bindgroup_layout,
            shader_handle,
            shadow_map: shadow_map_new,
            instances,
            light_transform_buffers,
        }
    }

    pub fn render(
        &mut self,
        ctx: &mut Context,
        meshes: Vec<(asset::AssetHandle<render::Mesh>, gbase_utils::Transform3D)>,
        camera: &gbase_utils::Camera,
        main_light_dir: Vec3,
        gizmo: &mut GizmoRenderer,
    ) {
        //
        // early exits
        //

        let mut assets_loaded = true;
        assets_loaded &= asset::handle_loaded(ctx, self.shader_handle.clone());

        // could probably skip not loaded ones
        for (mesh, _) in meshes.iter() {
            assets_loaded &= asset::handle_loaded(ctx, mesh.clone());
        }
        if !assets_loaded {
            return;
        }

        //
        // light projection matrices
        //
        let mut sorted_meshes = meshes;
        sorted_meshes.sort_by_key(|(mesh, ..)| mesh.clone());

        for (i, (near, far)) in [(0.01, 20.0), (20.01, 50.0), (50.01, 150.0)]
            .into_iter()
            .enumerate()
        {
            let light_transform =
                calculate_light_matrix(ctx, main_light_dir, camera.clone(), near, far);

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

                let gpu_mesh =
                    asset::convert_asset::<GpuMesh>(ctx, mesh_handle.clone(), &()).unwrap();
                draws.push(gpu_mesh);
                ranges.push(index);
            }
            ranges.push(sorted_meshes.len());

            //
            // update data & render meshes
            //
            self.instances.write(ctx, &instances);
            self.light_transform_buffers[i].write(ctx, &light_transform);

            // setup state
            let bindgroup = render::BindGroupBuilder::new(self.bindgroup_layout.clone())
                .label("shadow_pass")
                .entries(vec![
                    // light transform
                    render::BindGroupEntry::Buffer(self.light_transform_buffers[i].buffer()),
                    // instances
                    render::BindGroupEntry::Buffer(self.instances.buffer()),
                ])
                .build(ctx);
            let shader = asset::convert_asset(ctx, self.shader_handle.clone(), &()).unwrap();
            let pipeline = render::RenderPipelineBuilder::new(shader, self.pipeline_layout.clone())
                .label("shadow_pass")
                // .cull_mode(wgpu::Face::Front)
                .cull_mode(wgpu::Face::Back)
                .buffers(vec![render::VertexBufferLayout::from_vertex_formats(
                    gbase::wgpu::VertexStepMode::Vertex,
                    vec![wgpu::VertexFormat::Float32x3], // pos
                )])
                .depth_stencil(wgpu::DepthStencilState {
                    format: self.shadow_map.format(),
                    // format: self.shadow_map.framebuffer().format(),
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::LessEqual,
                    stencil: wgpu::StencilState::default(),
                    // TODO: be tweakable
                    bias: wgpu::DepthBiasState {
                        constant: 4,
                        slope_scale: 8.0,
                        clamp: 0.0, // disable with 0.0
                    },
                })
                .build(ctx);

            let mut encoder = render::EncoderBuilder::new().build(ctx);
            render::RenderPassBuilder::new()
                .label("shadow_pass")
                // .depth_stencil_attachment(self.shadow_map.depth_render_attachment_clear())
                .depth_stencil_attachment(wgpu::RenderPassDepthStencilAttachment {
                    view: &render::TextureViewBuilder::new(self.shadow_map.clone())
                        .base_array_layer(i as u32)
                        .dimension(wgpu::TextureViewDimension::D2)
                        .build(ctx),
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: Default::default(),
                })
                .build_run(&mut encoder, |mut pass: wgpu::RenderPass| {
                    pass.set_pipeline(&pipeline);
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

        // gizmo.draw_sphere(
        //     &Transform3D::from_pos(center).with_scale(Vec3::ONE * 1.0),
        //     vec3(1.0, 1.0, 1.0),
        // );

        // TODO: culling?

        //
        // batch meshes
        //
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable, Debug)]
pub struct Instance {
    // transform
    model: [[f32; 4]; 4],
}
