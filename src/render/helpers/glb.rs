use crate::{filesystem, render, Context};
use glam::{Mat4, Quat, Vec3};
use gltf::{
    accessor::{DataType, Dimensions},
    Semantic,
};

//
// GPU
//

pub struct GpuModel {
    pub nodes: Vec<GpuModelNode>,
}

pub struct GpuModelNode {
    pub parent: usize,
    pub local_transform: render::Transform,
    pub global_transform: render::Transform,

    pub mesh: Option<GpuModelNodeMesh>,
}

pub struct GpuModelNodeMesh {
    pub mesh: GpuMesh,
    pub material: GpuMaterial,
    pub bindgroup: wgpu::BindGroup,
}

pub struct GpuMaterial {
    pub albedo_texture: render::Texture,
    pub normal_texture: render::Texture,
    pub roughness_texture: render::Texture,
}

pub struct GpuMesh {
    pub vertex_buffer: render::VertexBuffer<render::VertexFull>,
    pub index_buffer: render::IndexBuffer,
}

impl GpuModel {
    pub fn from_model(
        ctx: &Context,
        model: Model,
        camera_buffer: &render::UniformBuffer,
        mesh_renderer: &render::MeshRenderer,
    ) -> Self {
        let nodes = model
            .meshes
            .into_iter()
            .map(|node| GpuModelNode::new(ctx, node, camera_buffer, mesh_renderer))
            .collect::<Vec<_>>();
        Self { nodes }
    }
}

impl GpuModelNode {
    pub fn new(
        ctx: &Context,
        node: ModelNode,
        camera_buffer: &render::UniformBuffer,
        mesh_renderer: &MeshRenderer,
    ) -> Self {
        let transform = node.global_transform.clone();
        let transform_buffer =
            render::UniformBufferBuilder::new().build_init(ctx, &transform.uniform());

        match node.mesh {
            None => Self {
                parent: node.parent,
                local_transform: node.local_transform.clone(),
                global_transform: node.global_transform.clone(),
                mesh: None,
            },
            Some((mesh, material)) => {
                let texture_usage =
                    wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST;
                let albedo_texture = if let Some(bytes) = &material.albedo {
                    render::TextureBuilder::new()
                        .usage(texture_usage)
                        .build_init(ctx, bytes)
                } else {
                    let color = material.color_factor.map(|a| (a * 255.0) as u8);
                    render::TextureBuilder::new()
                        .usage(texture_usage)
                        .build_single_pixel(ctx, color)
                };
                let normal_texture = if let Some(bytes) = &material.normal {
                    render::TextureBuilder::new()
                        .usage(texture_usage)
                        .build_init(ctx, bytes)
                } else {
                    let default_normal = [128, 128, 255, 128];
                    render::TextureBuilder::new()
                        .usage(texture_usage)
                        .build_single_pixel(ctx, default_normal)
                };
                let roughness_texture = if let Some(bytes) = &material.roughness {
                    render::TextureBuilder::new()
                        .usage(texture_usage)
                        .build_init(ctx, bytes)
                } else {
                    let default_roughness = material.roughness_value_u8();
                    render::TextureBuilder::new()
                        .usage(texture_usage)
                        .build_single_pixel(ctx, default_roughness)
                };

                let vertex_buffer = render::VertexBufferBuilder::new(&mesh.vertices).build(ctx);
                let index_buffer = render::IndexBufferBuilder::new(&mesh.indices).build(ctx);

                let sampler = render::SamplerBuilder::new().build(ctx);
                let bindgroup = render::BindGroupBuilder::new()
                    .entries(&[
                        render::BindGroupEntry::new(sampler.resource()),
                        render::BindGroupEntry::new(normal_texture.resource()),
                        render::BindGroupEntry::new(albedo_texture.resource()),
                        render::BindGroupEntry::new(roughness_texture.resource()),
                        render::BindGroupEntry::new(transform_buffer.resource()),
                        render::BindGroupEntry::new(camera_buffer.resource()),
                    ])
                    .build(ctx, &mesh_renderer.bindgroup_layout);

                Self {
                    parent: node.parent,
                    local_transform: node.local_transform,
                    global_transform: node.global_transform,

                    mesh: Some(GpuModelNodeMesh {
                        mesh: GpuMesh {
                            vertex_buffer,
                            index_buffer,
                        },
                        material: GpuMaterial {
                            albedo_texture,
                            normal_texture,
                            roughness_texture,
                        },
                        bindgroup,
                    }),
                }
            }
        }
    }
}

//
// CPU
//

#[derive(Debug)]
pub struct Model {
    pub meshes: Vec<ModelNode>,
}

#[derive(Debug)]
pub struct ModelNode {
    pub mesh: Option<(Mesh, Material)>,
    pub local_transform: render::Transform,
    pub global_transform: render::Transform,
    pub parent: usize,
}

