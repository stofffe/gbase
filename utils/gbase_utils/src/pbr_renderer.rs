//
// GPU types
//

#[derive(Clone)]
pub struct GpuMaterial {
    pub base_color_texture: render::TextureWithView,
    pub color_factor: [f32; 4],

    pub metallic_roughness_texture: render::TextureWithView,
    pub roughness_factor: f32,
    pub metallic_factor: f32,

    pub occlusion_texture: render::TextureWithView,
    pub occlusion_strength: f32,

    pub normal_texture: render::TextureWithView,
    pub normal_scale: f32,
}

impl GpuMaterial {
    pub fn uniform(&self) -> PbrMaterialUniform {
        PbrMaterialUniform {
            color_factor: self.color_factor.into(),
            roughness_factor: self.roughness_factor,
            metallic_factor: self.metallic_factor,
            occlusion_strength: self.occlusion_strength,
            normal_scale: self.normal_scale,
        }
    }
}

//
// Mesh renderer
//

// PBR
// unqiue per draw call
//
// transform
//
// material
//  base color
//  normal
//  rougness
//  occlusion

use std::{collections::BTreeSet, sync::Arc};

use encase::ShaderType;
use gbase::{
    glam::{Vec3, Vec4, Vec4Swizzles},
    log, render, wgpu, Context,
};

use crate::{
    GizmoRenderer, GpuMesh, GpuModel, GrowingBufferArena, Transform3D, TransformUniform,
    VertexAttributeId, WHITE,
};

pub struct PbrRenderer {
    pipeline: render::ArcRenderPipeline,
    bindgroup_layout: render::ArcBindGroupLayout,
    vertex_attributes: BTreeSet<VertexAttributeId>,

    transform_arena: GrowingBufferArena,
    material_arena: GrowingBufferArena,

    frame_meshes: Vec<(Arc<GpuMesh>, Arc<GpuMaterial>, Transform3D)>,
}

impl PbrRenderer {
    pub fn new(ctx: &mut Context, depth_buffer: &render::DepthBuffer) -> Self {
        let shader =
            render::ShaderBuilder::new(include_str!("../assets/shaders/mesh.wgsl")).build(ctx);

        let bindgroup_layout = render::BindGroupLayoutBuilder::new()
            .entries(vec![
                // camera
                render::BindGroupLayoutEntry::new()
                    .uniform()
                    .vertex()
                    .fragment(),
                // lights
                render::BindGroupLayoutEntry::new().uniform().fragment(),
                // transform
                render::BindGroupLayoutEntry::new().uniform().vertex(),
                // pbr material
                render::BindGroupLayoutEntry::new().uniform().fragment(),
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
            ])
            .build(ctx);

        let vertex_attributes = BTreeSet::from([
            VertexAttributeId::Position,
            VertexAttributeId::Normal,
            VertexAttributeId::Uv(0),
            VertexAttributeId::Tangent,
            VertexAttributeId::Color(0),
        ]);
        let mut buffers = Vec::new();
        for attr in vertex_attributes.iter() {
            buffers.push(render::VertexBufferLayout::from_vertex_formats(
                wgpu::VertexStepMode::Vertex,
                vec![attr.format()],
            ));
        }

        let pipeline_layout = render::PipelineLayoutBuilder::new()
            .bind_groups(vec![bindgroup_layout.clone()])
            .build(ctx);
        let pipeline = render::RenderPipelineBuilder::new(shader, pipeline_layout)
            .buffers(buffers)
            .single_target(render::ColorTargetState::from_current_screen(ctx))
            .cull_mode(wgpu::Face::Back)
            .depth_stencil(depth_buffer.depth_stencil_state())
            .build(ctx);

        // let size = dbg!(u64::from(TransformUniform::min_size()));
        let transform_size = u64::from(TransformUniform::min_size()).next_multiple_of(256);
        const DRAWS_MAX: u64 = 4096;
        let transform_arena = GrowingBufferArena::new(
            render::device(ctx),
            transform_size,
            wgpu::BufferDescriptor {
                label: None,
                size: transform_size * DRAWS_MAX,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            },
        );
        let material_size = u64::from(PbrMaterialUniform::min_size()).next_multiple_of(256);
        let material_arena = GrowingBufferArena::new(
            render::device(ctx),
            transform_size,
            wgpu::BufferDescriptor {
                label: None,
                size: material_size * DRAWS_MAX,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            },
        );

        Self {
            pipeline,
            bindgroup_layout,
            vertex_attributes,
            transform_arena,
            material_arena,
            frame_meshes: Vec::new(),
        }
    }

