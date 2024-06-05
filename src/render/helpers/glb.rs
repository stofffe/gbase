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
        let albedo_bytes = material.albedo.as_ref().unwrap();
        let normal_bytes = material.normal.as_ref().unwrap();
        let roughness_bytes = material.roughness.as_ref().unwrap();

        let texture_usage = wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST;
        let albedo_texture = render::TextureBuilder::new()
            .usage(texture_usage)
            .build_init(ctx, albedo_bytes);
        let normal_texture = render::TextureBuilder::new()
            .usage(texture_usage)
            .build_init(ctx, normal_bytes);
        let roughness_texture = render::TextureBuilder::new()
            .usage(texture_usage)
            .build_init(ctx, roughness_bytes);

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
                // camera
                render::BindGroupCombinedEntry::new(camera_buffer.buf().as_entire_binding())
                    .visibility(wgpu::ShaderStages::VERTEX_FRAGMENT)
                    .uniform(),
                // transform
                render::BindGroupCombinedEntry::new(model_transform.buf().as_entire_binding())
                    .visibility(wgpu::ShaderStages::VERTEX)
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

#[derive(Debug)]
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

#[derive(Debug)]
pub struct Material {
    pub color_factor: [f32; 4],
    pub roughness_factor: f32,
    pub metalness_factor: f32,
    pub albedo: Option<Vec<u8>>,
    pub normal: Option<Vec<u8>>,
    pub roughness: Option<Vec<u8>>,
}

#[derive(Debug)]
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
            albedo: None,
            normal: None,
            roughness: None,
        }
    }
}

pub fn load_glb(ctx: &Context, glb_bytes: &[u8]) -> Model {
    let glb = gltf::Glb::from_slice(glb_bytes).unwrap();
    let info = gltf::Gltf::from_slice(&glb.json).unwrap();
    let buffer = glb.bin.expect("no buffer");

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
            eprintln!("INDEX {}", indices.len());

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
                    (
                        gltf::Semantic::Positions,
                        gltf::accessor::DataType::F32,
                        gltf::accessor::Dimensions::Vec3,
                    ) => {
                        let buf: &[f32] = bytemuck::cast_slice(&buffer[offset..offset + size]);
                        for pos in buf.chunks(3) {
                            positions.push((pos[0], pos[1], pos[2]));
                        }
                        eprintln!("POS {:?}", buf.len());
                    }
                    (
                        gltf::Semantic::Normals,
                        gltf::accessor::DataType::F32,
                        gltf::accessor::Dimensions::Vec3,
                    ) => {
                        let buf: &[f32] = bytemuck::cast_slice(&buffer[offset..offset + size]);
                        for normal in buf.chunks(3) {
                            normals.push((normal[0], normal[1], normal[2]))
                        }
                        eprintln!("NORMAL {:?}", buf.len());
                    }
                    (
                        gltf::Semantic::Tangents,
                        gltf::accessor::DataType::F32,
                        gltf::accessor::Dimensions::Vec4,
                    ) => {
                        let buf: &[f32] = bytemuck::cast_slice(&buffer[offset..offset + size]);
                        for tangent in buf.chunks(4) {
                            tangents.push((tangent[0], tangent[1], tangent[2], tangent[3]));
                            // TODO eprintln!("HAND {}", tangent[3]);
                        }
                        eprintln!("TANGENT {:?}", buf.len());
                    }
                    (
                        gltf::Semantic::Colors(_),
                        gltf::accessor::DataType::F32,
                        gltf::accessor::Dimensions::Vec3,
                    ) => {
                        let buf: &[f32] = bytemuck::cast_slice(&buffer[offset..offset + size]);
                        eprintln!("COLOR {:?}", buf.len());
                    }
                    (
                        gltf::Semantic::TexCoords(i),
                        gltf::accessor::DataType::F32,
                        gltf::accessor::Dimensions::Vec2,
                    ) => {
                        let buf: &[f32] = bytemuck::cast_slice(&buffer[offset..offset + size]);
                        for uv in buf.chunks(2) {
                            uvs.push((uv[0], uv[1]))
                        }
                        eprintln!("UV({i}) {:?}", buf.len());
                    }
                    info => log::warn!("cringe type: {:?}", info),
                }
            }

            // Material
            let mut material = Material::default();
            let mat = prim.material();
            let metallic_roughness = mat.pbr_metallic_roughness();

            material.color_factor = metallic_roughness.base_color_factor();
            material.roughness_factor = metallic_roughness.roughness_factor();
            material.metalness_factor = metallic_roughness.metallic_factor();

            let mut vertices = Vec::new();
            for (((pos, normal), uv), tangent) in
                positions.into_iter().zip(normals).zip(uvs).zip(tangents)
            {
                vertices.push(render::VertexFull {
                    position: [pos.0, pos.1, pos.2],
                    normal: [normal.0, normal.1, normal.2],
                    color: material.color_factor,
                    uv: [uv.0, uv.1],
                    tangent: [tangent.0, tangent.1, tangent.2, tangent.3],
                });
            }

            // Normal texture
            if let Some(normal_texture) = mat.normal_texture() {
                if let gltf::image::Source::View { view, .. } =
                    normal_texture.texture().source().source()
                {
                    let img_buf = &buffer[view.offset()..view.offset() + view.length()];
                    material.normal = Some(img_buf.to_vec());
                    // material.normal = Some(
                    //     render::TextureBuilder::new()
                    //         .usage(texture_usage)
                    //         .build_init(ctx, img_buf),
                    // );
                }
            }

            // Albedo texture
            if let Some(base_color_texture) = metallic_roughness.base_color_texture() {
                if let gltf::image::Source::View { view, .. } =
                    base_color_texture.texture().source().source()
                {
                    let img_buf = &buffer[view.offset()..view.offset() + view.length()];
                    material.albedo = Some(img_buf.to_vec());
                }
            }

            // Metal
            if let Some(roughness_texture) = metallic_roughness.metallic_roughness_texture() {
                if let gltf::image::Source::View { view, .. } =
                    roughness_texture.texture().source().source()
                {
                    let img_buf = &buffer[view.offset()..view.offset() + view.length()];
                    material.roughness = Some(img_buf.to_vec());
                }
            }

            meshes.push(Primitive {
                mesh: Mesh { vertices, indices },
                material,
            });
        }
    }

    Model { meshes }
}

// eprintln!("IMAGES {}", info.images().len());
// for image in info.images() {
//     // eprintln!("{:?}", image.source());
//     match image.source() {
//         gltf::image::Source::View { view, .. } => {
//             // let img_buf = &buffer[view.offset()..view.offset() + view.length()];
//             // material.albedo = Some(render::TextureBuilder::new().build_init(ctx, img_buf));
//             let img_buf = &buffer[view.offset()..view.offset() + view.length()];
//             material.other.push(
//                 render::TextureBuilder::new()
//                     .usage(wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::COPY_SRC)
//                     .build_init(ctx, img_buf),
//             );
//         }
//         gltf::image::Source::Uri { uri, .. } => {
//             eprintln!("URI")
//             // let mut path = PathBuf::from("kenney_survival-kit/Models");
//             // path.push(uri);
//             //
//             // eprintln!("{}", path.to_str().unwrap());
//             // let img_buf = filesystem::load_bytes(ctx, path).await.unwrap();
//             //
//             // // let img_buf = fs::read(uri).unwrap();
//             // material.albedo = Some(render::TextureBuilder::new().build_init(ctx, &img_buf));
//         }
//     }
// }

// material