#[derive(Debug, Clone)]
pub struct Mesh {
    pub vertices: Vec<render::VertexFull>,
    pub indices: Vec<u32>,
}

#[derive(Debug, Clone)]
pub struct Material {
    pub color_factor: [f32; 4],
    pub roughness_factor: f32,
    pub metalness_factor: f32,
    pub occlusion_strength: f32,

    pub albedo: Option<Vec<u8>>,
    pub normal: Option<Vec<u8>>,
    pub roughness: Option<Vec<u8>>,
}

impl Model {
    pub fn from_glb_bytes(bytes: &[u8]) -> Self {
        parse_glb(bytes)
    }
}

impl Material {
    pub fn roughness_value_u8(&self) -> [u8; 4] {
        [
            self.occlusion_strength_u8(),
            self.roughness_factor_u8(),
            self.metalness_factor_u8(),
            0,
        ]
    }
    pub fn color_factor_u8(&self) -> [u8; 4] {
        self.color_factor.map(|v| (v * 255.0) as u8)
    }
    pub fn roughness_factor_u8(&self) -> u8 {
        (self.roughness_factor * 255.0) as u8
    }
    pub fn metalness_factor_u8(&self) -> u8 {
        (self.metalness_factor * 255.0) as u8
    }
    pub fn occlusion_strength_u8(&self) -> u8 {
        (self.occlusion_strength * 255.0) as u8
    }
}

impl Default for Material {
    fn default() -> Self {
        Self {
            color_factor: [1.0, 1.0, 1.0, 1.0],
            roughness_factor: 0.0,
            metalness_factor: 0.0,
            occlusion_strength: 1.0,

            albedo: None,
            normal: None,
            roughness: None,
        }
    }
}

fn parse_glb(bytes: &[u8]) -> Model {
    let glb = gltf::Glb::from_slice(bytes).unwrap();
    let info = gltf::Gltf::from_slice(&glb.json).unwrap();
    let buffer = glb.bin.expect("no buffer in glb file");

    let mut meshes = Vec::new();
    for scene in info.scenes() {
        for node in scene.nodes() {
            parse_scene(node, &buffer, &mut meshes, render::Transform::default(), 0);
        }
    }

    Model { meshes }
}

fn parse_scene(
    node: gltf::Node<'_>,
    buffer: &[u8],
    nodes: &mut Vec<ModelNode>,
    parent_transform: render::Transform,
    parent: usize,
) {
    let index = nodes.len();
    let local_transform = parse_transform(node.transform());
    let global_transform =
        render::Transform::from_matrix(parent_transform.matrix() * local_transform.matrix());

    eprintln!("Transform {:?}", global_transform);

    match node.mesh() {
        Some(mesh) => {
            for primitive in mesh.primitives() {
                let mesh = parse_mesh(buffer, &primitive);
                let material = parse_material(buffer, &primitive);
                nodes.push(ModelNode {
                    mesh: Some((mesh, material)),
                    local_transform: local_transform.clone(),
                    global_transform: global_transform.clone(),
                    parent,
                });
            }
        }
        None => {
            nodes.push(ModelNode {
                mesh: None,
                local_transform: local_transform.clone(),
                global_transform: global_transform.clone(),
                parent,
            });
        }
    }
    for child in node.children() {
        parse_scene(child, buffer, nodes, global_transform.clone(), index);
    }
}

// fn traverse_gltf(node: gltf::Node<'_>, depth: usize) {
//     let prefix = " ".repeat(depth);
//     eprintln!("{prefix} {:?}", node.name());
//     eprintln!("{prefix} {:?}", node.transform());
//     for child in node.children() {
//         traverse_gltf(child, depth + 1);
//     }
// }