    pub fn add_mesh(
        &mut self,
        mesh: Arc<GpuMesh>,
        material: Arc<GpuMaterial>,
        transform: Transform3D,
    ) {
        self.frame_meshes.push((mesh, material, transform));
    }

    pub fn add_model(&mut self, model: &GpuModel, global_transform: Transform3D) {
        for (mesh, material, transform) in model.meshes.iter() {
            let final_transform =
                Transform3D::from_matrix(global_transform.matrix() * transform.matrix());
            self.frame_meshes
                .push((mesh.clone(), material.clone(), final_transform));
        }
    }

    // temp?
    pub fn render_bounding_boxes(&self, gizmo_renderer: &mut GizmoRenderer) {
        for (gpu_mesh, _, transform) in self.frame_meshes.iter() {
            let bounding_radius = gpu_mesh.bounds.bounding_radius();

            gizmo_renderer.draw_sphere(
                &Transform3D::new(
                    transform.pos,
                    transform.rot,
                    Vec3::ONE * bounding_radius * 2.0,
                ),
                WHITE.xyz(),
            );
        }
    }

    // TODO: should have internal vector<(mesh mat transform)>
    pub fn render(
        &mut self,
        ctx: &mut Context,
        view: &wgpu::TextureView,
        camera: &crate::Camera,
        camera_buffer: &render::UniformBuffer<crate::CameraUniform>,
        lights: &render::UniformBuffer<PbrLightUniforms>,
        depth_buffer: &render::DepthBuffer,
    ) {
        let frustum = camera.calculate_frustum(ctx);

        let mut draws = Vec::new();
        for (gpu_mesh, mat, transform) in self.frame_meshes.iter() {
            // cull
            // TODO: use circles or AABB?
            let bounding_radius = gpu_mesh.bounds.bounding_radius();

            if !frustum.sphere_inside(transform.pos, bounding_radius) {
                continue;
            }

            // transform
            let transform_allocation = self
                .transform_arena
                .allocate(render::device(ctx), TransformUniform::min_size().into());
            let mut transform_buffer = encase::UniformBuffer::new(Vec::new());
            transform_buffer
                .write(&transform.uniform())
                .expect("could not write to transform buffer");

            render::queue(ctx).write_buffer(
                &transform_allocation.buffer,
                transform_allocation.offset,
                &transform_buffer.into_inner(),
            );
            // material
            let material_allocation = self
                .material_arena
                .allocate(render::device(ctx), PbrMaterialUniform::min_size().into());
            let mut material_buffer = encase::UniformBuffer::new(Vec::new());
            material_buffer
                .write(&mat.uniform())
                .expect("could not write to material buffer");
            render::queue(ctx).write_buffer(
                &material_allocation.buffer,
                material_allocation.offset,
                &material_buffer.into_inner(),
            );

            let bindgroup = render::BindGroupBuilder::new(self.bindgroup_layout.clone())
                .entries(vec![
                    // camera
                    render::BindGroupEntry::Buffer(camera_buffer.buffer()),
                    // lights
                    render::BindGroupEntry::Buffer(lights.buffer()),
                    // model
                    render::BindGroupEntry::BufferSlice {
                        buffer: transform_allocation.buffer,
                        offset: transform_allocation.offset,
                        size: TransformUniform::min_size().into(),
                    },
                    // pbr material
                    render::BindGroupEntry::BufferSlice {
                        buffer: material_allocation.buffer,
                        offset: material_allocation.offset,
                        size: PbrMaterialUniform::min_size().into(),
                    },
                    // base color texture
                    render::BindGroupEntry::Texture(mat.base_color_texture.view()),
                    // base color sampler
                    render::BindGroupEntry::Sampler(mat.base_color_texture.sampler()),
                    // normal texture
                    render::BindGroupEntry::Texture(mat.normal_texture.view()),
                    // normal sampler
                    render::BindGroupEntry::Sampler(mat.normal_texture.sampler()),
                    // metallic roughness texture
                    render::BindGroupEntry::Texture(mat.metallic_roughness_texture.view()),
                    // metallic roughness sampler
                    render::BindGroupEntry::Sampler(mat.metallic_roughness_texture.sampler()),
                    // occlusion roughness texture
                    render::BindGroupEntry::Texture(mat.occlusion_texture.view()),
                    // occlusion roughness sampler
                    render::BindGroupEntry::Sampler(mat.occlusion_texture.sampler()),
                ])
                .build(ctx);

            draws.push((bindgroup, gpu_mesh));
        }

        // log::info!("Issuing {:?} draw calls", draws.len());

        // TODO: using one render pass per draw call
        render::RenderPassBuilder::new()
            .color_attachments(&[Some(render::RenderPassColorAttachment::new(view))])
            .depth_stencil_attachment(depth_buffer.depth_render_attachment_load())
            .build_run_submit(ctx, |mut pass| {
                pass.set_pipeline(&self.pipeline);

                for (bindgroup, gpu_mesh) in draws {
                    for (i, (_, (start, end))) in gpu_mesh.attribute_ranges.iter().enumerate() {
                        let slice = gpu_mesh.attribute_buffer.slice(start..end);
                        pass.set_vertex_buffer(i as u32, slice);
                    }
                    pass.set_index_buffer(
                        gpu_mesh.index_buffer.as_ref().unwrap().slice(..),
                        wgpu::IndexFormat::Uint32,
                    );
                    pass.set_bind_group(0, Some(bindgroup.as_ref()), &[]);
                    pass.draw_indexed(0..gpu_mesh.index_count.unwrap(), 0, 0..1);
                }
            });

        self.transform_arena.free();
        self.material_arena.free();
        self.frame_meshes.clear();
    }
    pub fn required_attributes(&self) -> &BTreeSet<VertexAttributeId> {
        &self.vertex_attributes
    }
}

