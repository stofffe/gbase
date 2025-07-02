use crate::{BoundingSphere, Camera, CameraProjection, GizmoRenderer, PixelCache, Transform3D};
use encase::ShaderType;
use gbase::{
    asset::{self, AssetHandle},
    glam::{vec3, Mat4, Vec3},
    render::{self, GpuImage, GpuMesh, Image, Mesh, RawBuffer},
    tracing, wgpu, Context,
};
use std::{collections::BTreeSet, sync::Arc};

//
// Pbr renderer
//

pub struct PbrRenderer {
    forward_shader_handle: asset::AssetHandle<render::ShaderBuilder>,
    deferred_shader_handle: asset::AssetHandle<render::ShaderBuilder>,

    pipeline_layout: render::ArcPipelineLayout,
    bindgroup_layout: render::ArcBindGroupLayout,
    vertex_attributes: BTreeSet<render::VertexAttributeId>,

    instances: RawBuffer<Instance>,

    frame_meshes: Vec<(AssetHandle<render::Mesh>, Arc<GpuMaterial>, Transform3D)>,
}

impl PbrRenderer {
    pub fn new(ctx: &mut Context) -> Self {
        let forward_shader_handle =
            asset::AssetBuilder::load("../../utils/gbase_utils/assets/shaders/mesh.wgsl")
                .watch(ctx)
                .build(ctx);
        let deferred_shader_handle =
            asset::AssetBuilder::load("../../utils/gbase_utils/assets/shaders/deferred_mesh.wgsl")
                .watch(ctx)
                .build(ctx);

        let bindgroup_layout = render::BindGroupLayoutBuilder::new()
            .entries(vec![
                // camera
                render::BindGroupLayoutEntry::new()
                    .uniform()
                    .vertex()
                    .fragment(),
                // lights
                render::BindGroupLayoutEntry::new().uniform().fragment(),
                // instances
                render::BindGroupLayoutEntry::new()
                    .storage_readonly()
                    .vertex()
                    .fragment(),
                // base color texture
                render::BindGroupLayoutEntry::new()
                    .texture_float_filterable()
                    .fragment(),
                // base color sampler
                render::BindGroupLayoutEntry::new()
                    .sampler_filtering()
                    .fragment(),
                // normal texture
                render::BindGroupLayoutEntry::new()
                    .texture_float_filterable()
                    .fragment(),
                // normal sampler
                render::BindGroupLayoutEntry::new()
                    .sampler_filtering()
                    .fragment(),
                // metallic roughness texture
                render::BindGroupLayoutEntry::new()
                    .texture_float_filterable()
                    .fragment(),
                // metallic roughness sampler
                render::BindGroupLayoutEntry::new()
                    .sampler_filtering()
                    .fragment(),
                // occlusion texture
                render::BindGroupLayoutEntry::new()
                    .texture_float_filterable()
                    .fragment(),
                // occlusion sampler
                render::BindGroupLayoutEntry::new()
                    .sampler_filtering()
                    .fragment(),
                // emissive texture
                render::BindGroupLayoutEntry::new()
                    .texture_float_filterable()
                    .fragment(),
                // emissive sampler
                render::BindGroupLayoutEntry::new()
                    .sampler_filtering()
                    .fragment(),
                // shadow map texture
                render::BindGroupLayoutEntry::new()
                    .ty(wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Depth,
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        multisampled: false,
                    })
                    .fragment(),
                // shadow map sampler comparison
                render::BindGroupLayoutEntry::new()
                    .sampler_comparison()
                    .fragment(),
                // shadow matrices
                render::BindGroupLayoutEntry::new()
                    .storage_readonly()
                    .fragment(),
                // shadow matrices distances
                render::BindGroupLayoutEntry::new()
                    .storage_readonly()
                    .fragment(),
                // // test
                // render::BindGroupLayoutEntry::new().texture_depth(),
            ])
            .build(ctx);

        let vertex_attributes = BTreeSet::from([
            render::VertexAttributeId::Position,
            render::VertexAttributeId::Normal,
            render::VertexAttributeId::Uv(0),
            render::VertexAttributeId::Tangent,
            render::VertexAttributeId::Color(0),
        ]);

