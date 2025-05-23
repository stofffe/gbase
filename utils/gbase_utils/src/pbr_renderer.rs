use crate::{
    deferred_buffers, AssetCache, AssetHandle, BoundingSphere, DeferredBuffers, GizmoRenderer,
    PixelCache, Transform3D, WHITE,
};
use encase::ShaderType;
use gbase::{
    glam::{Vec3, Vec4Swizzles},
    render::{self, GpuImage, GpuMesh, Image, Mesh, RawBuffer, ShaderBuilder},
    time, tracing, wgpu, Context,
};
use std::{collections::BTreeSet, sync::Arc};

//
// Pbr renderer
//

pub struct PbrRenderer {
    forward_shader_handle: AssetHandle<render::ShaderBuilder>,
    deferred_shader_handle: AssetHandle<render::ShaderBuilder>,

    pipeline_layout: render::ArcPipelineLayout,
    bindgroup_layout: render::ArcBindGroupLayout,
    vertex_attributes: BTreeSet<render::VertexAttributeId>,

    transforms: RawBuffer<Instances>,

    frame_meshes: Vec<(AssetHandle<render::Mesh>, Arc<GpuMaterial>, Transform3D)>,

    shader_cache: AssetCache<ShaderBuilder, wgpu::ShaderModule>,
}

