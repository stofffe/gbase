use gltf::{
    accessor::{DataType, Dimensions},
    Semantic,
};

use crate::{render, Context};

pub struct GpuModel {
    pub primitives: Vec<GpuPrimitive>,
}

pub struct GpuPrimitive {
    pub mesh: GpuMesh,
    pub material: GpuMaterial,
    pub bindgroup: wgpu::BindGroup,
    pub bindgroup_layout: wgpu::BindGroupLayout,
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
        model_transform: &render::UniformBuffer,
    ) -> Self {
        let primitives = model
            .meshes
            .into_iter()
            .map(|primitive| GpuPrimitive::new(ctx, primitive, camera_buffer, model_transform))
            .collect::<Vec<_>>();
        Self { primitives }
    }
}

impl GpuPrimitive {
    pub fn new(
        ctx: &Context,
        primitive: Primitive,
        camera_buffer: &render::UniformBuffer,
        model_transform: &render::UniformBuffer,
    ) -> Self {
        let material = &primitive.material;

        let texture_usage = wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST;
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

        let mesh = &primitive.mesh;
        let vertex_buffer = render::VertexBufferBuilder::new(&mesh.vertices).build(ctx);
        let index_buffer = render::IndexBufferBuilder::new(&mesh.indices).build(ctx);

        let sampler = render::SamplerBuilder::new().build(ctx);
        let (bindgroup_layout, bindgroup) = render::BindGroupCombinedBuilder::new()
            .entries(&[
                // normal
                render::BindGroupCombinedEntry::new(normal_texture.resource())
                    .visibility(wgpu::ShaderStages::FRAGMENT)
                    .ty(normal_texture.binding_type()),
                // albedo
                render::BindGroupCombinedEntry::new(albedo_texture.resource())
                    .visibility(wgpu::ShaderStages::FRAGMENT)
                    .ty(albedo_texture.binding_type()),
                // roughness
                render::BindGroupCombinedEntry::new(roughness_texture.resource())
                    .visibility(wgpu::ShaderStages::FRAGMENT)
                    .ty(roughness_texture.binding_type()),
                // sampler
                render::BindGroupCombinedEntry::new(sampler.resource())
                    .visibility(wgpu::ShaderStages::FRAGMENT)
                    .ty(sampler.binding_filtering()),
                // transform
                render::BindGroupCombinedEntry::new(model_transform.buf().as_entire_binding())
                    .visibility(wgpu::ShaderStages::VERTEX)
                    .uniform(),
                // camera
                render::BindGroupCombinedEntry::new(camera_buffer.buf().as_entire_binding())
                    .visibility(wgpu::ShaderStages::VERTEX_FRAGMENT)
                    .uniform(),
            ])
            .build(ctx);
        Self {
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
            bindgroup_layout,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Primitive {
    pub mesh: Mesh,
    pub material: Material,
}

impl Primitive {
    pub fn new(mesh: Mesh, material: Material) -> Self {
        Self { mesh, material }
    }
}

#[derive(Debug)]
pub struct Model {
    pub meshes: Vec<Primitive>,
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

impl Material {
    pub fn roughness_value_u8(&self) -> [u8; 4] {
        [
            255,
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
}

#[derive(Debug, Clone)]
pub struct Mesh {
    pub vertices: Vec<render::VertexFull>,
    pub indices: Vec<u32>,
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

fn traverse(node: gltf::Node<'_>, depth: usize) {
    eprintln!("{} {:?}", " ".repeat(depth), node.name());
    for child in node.children() {
        traverse(child, depth + 1);
    }
}

pub fn load_glb(ctx: &Context, glb_bytes: &[u8]) -> Model {
    let glb = gltf::Glb::from_slice(glb_bytes).unwrap();
    let info = gltf::Gltf::from_slice(&glb.json).unwrap();
    let buffer = glb.bin.expect("no buffer");

    // eprintln!("{:?}", &info.nodes());

    for (i, scene) in info.scenes().enumerate() {
        for (j, node) in scene.nodes().enumerate() {
            traverse(node, 0);
            // let mesh = node.mesh();
            // eprintln!(
            //     "SCENE {i} NODE {j} NAME {:?} MESH {}",
            //     node.name(),
            //     mesh.is_some()
            // );
        }
    }

    let mut meshes = Vec::new();
    for mesh in info.meshes() {
        for prim in mesh.primitives() {
            // Load indices
            let ind = prim.indices().unwrap();
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

            for (sem, acc) in prim.attributes() {
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
            let mut material = Material::default();
            let mat = prim.material();
            let metallic_roughness = mat.pbr_metallic_roughness();

            material.color_factor = metallic_roughness.base_color_factor();
            material.roughness_factor = metallic_roughness.roughness_factor();
            material.metalness_factor = metallic_roughness.metallic_factor();

            let mut vertices = Vec::new();
            for pos in positions.iter() {
                vertices.push(render::VertexFull {
                    position: [pos.0, pos.1, pos.2],
                    color: material.color_factor,
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

            meshes.push(Primitive {
                mesh: Mesh { vertices, indices },
                material,
            });
        }
    }

    Model { meshes }
}