fn parse_mesh(buffer: &[u8], primitive: &gltf::Primitive<'_>) -> Mesh {
    // Load indices
    let ind = primitive.indices().unwrap();
    let view = ind.view().unwrap();
    let ind_off = view.offset() + ind.offset();
    let ind_size = ind.count() * ind.size();
    let indices = match (ind.data_type(), ind.dimensions()) {
        (gltf::accessor::DataType::U8, gltf::accessor::Dimensions::Scalar) => {
            let inds: &[u8] = bytemuck::cast_slice(&buffer[ind_off..ind_off + ind_size]);
            inds.iter().map(|&i| i as u32).collect::<Vec<_>>()
        }
        (gltf::accessor::DataType::U16, gltf::accessor::Dimensions::Scalar) => {
            let inds: &[u16] = bytemuck::cast_slice(&buffer[ind_off..ind_off + ind_size]);
            inds.iter().map(|&i| i as u32).collect::<Vec<_>>()
        }
        (gltf::accessor::DataType::U32, gltf::accessor::Dimensions::Scalar) => {
            let inds: &[u32] = bytemuck::cast_slice(&buffer[ind_off..ind_off + ind_size]);
            inds.to_vec()
        }
        form => {
            panic!("cringe index format {form:?}")
        }
    };

    // Load pos, albedo, normal, tangent
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut tangents = Vec::new();
    let mut uvs = Vec::new();

    for (sem, acc) in primitive.attributes() {
        let view = acc.view().unwrap();
        let offset = acc.offset() + view.offset();
        let size = acc.count() * acc.size();
        let typ = acc.data_type();
        let dimension = acc.dimensions();

        match (sem, typ, dimension) {
            (Semantic::Positions, DataType::F32, Dimensions::Vec3) => {
                let buf: &[f32] = bytemuck::cast_slice(&buffer[offset..offset + size]);
                for pos in buf.chunks(3) {
                    positions.push((pos[0], pos[1], pos[2]));
                }
                eprintln!("POS {:?}", buf.len());
            }
            (Semantic::Normals, DataType::F32, Dimensions::Vec3) => {
                let buf: &[f32] = bytemuck::cast_slice(&buffer[offset..offset + size]);
                for normal in buf.chunks(3) {
                    normals.push((normal[0], normal[1], normal[2]))
                }
                eprintln!("NORMAL {:?}", buf.len());
            }
            (Semantic::Tangents, DataType::F32, Dimensions::Vec4) => {
                let buf: &[f32] = bytemuck::cast_slice(&buffer[offset..offset + size]);
                for tangent in buf.chunks(4) {
                    tangents.push((tangent[0], tangent[1], tangent[2], tangent[3]));
                }
                eprintln!("TANGENT {:?}", buf.len());
            }
            (Semantic::Colors(_), DataType::F32, Dimensions::Vec3) => {
                let buf: &[f32] = bytemuck::cast_slice(&buffer[offset..offset + size]);
                eprintln!("COLOR {:?}", buf.len());
            }
            (Semantic::TexCoords(i), DataType::F32, Dimensions::Vec2) => {
                if i == 0 {
                    let buf: &[f32] = bytemuck::cast_slice(&buffer[offset..offset + size]);
                    for uv in buf.chunks(2) {
                        uvs.push((uv[0], uv[1]))
                    }
                    eprintln!("UV({i}) {:?}", buf.len());
                }
            }
            info => log::warn!("cringe type: {:?}", info),
        }
    }

    eprintln!("Indices {}", indices.len());
    eprintln!("Positions {}", positions.len());
    eprintln!("Normals {}", normals.len());
    eprintln!("Uvs {}", uvs.len());
    eprintln!("Tangents {}", tangents.len());

    assert!(!positions.is_empty());
    assert!(!normals.is_empty());
    assert!(!tangents.is_empty());
    assert!(!uvs.is_empty());

    // Material
    let color = primitive
        .material()
        .pbr_metallic_roughness()
        .base_color_factor();

    let mut vertices = Vec::new();
    for pos in positions.iter() {
        vertices.push(render::VertexFull {
            position: [pos.0, pos.1, pos.2],
            color,
            normal: [0.0, 0.0, 0.0],
            uv: [0.0, 0.0],
            tangent: [0.0, 0.0, 0.0, 0.0],
        });
    }
    for (i, normal) in normals.iter().enumerate() {
        vertices[i].normal = [normal.0, normal.1, normal.2];
    }
    for (i, uv) in uvs.iter().enumerate() {
        vertices[i].uv = [uv.0, uv.1];
    }
    for (i, tangent) in tangents.iter().enumerate() {
        vertices[i].tangent = [tangent.0, tangent.1, tangent.2, tangent.3];
    }

    Mesh { vertices, indices }
}