impl PbrRenderer {
    pub fn new(ctx: &mut Context) -> Self {
        let mut shader_cache = AssetCache::new();
        let forward_shader_handle = shader_cache.allocate_reload(
            render::ShaderBuilder {
                label: None,
                source: include_str!("../assets/shaders/mesh.wgsl").into(),
            },
            "../../utils/gbase_utils/assets/shaders/mesh.wgsl".into(),
        );
        let deferred_shader_handle = shader_cache.allocate_reload(
            render::ShaderBuilder {
                label: None,
                source: include_str!("../assets/shaders/deferred_mesh.wgsl").into(),
            },
            "../../utils/gbase_utils/assets/shaders/deferred_mesh.wgsl".into(),
        );

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

        let transforms = render::RawBufferBuilder::new(render::RawBufferSource::Size(
            100000 * std::mem::size_of::<Instances>() as u64,
        ))
        .usage(wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::STORAGE)
        .build(ctx);

        Self {
            forward_shader_handle,
            deferred_shader_handle,

            pipeline_layout,
            bindgroup_layout,
            vertex_attributes,
            frame_meshes: Vec::new(),
            shader_cache,
            transforms,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render_deferred(
        &mut self,
        ctx: &mut Context,
        deferred_buffers: &DeferredBuffers,
        mesh_cache: &mut AssetCache<Mesh, GpuMesh>,
        image_cache: &mut AssetCache<Image, GpuImage>,

        camera: &crate::Camera,
        camera_buffer: &render::UniformBuffer<crate::CameraUniform>,
        lights: &render::UniformBuffer<PbrLightUniforms>,
    ) {
        if self.frame_meshes.is_empty() {
            tracing::warn!("trying to render without any meshes");
            return;
        }

        self.shader_cache.check_watched_files(ctx);

        let shader = self
            .shader_cache
            .get_gpu(ctx, self.deferred_shader_handle.clone());
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
            .multiple_targets(deferred_buffers.targets().into())
            // .polygon_mode(wgpu::PolygonMode::Line)
            .cull_mode(wgpu::Face::Back)
            .depth_stencil(deferred_buffers.depth.depth_stencil_state())
            .build(ctx);

        let frustum = camera.calculate_frustum(ctx);

        self.frame_meshes.sort_by_key(|a| a.0.clone());

        let mut instances = Vec::new();
        let mut draws = Vec::new();
        let mut ranges = Vec::new();

        //
        // Culling
        //
        self.frame_meshes.retain(|(handle, _, transform)| {
            let gpu_mesh = mesh_cache.get_gpu(ctx, handle.clone());
            frustum.sphere_inside(&gpu_mesh.bounds, transform)
        });

        //
        // Grouping of draws
        //
        let mut prev_mesh: Option<AssetHandle<Mesh>> = None;
        for (index, (mesh_handle, mat, transform)) in self.frame_meshes.iter().enumerate() {
            instances.push(Instances {
                model: transform.matrix().to_cols_array_2d(),
                color_factor: mat.color_factor,
                roughness_factor: mat.roughness_factor,
                metallic_factor: mat.metallic_factor,
                occlusion_strength: mat.occlusion_strength,
                normal_scale: mat.normal_scale,
                emissive_factor: mat.emissive_factor,
            });

            if let Some(prev) = &prev_mesh {
                if prev == mesh_handle {
                    continue;
                }
            }

            let gpu_mesh = mesh_cache.get_gpu(ctx, mesh_handle.clone());

            let base_color_texture = image_cache.get_gpu(ctx, mat.base_color_texture.clone());
            let normal_texture = image_cache.get_gpu(ctx, mat.normal_texture.clone());
            let metallic_roughness_texture =
                image_cache.get_gpu(ctx, mat.metallic_roughness_texture.clone());
            let occlusion_texture = image_cache.get_gpu(ctx, mat.occlusion_texture.clone());
            let emissive_texture = image_cache.get_gpu(ctx, mat.emissive_texture.clone());
            let bindgroup = render::BindGroupBuilder::new(self.bindgroup_layout.clone())
                .entries(vec![
                    // camera
                    render::BindGroupEntry::Buffer(camera_buffer.buffer()),
                    // lights
                    render::BindGroupEntry::Buffer(lights.buffer()),
                    // instances
                    render::BindGroupEntry::Buffer(self.transforms.buffer()),
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
                ])
                .build(ctx);

            draws.push((gpu_mesh, bindgroup));
            ranges.push(index);
            prev_mesh = Some(mesh_handle.clone());
        }
        ranges.push(self.frame_meshes.len());

        self.transforms.write(ctx, &instances);

        // TODO: using one render pass per draw call
        let attachments = &deferred_buffers.color_attachments();
        render::RenderPassBuilder::new()
            .color_attachments(attachments)
            .depth_stencil_attachment(deferred_buffers.depth.depth_render_attachment_load())
            .build_run_submit(ctx, |mut pass| {
                pass.set_pipeline(&pipeline);

                for (i, range) in ranges.windows(2).enumerate() {
                    let (from, to) = (range[0], range[1]);
                    let (mesh, bindgroup) = draws[i].clone();

                    mesh.bind_to_render_pass(&mut pass);
                    pass.set_bind_group(0, Some(bindgroup.as_ref()), &[]);
                    pass.draw_indexed(0..mesh.index_count.unwrap(), 0, from as u32..to as u32);
                }
            });

        self.frame_meshes.clear();
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        ctx: &mut Context,
        mesh_cache: &mut AssetCache<Mesh, GpuMesh>,
        image_cache: &mut AssetCache<Image, GpuImage>,
        view: &wgpu::TextureView,
        view_format: wgpu::TextureFormat,
        camera: &crate::Camera,
        camera_buffer: &render::UniformBuffer<crate::CameraUniform>,
        lights: &render::UniformBuffer<PbrLightUniforms>,
        depth_buffer: &render::DepthBuffer,
    ) {
        if self.frame_meshes.is_empty() {
            tracing::warn!("trying to render without any meshes");
            return;
        }

        self.shader_cache.check_watched_files(ctx);

        let shader = self
            .shader_cache
            .get_gpu(ctx, self.forward_shader_handle.clone());
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

        let frustum = camera.calculate_frustum(ctx);

        self.frame_meshes.sort_by_key(|a| a.0.clone());

        let mut instances = Vec::new();
        let mut draws = Vec::new();
        let mut ranges = Vec::new();

        //
        // Culling
        //
        self.frame_meshes.retain(|(handle, _, transform)| {
            let gpu_mesh = mesh_cache.get_gpu(ctx, handle.clone());
            frustum.sphere_inside(&gpu_mesh.bounds, transform)
        });

        //
        // Grouping of draws
        //
        let mut prev_mesh: Option<AssetHandle<Mesh>> = None;
        for (index, (mesh_handle, mat, transform)) in self.frame_meshes.iter().enumerate() {
            instances.push(Instances {
                model: transform.matrix().to_cols_array_2d(),
                color_factor: mat.color_factor,
                roughness_factor: mat.roughness_factor,
                metallic_factor: mat.metallic_factor,
                occlusion_strength: mat.occlusion_strength,
                normal_scale: mat.normal_scale,
                emissive_factor: mat.emissive_factor,
            });

            if let Some(prev) = &prev_mesh {
                if prev == mesh_handle {
                    continue;
                }
            }

            let gpu_mesh = mesh_cache.get_gpu(ctx, mesh_handle.clone());
            let base_color_texture = image_cache.get_gpu(ctx, mat.base_color_texture.clone());
            let normal_texture = image_cache.get_gpu(ctx, mat.normal_texture.clone());
            let metallic_roughness_texture =
                image_cache.get_gpu(ctx, mat.metallic_roughness_texture.clone());
            let occlusion_texture = image_cache.get_gpu(ctx, mat.occlusion_texture.clone());
            let emissive_texture = image_cache.get_gpu(ctx, mat.emissive_texture.clone());
            let bindgroup = render::BindGroupBuilder::new(self.bindgroup_layout.clone())
                .entries(vec![
                    // camera
                    render::BindGroupEntry::Buffer(camera_buffer.buffer()),
                    // lights
                    render::BindGroupEntry::Buffer(lights.buffer()),
                    // instances
                    render::BindGroupEntry::Buffer(self.transforms.buffer()),
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
                ])
                .build(ctx);

            draws.push((gpu_mesh, bindgroup));
            ranges.push(index);
            prev_mesh = Some(mesh_handle.clone());
        }
        ranges.push(self.frame_meshes.len());

        self.transforms.write(ctx, &instances);

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
        mesh: AssetHandle<render::Mesh>,
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
        mesh_cache: &mut AssetCache<render::Mesh, render::GpuMesh>,
    ) {
        for (mesh_handle, _, transform) in self.frame_meshes.iter() {
            let gpu_mesh = mesh_cache.get_gpu(ctx, mesh_handle.clone());
            let bounding_sphere = BoundingSphere::new(&gpu_mesh.bounds, transform);

            gizmo_renderer.draw_sphere(
                &Transform3D::new(
                    bounding_sphere.center,
                    transform.rot,
                    Vec3::ONE * bounding_sphere.radius * 2.0,
                ),
                WHITE.xyz(),
            );
        }
    }

    pub fn required_attributes(&self) -> &BTreeSet<render::VertexAttributeId> {
        &self.vertex_attributes
    }
}

//
// GPU types
//

#[derive(Clone)]
pub struct GpuMaterial {
    pub base_color_texture: AssetHandle<Image>,
    pub color_factor: [f32; 4],

    pub metallic_roughness_texture: AssetHandle<Image>,
    pub roughness_factor: f32,
    pub metallic_factor: f32,

    pub occlusion_texture: AssetHandle<Image>,
    pub occlusion_strength: f32,

    pub normal_texture: AssetHandle<Image>,
    pub normal_scale: f32,

    pub emissive_texture: AssetHandle<Image>,
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
        // TODO: part of context?
        image_cache: &mut AssetCache<Image, GpuImage>,
        pixel_cache: &mut PixelCache,
    ) -> GpuMaterial {
        const BASE_COLOR_DEFAULT: [u8; 4] = [255, 255, 255, 255];
        const NORMAL_DEFAULT: [u8; 4] = [128, 128, 255, 0];
        const METALLIC_ROUGHNESS_DEFAULT: [u8; 4] = [0, 255, 0, 0];
        const OCCLUSION_DEFAULT: [u8; 4] = [255, 0, 0, 0];
        const EMISSIVE_DEFAULT: [u8; 4] = [0, 0, 0, 0];
        fn alloc(
            image_cache: &mut AssetCache<Image, GpuImage>,
            pixel_cache: &mut PixelCache,
            tex: Option<Image>,
            default: [u8; 4],
        ) -> AssetHandle<Image> {
            if let Some(tex) = tex {
                image_cache.allocate(tex)
            } else {
                pixel_cache.allocate(image_cache, default)
            }
        }
        let base_color_texture = alloc(
            image_cache,
            pixel_cache,
            self.base_color_texture,
            BASE_COLOR_DEFAULT,
        );
        let normal_texture = alloc(
            image_cache,
            pixel_cache,
            self.normal_texture,
            NORMAL_DEFAULT,
        );
        let metallic_roughness_texture = alloc(
            image_cache,
            pixel_cache,
            self.metallic_roughness_texture,
            METALLIC_ROUGHNESS_DEFAULT,
        );
        let occlusion_texture = alloc(
            image_cache,
            pixel_cache,
            self.occlusion_texture,
            OCCLUSION_DEFAULT,
        );
        let emissive_texture = alloc(
            image_cache,
            pixel_cache,
            self.emissive_texture,
            EMISSIVE_DEFAULT,
        );

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
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Instances {
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
}