        let pipeline_layout = render::PipelineLayoutBuilder::new()
            .bind_groups(vec![bindgroup_layout.clone()])
            .build(ctx);

        let instances = render::RawBufferBuilder::new(render::RawBufferSource::Size(
            10000 * std::mem::size_of::<Instance>() as u64, // TODO: hardocoded
        ))
        .label("instances")
        .usage(wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE)
        .build(ctx);

        Self {
            forward_shader_handle,
            deferred_shader_handle,

            pipeline_layout,
            bindgroup_layout,
            vertex_attributes,
            frame_meshes: Vec::new(),
            instances,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        ctx: &mut Context,
        view: &wgpu::TextureView,
        view_format: wgpu::TextureFormat,
        camera: &crate::Camera,
        camera_buffer: &render::UniformBuffer<crate::CameraUniform>,
        lights: &render::UniformBuffer<PbrLightUniforms>,
        depth_buffer: &render::DepthBuffer,

        // optional
        shadow_map: &render::ArcTexture,
        shadow_matrices: &render::StorageBuffer<Vec<Mat4>>,
        shadow_matrices_distances: &render::StorageBuffer<Vec<f32>>,
    ) {
        if !asset::handle_loaded(ctx, self.forward_shader_handle.clone())
            || !asset::handle_loaded(ctx, self.deferred_shader_handle.clone())
        {
            self.frame_meshes.clear();
            return;
        }

        if self.frame_meshes.is_empty() {
            tracing::warn!("trying to render without any meshes");
            return;
        }

        let shader = asset::convert_asset(ctx, self.forward_shader_handle.clone(), &()).unwrap();
        let mut buffers = Vec::new();
        for attr in self.vertex_attributes.iter() {
            buffers.push(render::VertexBufferLayout::from_vertex_formats(
                wgpu::VertexStepMode::Vertex,
                vec![attr.format()],
            ));
        }

        let pipeline = render::RenderPipelineBuilder::new(shader, self.pipeline_layout.clone())
            .label("pbr")
            .buffers(buffers)
            .single_target(render::ColorTargetState::new().format(view_format))
            // .polygon_mode(wgpu::PolygonMode::Line)
            .cull_mode(wgpu::Face::Back)
            .depth_stencil(depth_buffer.depth_stencil_state())
            .build(ctx);

        self.frame_meshes.sort_by_key(|a| a.0.clone());

        let mut instances = Vec::new();
        let mut draws = Vec::new();
        let mut ranges = Vec::new();

        //
        // Culling
        //
        let frustum = camera.calculate_frustum(ctx);
        self.frame_meshes.retain(|(handle, _, transform)| {
            let gpu_mesh = asset::convert_asset::<GpuMesh>(ctx, handle.clone(), &()).unwrap();
            frustum.sphere_inside(&gpu_mesh.bounds, transform)
        });

        //
        // Grouping of draws
        //
        let mut prev_mesh: Option<asset::AssetHandle<Mesh>> = None;
        for (index, (mesh_handle, mat, transform)) in self.frame_meshes.iter().enumerate() {
            instances.push(Instance {
                model: transform.matrix().to_cols_array_2d(),
                color_factor: mat.color_factor,
                roughness_factor: mat.roughness_factor,
                metallic_factor: mat.metallic_factor,
                occlusion_strength: mat.occlusion_strength,
                normal_scale: mat.normal_scale,
                emissive_factor: mat.emissive_factor,
                pad: 0.0,
            });

            if let Some(prev) = &prev_mesh {
                if prev == mesh_handle {
                    continue;
                }
            }
            prev_mesh = Some(mesh_handle.clone());

            let gpu_mesh = asset::convert_asset::<GpuMesh>(ctx, mesh_handle.clone(), &()).unwrap();
            let base_color_texture =
                asset::convert_asset::<GpuImage>(ctx, mat.base_color_texture.clone(), &()).unwrap();
            let normal_texture =
                asset::convert_asset::<GpuImage>(ctx, mat.normal_texture.clone(), &()).unwrap();
            let metallic_roughness_texture =
                asset::convert_asset::<GpuImage>(ctx, mat.metallic_roughness_texture.clone(), &())
                    .unwrap();
            let occlusion_texture =
                asset::convert_asset::<GpuImage>(ctx, mat.occlusion_texture.clone(), &()).unwrap();
            let emissive_texture =
                asset::convert_asset::<GpuImage>(ctx, mat.emissive_texture.clone(), &()).unwrap();

            // TODO: enable linear/nearest depending on soft shadows
            let shadow_map_sampler_comparison = render::SamplerBuilder::new()
                .min_mag_filter(wgpu::FilterMode::Linear, wgpu::FilterMode::Linear)
                // .min_mag_filter(wgpu::FilterMode::Nearest, wgpu::FilterMode::Nearest)
                .compare(wgpu::CompareFunction::Less)
                .build(ctx);

            let bindgroup = render::BindGroupBuilder::new(self.bindgroup_layout.clone())
                .entries(vec![
                    // camera
                    render::BindGroupEntry::Buffer(camera_buffer.buffer()),
                    // lights
                    render::BindGroupEntry::Buffer(lights.buffer()),
                    // instances
                    render::BindGroupEntry::Buffer(self.instances.buffer()),
                    // base color texture
                    render::BindGroupEntry::Texture(base_color_texture.view()),
                    // base color sampler
                    render::BindGroupEntry::Sampler(base_color_texture.sampler()),
                    // normal texture
                    render::BindGroupEntry::Texture(normal_texture.view()),
                    // normal sampler
                    render::BindGroupEntry::Sampler(normal_texture.sampler()),
                    // metallic roughness texture
                    render::BindGroupEntry::Texture(metallic_roughness_texture.view()),
                    // metallic roughness sampler
                    render::BindGroupEntry::Sampler(metallic_roughness_texture.sampler()),
                    // occlusion roughness texture
                    render::BindGroupEntry::Texture(occlusion_texture.view()),
                    // occlusion roughness sampler
                    render::BindGroupEntry::Sampler(occlusion_texture.sampler()),
                    // emissive roughness texture
                    render::BindGroupEntry::Texture(emissive_texture.view()),
                    // emissive roughness sampler
                    render::BindGroupEntry::Sampler(emissive_texture.sampler()),
                    // shadow map texture
                    render::BindGroupEntry::Texture(
                        render::TextureViewBuilder::new(shadow_map.clone())
                            .array_layer_count(3) // TODO: hardcoded
                            .dimension(wgpu::TextureViewDimension::D2Array)
                            .build(ctx), // render::TextureViewBuilder::new(shadow_map.clone()).build(ctx),
                    ),
                    // shadow map sampler comparison
                    render::BindGroupEntry::Sampler(shadow_map_sampler_comparison),
                    // shadow matrices
                    render::BindGroupEntry::Buffer(shadow_matrices.buffer()),
                    // shadow matrices distances
                    render::BindGroupEntry::Buffer(shadow_matrices_distances.buffer()),
                ])
                .build(ctx);

            draws.push((gpu_mesh, bindgroup));
            ranges.push(index);
        }
        ranges.push(self.frame_meshes.len());

        self.instances.write(ctx, &instances);

        let mut encoder = render::EncoderBuilder::new().build(ctx);

        // TODO: using one render pass per draw call
        render::RenderPassBuilder::new()
            .label("pbr")
            .color_attachments(&[Some(render::RenderPassColorAttachment::new(view))])
            .trace_gpu(ctx, "pbr")
            .depth_stencil_attachment(depth_buffer.depth_render_attachment_load())
            .build_run(&mut encoder, |mut pass| {
                pass.set_pipeline(&pipeline);

                for (i, range) in ranges.windows(2).enumerate() {
                    let (from, to) = (range[0], range[1]);
                    let (mesh, bindgroup) = draws[i].clone();

                    mesh.bind_to_render_pass(&mut pass);
                    pass.set_bind_group(0, Some(bindgroup.as_ref()), &[]);
                    pass.draw_indexed(0..mesh.index_count.unwrap(), 0, from as u32..to as u32);
                }
            });

        render::queue(ctx).submit([encoder.finish()]);

        self.frame_meshes.clear();
    }

