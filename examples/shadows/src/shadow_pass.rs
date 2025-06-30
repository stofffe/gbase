use encase::rts_array::Length;
use gbase::{
    asset,
    encase::ShaderType,
    glam::{vec3, vec4, Mat4, Vec3, Vec4Swizzles},
    render::{self, GpuMesh},
    tracing, wgpu, Context,
};
use gbase_utils::{Camera, GizmoRenderer};

pub struct ShadowPass {
    pipeline_layout: render::ArcPipelineLayout,
    bindgroup_layout: render::ArcBindGroupLayout,
    instances: render::StorageBuffer<Vec<ShadowInstance>>,

    shader_handle: asset::AssetHandle<render::ShaderBuilder>,
    pub shadow_map: render::ArcTexture,
    pub light_matrices_buffer: render::StorageBuffer<Vec<Mat4>>,
    pub light_matrices_index: render::UniformBuffer<u32>,
    pub light_matrices_distances: render::StorageBuffer<Vec<f32>>,
}

const MAX_SHADOW_INSTANCES: u64 = 10000;
const MAX_SHADOW_CASCADES: u64 = 3;
const SHADOW_MAP_RESOLUTION: u32 = 1024;

impl ShadowPass {
    pub fn new(ctx: &mut Context) -> Self {
        let shader_handle = asset::AssetBuilder::load("assets/shaders/shadow_pass.wgsl")
            .watch(ctx)
            .build(ctx);
        let bindgroup_layout = render::BindGroupLayoutBuilder::new()
            .entries(vec![
                // light matrices
                render::BindGroupLayoutEntry::new()
                    .storage_readonly()
                    .vertex(),
                // light matrix index
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
        let instances = render::StorageBufferBuilder::new(render::StorageBufferSource::Size(
            MAX_SHADOW_INSTANCES * std::mem::size_of::<ShadowInstance>() as u64, // TODO: hardocoded
        ))
        .label("instances")
        .usage(wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE)
        .build(ctx);

        let shadow_map = render::TextureBuilder::new(render::TextureSource::Empty(
            SHADOW_MAP_RESOLUTION,
            SHADOW_MAP_RESOLUTION,
        ))
        .label("shadow map")
        .with_format(wgpu::TextureFormat::Depth32Float)
        .usage(wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING)
        .depth_or_array_layers(MAX_SHADOW_CASCADES as u32)
        .build(ctx);

        let light_matrices_index =
            render::UniformBufferBuilder::new(render::UniformBufferSource::Empty).build(ctx);

        let light_matrices_distances =
            render::StorageBufferBuilder::new(render::StorageBufferSource::Size(
                MAX_SHADOW_CASCADES * std::mem::size_of::<u32>() as u64,
            ))
            .label("light matrices distances")
            .build(ctx);

        let light_matrices_buffer =
            render::StorageBufferBuilder::new(render::StorageBufferSource::Size(
                MAX_SHADOW_CASCADES * std::mem::size_of::<Mat4>() as u64,
            ))
            .label("light matrices")
            .usage(wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE)
            .build(ctx);

        Self {
            pipeline_layout,
            bindgroup_layout,
            shader_handle,
            shadow_map,
            instances,
            light_matrices_index,
            light_matrices_distances,
            light_matrices_buffer,
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
        let mut light_matrices = Vec::new();

        let planes = [0.01, 3.0, 10.0, 30.0];
        for plane in planes.windows(2) {
            light_matrices.push(calculate_light_matrix(
                ctx,
                main_light_dir,
                camera.clone(),
                plane[0],
                plane[1],
            ));
        }
        self.light_matrices_buffer.write(ctx, &light_matrices);
        self.light_matrices_distances
            .write(ctx, &planes[1..].to_vec()); // ignore first

        //
        // meshes
        //

        let mut sorted_meshes = meshes;
        sorted_meshes.sort_by_key(|(mesh, ..)| mesh.clone());

        for i in 0..planes.len() - 1 {
            let mut instances = Vec::new();
            let mut draws = Vec::new();
            let mut ranges = Vec::new();
            let mut prev_mesh: Option<asset::AssetHandle<render::Mesh>> = None;
            for (index, (mesh_handle, transform)) in sorted_meshes.iter().enumerate() {
                instances.push(ShadowInstance {
                    model: transform.matrix(),
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
            self.light_matrices_index.write(ctx, &(i as u32));

            // setup state
            let bindgroup = render::BindGroupBuilder::new(self.bindgroup_layout.clone())
                .label("shadow_pass")
                .entries(vec![
                    // light matrices
                    render::BindGroupEntry::Buffer(self.light_matrices_buffer.buffer()),
                    // light matrix index
                    render::BindGroupEntry::Buffer(self.light_matrices_index.buffer()),
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

    let mut center = Vec3::ZERO;
    for corner in corners.iter() {
        center += *corner;
    }
    center /= corners.len() as f32;

    let mut radius = 0.0f32;
    for corner in corners.iter() {
        radius = radius.max(center.distance(*corner));
    }

    // snap radius to larger steps to avoid shimmering
    radius = f32::ceil(radius * 16.0) / 16.0; // TODO: whats best value here?

    let min = Vec3::splat(-radius);
    let max = Vec3::splat(radius);

    const MUL: f32 = 8.0;
    let shadow_camera_pos = center - main_light_dir * radius * MUL;
    let ortho = Mat4::orthographic_rh(min.x, max.x, min.y, max.y, 0.01, radius * MUL * 2.0); // Larger here?
    let lookat = Mat4::look_at_rh(shadow_camera_pos, center, Vec3::Y);

    let shadow_matrix = ortho * lookat;
    let world_origin = vec4(0.0, 0.0, 0.0, 1.0);
    let shadow_origin = shadow_matrix * world_origin * (SHADOW_MAP_RESOLUTION as f32 / 2.0);
    let rounded_offset =
        (shadow_origin.round() - shadow_origin) * (2.0 / SHADOW_MAP_RESOLUTION as f32);

    let mut snapped_ortho = ortho;
    snapped_ortho.col_mut(3).x += rounded_offset.x;
    snapped_ortho.col_mut(3).y += rounded_offset.y;

    snapped_ortho * lookat
}

#[repr(C)]
#[derive(Copy, Clone, Debug, ShaderType)]
pub struct ShadowInstance {
    model: Mat4,
}

// // get world space corners
// // change zfar to cover smaller area
// camera.znear = znear;
// camera.zfar = zfar;
// let camera_inv_view_proj = camera.uniform(ctx).inv_view_proj;
//
// let mut corners = Vec::new();
// for x in [-1.0, 1.0] {
//     for y in [-1.0, 1.0] {
//         for z in [0.0, 1.0] {
//             let world_coord_homo = camera_inv_view_proj * vec4(x, y, z, 1.0);
//             let world_coord = world_coord_homo / world_coord_homo.w;
//             corners.push(world_coord.xyz());
//         }
//     }
// }
//
// // calc aabb (view space)
// let summed_corners = corners.iter().sum::<Vec3>();
// let mut center = summed_corners / corners.len() as f32;
//
// // view matrix
// // let light_cam_view = Mat4::look_to_rh(center, main_light_dir, vec3(0.0, 1.0, 0.0));
// // let light_cam_view_inv = light_cam_view.inverse();
//
// //     let mut tmp = light_cam_view * center.extend(1.0);
// // tmp.x = tmp.x.floor() ;
// //     tmp.y = tmp.y.floor();
// //     center = (light_cam_view_inv * tmp).xyz();
//
// let light_cam_view = Mat4::look_to_rh(center, main_light_dir, vec3(0.0, 1.0, 0.0));
//
// let mut min_light_space = Vec3::MAX;
// let mut max_light_space = Vec3::MIN;
// for corner in corners.iter() {
//     let pos = light_cam_view.transform_point3(*corner);
//     min_light_space = min_light_space.min(pos);
//     max_light_space = max_light_space.max(pos);
// }
//
// let mut left = min_light_space.x;
// let mut right = max_light_space.x;
// let mut bottom = min_light_space.y;
// let mut top = max_light_space.y;
// let mut near = min_light_space.z;
// let mut far = max_light_space.z;
//
// // grow camera depth behind and in front of camera
// let z_mult = 10.0;
// if min_light_space.z < 0.0 {
//     near *= z_mult;
// } else {
//     near /= z_mult;
// }
// if max_light_space.z < 0.0 {
//     far /= z_mult;
// } else {
//     far *= z_mult;
// }
//
// let constant_size = true;
// let square = true;
// let round_to_pixel = true;
//
// let actual_size = if constant_size {
//     let far_face_diagnoal = (corners[7] - corners[1]).length();
//     let forward_diagnoal = (corners[7] - corners[0]).length();
//     f32::max(far_face_diagnoal, forward_diagnoal)
// } else {
//     f32::max(right - left, top - bottom)
// };
//
// let height = top - bottom;
// let width = right - left;
//
// if square {
//     let mut diff = actual_size - height;
//     if diff > 0.0 {
//         top += diff / 2.0;
//         bottom -= diff / 2.0;
//     }
//     diff = actual_size - width;
//     if diff > 0.0 {
//         right += diff / 2.0;
//         left -= diff / 2.0;
//     }
// }
//
// let dim = 1024.0;
// if round_to_pixel {
//     let pixel_size = width.max(height) / dim;
//     left = f32::round(left / pixel_size) * pixel_size;
//     right = f32::round(right / pixel_size) * pixel_size;
//     bottom = f32::round(bottom / pixel_size) * pixel_size;
//     top = f32::round(top / pixel_size) * pixel_size;
// }
//
// // projection matrix
// let light_cam_proj = Mat4::orthographic_rh(left, right, bottom, top, near, far);
//
// light_cam_proj * light_cam_view