// TODO: shoudl use handles for textures to reuse
// TODO: emissive
#[derive(Debug, Clone, Default)]
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
}

#[derive(Debug, Clone)]
pub struct Image {
    pub texture: render::TextureBuilder,
    pub sampler: render::SamplerBuilder,
}

impl PbrMaterial {
    pub fn new_colored(color: [f32; 4]) -> Self {
        Self {
            color_factor: color,
            ..Default::default()
        }
    }
    pub fn to_material(&self, ctx: &mut Context) -> GpuMaterial {
        // https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#materials-overview
        fn create_texture_or_default(
            ctx: &mut Context,
            texture: &Option<Image>,
            default: [u8; 4],
        ) -> render::TextureWithView {
            if let Some(tex) = texture {
                let texture = tex.texture.clone().build(ctx);
                let sampler = tex.sampler.clone().build(ctx);
                let view = render::TextureViewBuilder::new(texture.clone()).build(ctx);
                render::TextureWithView::new(texture, view, sampler)
            } else {
                let texture =
                    render::TextureBuilder::new(render::TextureSource::Data(1, 1, default.into()))
                        .build(ctx);
                let sampler = render::SamplerBuilder::new()
                    .min_mag_filter(wgpu::FilterMode::Nearest, wgpu::FilterMode::Nearest)
                    .build(ctx);
                let view = render::TextureViewBuilder::new(texture.clone()).build(ctx);
                render::TextureWithView::new(texture, view, sampler)
            }
        }

        let base_color_texture =
            create_texture_or_default(ctx, &self.base_color_texture, [255, 255, 255, 255]);
        let metallic_roughness_texture =
            create_texture_or_default(ctx, &self.metallic_roughness_texture, [0, 255, 0, 0]);
        let normal_texture =
            create_texture_or_default(ctx, &self.normal_texture, [128, 128, 255, 0]);
        let occlusion_texture =
            create_texture_or_default(ctx, &self.occlusion_texture, [255, 0, 0, 0]);

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
        }
    }
}

impl PbrMaterial {
    pub fn uniform(&self) -> PbrMaterialUniform {
        PbrMaterialUniform {
            color_factor: self.color_factor.into(),
            roughness_factor: self.roughness_factor,
            metallic_factor: self.metallic_factor,
            occlusion_strength: self.occlusion_strength,
            normal_scale: self.normal_scale,
        }
    }
}

#[derive(Debug, Clone, ShaderType)]
pub struct PbrMaterialUniform {
    pub color_factor: Vec4,
    pub roughness_factor: f32,
    pub metallic_factor: f32,
    pub occlusion_strength: f32,
    pub normal_scale: f32,
}

//
// lights
//

#[derive(ShaderType)]
pub struct PbrLightUniforms {
    pub main_light_dir: Vec3,
}