    pub fn add_mesh(
        &mut self,
        mesh: asset::AssetHandle<render::Mesh>,
        material: Arc<GpuMaterial>,
        transform: Transform3D,
    ) {
        self.frame_meshes.push((mesh, material, transform));
    }

    // temp?
    pub fn render_bounding_boxes(
        &self,
        ctx: &mut Context,
        gizmo_renderer: &mut GizmoRenderer,
        camera: &Camera,
    ) {
        for (mesh_handle, _, transform) in self.frame_meshes.iter() {
            let gpu_mesh = asset::convert_asset::<GpuMesh>(ctx, mesh_handle.clone(), &()).unwrap();
            let bounding_sphere = BoundingSphere::new(&gpu_mesh.bounds, transform);

            let mut color = vec3(1.0, 1.0, 1.0);

            if let CameraProjection::Perspective { fov } = camera.projection {
                let screen_coverage = screen_space_coverage(
                    bounding_sphere.radius,
                    transform.pos,
                    fov,
                    camera.view_matrix(),
                );

                color = if screen_coverage > 0.5 {
                    vec3(1.0, 0.0, 0.0)
                } else if screen_coverage > 0.25 {
                    vec3(0.0, 1.0, 0.0)
                } else {
                    vec3(0.0, 0.0, 1.0)
                };
            };

            gizmo_renderer.draw_sphere(
                &Transform3D::new(
                    bounding_sphere.center,
                    transform.rot,
                    Vec3::ONE * bounding_sphere.radius * 2.0,
                ),
                color,
            );
        }
    }

