use crate::{
    BoundingBoxWrapper, BoundingSphere, Camera, CameraFrustum, CameraProjection, Material, MeshLod,
    PixelCache, Transform3D, THRESHOLDS,
};
use encase::ShaderType;
use gbase::{
    asset::{self, AssetHandle, ShaderLoader},
    glam::{Mat4, Vec3},
    render::{self, GpuImage, GpuMesh, Image, Mesh, RawBuffer},
    tracing, wgpu, Context,
};
use std::collections::BTreeSet;

//
// Pbr renderer
//

pub struct PbrRenderer {
    forward_shader_handle: asset::AssetHandle<render::ShaderBuilder>,
    deferred_shader_handle: asset::AssetHandle<render::ShaderBuilder>,

    pipeline_layout: render::ArcPipelineLayout,
    bindgroup_layout: render::ArcBindGroupLayout,
    vertex_attributes: BTreeSet<render::VertexAttributeId>,

    // TODO: use storagebuffer to avoid manual padding
    instances: RawBuffer<Instance>,
}

impl PbrRenderer {
    pub fn new(ctx: &mut Context, cache: &mut gbase::asset::AssetCache) -> Self {
        // let forward_shader_handle = asset::AssetBuilder::load::<ShaderLoader>(
        //     "../../utils/gbase_utils/assets/shaders/mesh.wgsl",
        // )
        // .watch(cache)
        // .build(cache);
        // let deferred_shader_handle = asset::AssetBuilder::load::<ShaderLoader>(
        //     "../../utils/gbase_utils/assets/shaders/deferred_mesh.wgsl",
        // )
        // .watch(cache)
        // .build(cache);

        let forward_shader_handle = asset::AssetBuilder::insert(render::ShaderBuilder::new(
            include_str!("../assets/shaders/mesh.wgsl"),
        ))
        .build(cache);
        let deferred_shader_handle = asset::AssetBuilder::insert(render::ShaderBuilder::new(
            include_str!("../assets/shaders/deferred_mesh.wgsl"),
        ))
        .build(cache);

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

        let instances = render::RawBufferBuilder::new(
            10000 * std::mem::size_of::<Instance>() as u64, // TODO: hardocoded
        )
        .label("instances")
        .usage(wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE)
        .build(ctx);

        Self {
            forward_shader_handle,
            deferred_shader_handle,

            pipeline_layout,
            bindgroup_layout,
            vertex_attributes,
            instances,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        ctx: &mut Context,
        cache: &mut gbase::asset::AssetCache,
        view: &wgpu::TextureView,
        view_format: wgpu::TextureFormat,
        camera: &Camera,
        camera_buffer: &render::UniformBuffer<crate::CameraUniform>,
        lights: &render::UniformBuffer<PbrLightUniforms>,
        depth_buffer: &render::DepthBuffer,
        frustum: &CameraFrustum,
        frame_meshes: Vec<(AssetHandle<MeshLod>, Transform3D)>,

        // optional
        shadow_map: &render::ArcTexture,
        shadow_matrices: &render::StorageBuffer<Vec<Mat4>>,
        shadow_matrices_distances: &render::StorageBuffer<Vec<f32>>,
    ) {
        if !asset::handle_loaded(cache, self.forward_shader_handle)
            || !asset::handle_loaded(cache, self.deferred_shader_handle)
        {
            return;
        }

        if frame_meshes.is_empty() {
            tracing::warn!("trying to render without any meshes");
            return;
        }

        let shader = asset::convert_asset(ctx, cache, self.forward_shader_handle, &()).unwrap();
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
            .cull_mode(wgpu::Face::Back)
            .depth_stencil(depth_buffer.depth_stencil_state())
            .build(ctx);

        let mut instances = Vec::new();
        let mut draws = Vec::new();
        let mut ranges = Vec::new();

        let mut frame_meshes = frame_meshes;

        //
        // Culling
        //

        frame_meshes.retain(|(mesh_lod, transform)| {
            if !cache.handle_loaded(*mesh_lod) {
                return false;
            }
            let bounds = mesh_lod
                .convert::<BoundingBoxWrapper>(ctx, cache, &())
                .unwrap();
            frustum.sphere_inside(&bounds, transform)
        });

        //
        // LOD
        //

        let mut final_meshes = Vec::new();
        for (mesh_lod, transform) in frame_meshes {
            let bounds = mesh_lod
                .convert::<BoundingBoxWrapper>(ctx, cache, &())
                .unwrap();
            let bounds_sphere = BoundingSphere::new(&bounds, &transform);
            let screen_coverage = screen_space_vertical_coverage(&bounds_sphere, camera);

            // TODO: hardcoded
            let lod = if screen_coverage >= THRESHOLDS[0] {
                0
            } else if screen_coverage >= THRESHOLDS[1] {
                1
            } else {
                2
            };

            final_meshes.push((lod, mesh_lod, transform));
        }

        //
        // Grouping of draws
        //

        // TODO: sort by material also?
        final_meshes.sort_by_key(|(_, mesh, _)| *mesh);

        let mut prev_mesh: Option<asset::AssetHandle<Mesh>> = None;
        for (index, (mesh_lod_level, mesh_lod_handle, transform)) in final_meshes.iter().enumerate()
        {
            let mesh_lod = mesh_lod_handle.get(cache).unwrap();
            let material = mesh_lod.material;
            let mesh = mesh_lod.get_lod_closest(*mesh_lod_level);
            let Material {
                base_color_texture,
                color_factor,
                metallic_roughness_texture,
                roughness_factor,
                metallic_factor,
                occlusion_texture,
                occlusion_strength,
                normal_texture,
                normal_scale,
                emissive_texture,
                emissive_factor,
            } = material.get(cache).unwrap().clone();

            instances.push(Instance {
                model: transform.matrix().to_cols_array_2d(),
                color_factor,
                roughness_factor,
                metallic_factor,
                occlusion_strength,
                normal_scale,
                emissive_factor,
                pad: 0.0,
            });

            if let Some(prev) = &prev_mesh {
                if *prev == mesh {
                    continue;
                }
            }
            prev_mesh = Some(mesh);

            let gpu_mesh = asset::convert_asset::<GpuMesh>(ctx, cache, mesh, &()).unwrap();
            let base_color_texture =
                asset::convert_asset::<GpuImage>(ctx, cache, base_color_texture, &()).unwrap();
            let normal_texture =
                asset::convert_asset::<GpuImage>(ctx, cache, normal_texture, &()).unwrap();
            let metallic_roughness_texture =
                asset::convert_asset::<GpuImage>(ctx, cache, metallic_roughness_texture, &())
                    .unwrap();
            let occlusion_texture =
                asset::convert_asset::<GpuImage>(ctx, cache, occlusion_texture, &()).unwrap();
            let emissive_texture =
                asset::convert_asset::<GpuImage>(ctx, cache, emissive_texture, &()).unwrap();

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
        ranges.push(final_meshes.len());

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
    }

    pub fn required_attributes(&self) -> &BTreeSet<render::VertexAttributeId> {
        &self.vertex_attributes
    }
}

// TODO: do area instead of just height
// circle area gives distortion?
// aabb does not?
//
// TODO:
// Clamp [0,1]
fn screen_space_vertical_coverage(bounds: &BoundingSphere, camera: &Camera) -> f32 {
    // TODO: is it bad to recalculate view matrix here?
    let view_space_pos = camera.view_matrix() * bounds.center.extend(1.0);

    // camera looks neg z, invert for positive distance
    let z = -view_space_pos.z;

    // if point behind near plane, coverage is 0.0
    if z <= camera.znear {
        return 0.0;
    }

    let diameter = 2.0 * bounds.radius;
    let screen_height = match camera.projection {
        CameraProjection::Perspective { fov } => 2.0 * (z * f32::tan(fov / 2.0)),
        CameraProjection::Orthographic { height } => height,
    };

    diameter / screen_height
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
        cache: &mut gbase::asset::AssetCache,
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
            cache: &mut gbase::asset::AssetCache,
            pixel_cache: &mut PixelCache,
            tex: Option<Image>,
            default: [u8; 4],
        ) -> asset::AssetHandle<Image> {
            if let Some(tex) = tex {
                asset::AssetBuilder::insert(tex).build(cache)
            } else {
                pixel_cache.allocate(cache, default)
            }
        }
        let base_color_texture = alloc(
            cache,
            pixel_cache,
            self.base_color_texture,
            BASE_COLOR_DEFAULT,
        );
        let normal_texture = alloc(cache, pixel_cache, self.normal_texture, NORMAL_DEFAULT);
        let metallic_roughness_texture = alloc(
            cache,
            pixel_cache,
            self.metallic_roughness_texture,
            METALLIC_ROUGHNESS_DEFAULT,
        );
        let occlusion_texture = alloc(
            cache,
            pixel_cache,
            self.occlusion_texture,
            OCCLUSION_DEFAULT,
        );
        let emissive_texture = alloc(cache, pixel_cache, self.emissive_texture, EMISSIVE_DEFAULT);

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