fn parse_material(buffer: &[u8], primitive: &gltf::Primitive<'_>) -> Material {
    let mut material = Material::default();

    let mat = primitive.material();
    let metallic_roughness = mat.pbr_metallic_roughness();

    material.color_factor = metallic_roughness.base_color_factor();
    material.roughness_factor = metallic_roughness.roughness_factor();
    material.metalness_factor = metallic_roughness.metallic_factor();

    // Normal texture
    if let Some(normal_texture) = mat.normal_texture() {
        eprintln!("Normal texture coord {}", normal_texture.tex_coord());
        match normal_texture.texture().source().source() {
            gltf::image::Source::View { view, .. } => {
                let img_buf = &buffer[view.offset()..view.offset() + view.length()];
                material.normal = Some(img_buf.to_vec());
            }
            gltf::image::Source::Uri { .. } => {
                eprintln!("Normal texture URI");
            }
        };
    }

    // Albedo texture
    if let Some(base_color_texture) = metallic_roughness.base_color_texture() {
        eprintln!("Albedo texture coord {}", base_color_texture.tex_coord());
        match base_color_texture.texture().source().source() {
            gltf::image::Source::View { view, .. } => {
                let img_buf = &buffer[view.offset()..view.offset() + view.length()];
                material.albedo = Some(img_buf.to_vec());
            }
            gltf::image::Source::Uri { .. } => {
                eprintln!("Albedo texture URI");
            }
        };
    }

    // AO Metallic Roughness
    if let Some(roughness_texture) = metallic_roughness.metallic_roughness_texture() {
        eprintln!(
            "Roughness texture coord {} index {}",
            roughness_texture.tex_coord(),
            roughness_texture.texture().index()
        );
        match roughness_texture.texture().source().source() {
            gltf::image::Source::View { view, .. } => {
                let img_buf = &buffer[view.offset()..view.offset() + view.length()];
                material.roughness = Some(img_buf.to_vec());
            }
            gltf::image::Source::Uri { .. } => {
                eprintln!("Roughness texture URI");
            }
        };
    }

    // Occlusion (included in roughness)
    if let Some(occlusion_texture) = mat.occlusion_texture() {
        eprintln!(
            "Occlusion texture coord {} index {}",
            occlusion_texture.tex_coord(),
            occlusion_texture.texture().index()
        );
        material.occlusion_strength = occlusion_texture.strength();
    }

    material
}

fn parse_transform(transform: gltf::scene::Transform) -> render::Transform {
    match transform {
        gltf::scene::Transform::Matrix { matrix } => {
            let a = Mat4::from_cols_array_2d(&matrix);
            let (scale, rot, pos) = a.to_scale_rotation_translation();
            render::Transform::new(pos, rot, scale)
        }
        gltf::scene::Transform::Decomposed {
            translation,
            rotation,
            scale,
        } => render::Transform::new(
            Vec3::from_array(translation),
            Quat::from_array(rotation),
            Vec3::from_array(scale),
        ),
    }
}

//
// Renderer
//

pub struct MeshRenderer {
    pipeline: wgpu::RenderPipeline,
    bindgroup_layout: wgpu::BindGroupLayout,
}

impl MeshRenderer {
    pub async fn new(ctx: &Context, deferred_buffers: &render::DeferredBuffers) -> Self {
        let bindgroup_layout = render::BindGroupLayoutBuilder::new()
            .entries(&[
                // Sampler
                render::BindGroupLayoutEntry::new()
                    .fragment()
                    .sampler_filtering(),
                // Normal
                render::BindGroupLayoutEntry::new()
                    .fragment()
                    .texture_float(true),
                // Albedo
                render::BindGroupLayoutEntry::new()
                    .fragment()
                    .texture_float(true),
                // Albedo
                render::BindGroupLayoutEntry::new()
                    .fragment()
                    .texture_float(true),
                // Transform
                render::BindGroupLayoutEntry::new().vertex().uniform(),
                // Camera
                render::BindGroupLayoutEntry::new()
                    .vertex()
                    .fragment()
                    .uniform(),
            ])
            .build(ctx);

        let shader_str = filesystem::load_string(ctx, "mesh.wgsl").await.unwrap();
        let shader = render::ShaderBuilder::new().build(ctx, &shader_str);
        let pipeline = render::RenderPipelineBuilder::new(&shader)
            .buffers(&[render::VertexFull::desc()])
            .targets(&deferred_buffers.targets())
            .bind_groups(&[&bindgroup_layout])
            .depth_stencil(deferred_buffers.depth_stencil_state())
            .cull_mode(wgpu::Face::Back)
            .build(ctx);

        Self {
            pipeline,
            bindgroup_layout,
        }
    }

    pub fn render(
        &mut self,
        _ctx: &render::Context,
        encoder: &mut wgpu::CommandEncoder,
        deferred_buffers: &render::DeferredBuffers,
        models: &[&render::GpuModel],
    ) {
        let color_attachments = deferred_buffers.color_attachments();
        let mut mesh_pass = render::RenderPassBuilder::new()
            .color_attachments(&color_attachments)
            .depth_stencil_attachment(deferred_buffers.depth_stencil_attachment_clear())
            .build(encoder);

        mesh_pass.set_pipeline(&self.pipeline);

        for model in models.iter() {
            for prim in model.nodes.iter() {
                match &prim.mesh {
                    None => continue,
                    Some(mesh_node) => {
                        let mesh = &prim.mesh.as_ref().unwrap().mesh;
                        mesh_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                        mesh_pass.set_index_buffer(
                            mesh.index_buffer.slice(..),
                            mesh.index_buffer.format(),
                        );
                        mesh_pass.set_bind_group(0, &mesh_node.bindgroup, &[]);
                        mesh_pass.draw_indexed(0..mesh.index_buffer.len(), 0, 0..1);
                    }
                }
            }
        }
    }
}