    pub fn required_attributes(&self) -> &BTreeSet<render::VertexAttributeId> {
        &self.vertex_attributes
    }
}

fn screen_space_coverage(r: f32, world_pos: Vec3, fov: f32, view_matrix: Mat4) -> f32 {
    let view_pos = view_matrix * world_pos.extend(1.0);
    let z = -view_pos.z;

    // behind camera?
    if z <= 0.0 {
        return 0.0;
    }

    r / (z * f32::tan(fov / 2.0))
}

//
// GPU types
//

#[derive(Clone, Debug)]
pub struct GpuMaterial {
    pub base_color_texture: asset::AssetHandle<Image>,
    pub color_factor: [f32; 4],

    pub metallic_roughness_texture: asset::AssetHandle<Image>,
    pub roughness_factor: f32,
    pub metallic_factor: f32,

    pub occlusion_texture: asset::AssetHandle<Image>,
    pub occlusion_strength: f32,

    pub normal_texture: asset::AssetHandle<Image>,
    pub normal_scale: f32,

    pub emissive_texture: asset::AssetHandle<Image>,
    pub emissive_factor: [f32; 3],
}

// TODO: shoudl use handles for textures to reuse
// TODO: emissive
#[derive(Debug, Clone)]
pub struct PbrMaterial {
    pub base_color_texture: Option<Image>,
    pub color_factor: [f32; 4],

    pub metallic_roughness_texture: Option<Image>,
    pub roughness_factor: f32,
    pub metallic_factor: f32,

    pub occlusion_texture: Option<Image>,
    pub occlusion_strength: f32,

    pub normal_texture: Option<Image>,
    pub normal_scale: f32,

    pub emissive_texture: Option<Image>,
    pub emissive_factor: [f32; 3],
}

impl PbrMaterial {
    // https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#materials-overview
    pub fn to_material(
        self,
        ctx: &mut Context,
        // TODO: part of context?
        // image_cache: &mut AssetCache<Image, GpuImage>,
        pixel_cache: &mut PixelCache,
    ) -> GpuMaterial {
        const BASE_COLOR_DEFAULT: [u8; 4] = [255, 255, 255, 255];
        const NORMAL_DEFAULT: [u8; 4] = [128, 128, 255, 0];
        const METALLIC_ROUGHNESS_DEFAULT: [u8; 4] = [0, 255, 0, 0];
        const OCCLUSION_DEFAULT: [u8; 4] = [255, 0, 0, 0];
        const EMISSIVE_DEFAULT: [u8; 4] = [0, 0, 0, 0];
        fn alloc(
            ctx: &mut Context,
            pixel_cache: &mut PixelCache,
            tex: Option<Image>,
            default: [u8; 4],
        ) -> asset::AssetHandle<Image> {
            if let Some(tex) = tex {
                asset::AssetBuilder::insert(tex).build(ctx)
            } else {
                pixel_cache.allocate(ctx, default)
            }
        }
        let base_color_texture = alloc(
            ctx,
            pixel_cache,
            self.base_color_texture,
            BASE_COLOR_DEFAULT,
        );
        let normal_texture = alloc(ctx, pixel_cache, self.normal_texture, NORMAL_DEFAULT);
        let metallic_roughness_texture = alloc(
            ctx,
            pixel_cache,
            self.metallic_roughness_texture,
            METALLIC_ROUGHNESS_DEFAULT,
        );
        let occlusion_texture = alloc(ctx, pixel_cache, self.occlusion_texture, OCCLUSION_DEFAULT);
        let emissive_texture = alloc(ctx, pixel_cache, self.emissive_texture, EMISSIVE_DEFAULT);

        GpuMaterial {
            base_color_texture,
            color_factor: self.color_factor,
            metallic_roughness_texture,
            roughness_factor: self.roughness_factor,
            metallic_factor: self.metallic_factor,
            occlusion_texture,
            occlusion_strength: self.occlusion_strength,
            normal_texture,
            normal_scale: self.normal_scale,
            emissive_texture,
            emissive_factor: self.emissive_factor,
        }
    }
}

//
// lights
//

#[derive(ShaderType)]
pub struct PbrLightUniforms {
    pub main_light_dir: Vec3,
    pub main_light_insensity: f32,
}

//
// Transforms
//

// TODO: use encase for auto alignment?
#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable, Debug)]
pub struct Instance {
    // transform
    model: [[f32; 4]; 4],

    // material
    color_factor: [f32; 4],
    roughness_factor: f32,
    metallic_factor: f32,
    occlusion_strength: f32,
    normal_scale: f32,
    emissive_factor: [f32; 3],
    // pad?
    pad: f32,
}

// #[allow(clippy::too_many_arguments)]
// pub fn render_deferred(
//     &mut self,
//     ctx: &mut Context,
//     deferred_buffers: &DeferredBuffers,
//
//     camera: &crate::Camera,
//     camera_buffer: &render::UniformBuffer<crate::CameraUniform>,
//     lights: &render::UniformBuffer<PbrLightUniforms>,
// ) {
//     if self.frame_meshes.is_empty() {
//         tracing::warn!("trying to render without any meshes");
//         return;
//     }
//
//     let shader = asset::convert_asset(ctx, self.deferred_shader_handle.clone(), &()).unwrap();
//     let mut buffers = Vec::new();
//     for attr in self.vertex_attributes.iter() {
//         buffers.push(render::VertexBufferLayout::from_vertex_formats(
//             wgpu::VertexStepMode::Vertex,
//             vec![attr.format()],
//         ));
//     }
//     let pipeline = render::RenderPipelineBuilder::new(shader, self.pipeline_layout.clone())
//         .label("pbr")
//         .buffers(buffers)
//         .multiple_targets(deferred_buffers.targets().into())
//         // .polygon_mode(wgpu::PolygonMode::Line)
//         .cull_mode(wgpu::Face::Back)
//         .depth_stencil(deferred_buffers.depth.depth_stencil_state())
//         .build(ctx);
//
//     let frustum = camera.calculate_frustum(ctx);
//
//     self.frame_meshes.sort_by_key(|a| a.0.clone());
//
//     let mut instances = Vec::new();
//     let mut draws = Vec::new();
//     let mut ranges = Vec::new();
//
//     //
//     // Culling
//     //
//     self.frame_meshes.retain(|(handle, _, transform)| {
//         let gpu_mesh = asset::convert_asset::<GpuMesh>(ctx, handle.clone(), &()).unwrap();
//         // let gpu_mesh = mesh_cache.get_gpu(ctx, handle.clone());
//         frustum.sphere_inside(&gpu_mesh.bounds, transform)
//     });
//
//     //
//     // Grouping of draws
//     //
//     let mut prev_mesh: Option<asset::AssetHandle<Mesh>> = None;
//     for (index, (mesh_handle, mat, transform)) in self.frame_meshes.iter().enumerate() {
//         instances.push(Instances {
//             model: transform.matrix().to_cols_array_2d(),
//             color_factor: mat.color_factor,
//             roughness_factor: mat.roughness_factor,
//             metallic_factor: mat.metallic_factor,
//             occlusion_strength: mat.occlusion_strength,
//             normal_scale: mat.normal_scale,
//             emissive_factor: mat.emissive_factor,
//         });
//
//         if let Some(prev) = &prev_mesh {
//             if prev == mesh_handle {
//                 continue;
//             }
//         }
//
//         let gpu_mesh = asset::convert_asset::<GpuMesh>(ctx, mesh_handle.clone(), &()).unwrap();
//
//         let base_color_texture =
//             asset::convert_asset::<GpuImage>(ctx, mat.base_color_texture.clone(), &()).unwrap();
//         let normal_texture =
//             asset::convert_asset::<GpuImage>(ctx, mat.normal_texture.clone(), &()).unwrap();
//         let metallic_roughness_texture =
//             asset::convert_asset::<GpuImage>(ctx, mat.metallic_roughness_texture.clone(), &())
//                 .unwrap();
//         let occlusion_texture =
//             asset::convert_asset::<GpuImage>(ctx, mat.occlusion_texture.clone(), &()).unwrap();
//         let emissive_texture =
//             asset::convert_asset::<GpuImage>(ctx, mat.emissive_texture.clone(), &()).unwrap();
//
//         let bindgroup = render::BindGroupBuilder::new(self.bindgroup_layout.clone())
//             .entries(vec![
//                 // camera
//                 render::BindGroupEntry::Buffer(camera_buffer.buffer()),
//                 // lights
//                 render::BindGroupEntry::Buffer(lights.buffer()),
//                 // instances
//                 render::BindGroupEntry::Buffer(self.instances.buffer()),
//                 // base color texture
//                 render::BindGroupEntry::Texture(base_color_texture.view()),
//                 // base color sampler
//                 render::BindGroupEntry::Sampler(base_color_texture.sampler()),
//                 // normal texture
//                 render::BindGroupEntry::Texture(normal_texture.view()),
//                 // normal sampler
//                 render::BindGroupEntry::Sampler(normal_texture.sampler()),
//                 // metallic roughness texture
//                 render::BindGroupEntry::Texture(metallic_roughness_texture.view()),
//                 // metallic roughness sampler
//                 render::BindGroupEntry::Sampler(metallic_roughness_texture.sampler()),
//                 // occlusion roughness texture
//                 render::BindGroupEntry::Texture(occlusion_texture.view()),
//                 // occlusion roughness sampler
//                 render::BindGroupEntry::Sampler(occlusion_texture.sampler()),
//                 // emissive roughness texture
//                 render::BindGroupEntry::Texture(emissive_texture.view()),
//                 // emissive roughness sampler
//                 render::BindGroupEntry::Sampler(emissive_texture.sampler()),
//             ])
//             .build(ctx);
//
//         draws.push((gpu_mesh, bindgroup));
//         ranges.push(index);
//         prev_mesh = Some(mesh_handle.clone());
//     }
//     ranges.push(self.frame_meshes.len());
//
//     self.instances.write(ctx, &instances);
//
//     // TODO: using one render pass per draw call
//     let attachments = &deferred_buffers.color_attachments();
//     render::RenderPassBuilder::new()
//         .color_attachments(attachments)
//         .depth_stencil_attachment(deferred_buffers.depth.depth_render_attachment_load())
//         .build_run_submit(ctx, |mut pass| {
//             pass.set_pipeline(&pipeline);
//
//             for (i, range) in ranges.windows(2).enumerate() {
//                 let (from, to) = (range[0], range[1]);
//                 let (mesh, bindgroup) = draws[i].clone();
//
//                 mesh.bind_to_render_pass(&mut pass);
//                 pass.set_bind_group(0, Some(bindgroup.as_ref()), &[]);
//                 pass.draw_indexed(0..mesh.index_count.unwrap(), 0, from as u32..to as u32);
//             }
//         });
//
//     self.frame_meshes.clear();
// }
