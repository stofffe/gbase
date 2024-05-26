use crate::{render, Context};

pub struct Model {
    pub meshes: Vec<Mesh>,
}

#[derive(Default)]
pub struct Material {
    pub albedo: Option<render::Texture>,
    pub normal: Option<render::Texture>,
    pub roughness: Option<render::Texture>,
    pub other: Vec<render::Texture>,
}

pub struct Mesh {
    pub vertex_buffer: render::VertexBuffer<render::VertexFull>,
    pub index_buffer: render::IndexBuffer,
}

pub fn load_glb(ctx: &Context, glb_bytes: &[u8]) -> (Model, Material) {
    let glb = gltf::Glb::from_slice(glb_bytes).unwrap();
    let info = gltf::Gltf::from_slice(&glb.json).unwrap();
    let buffer = glb.bin.expect("no buffer");

    let mut meshes = Vec::new();
    let mut material = Material::default();
    for mesh in info.meshes() {
        for prim in mesh.primitives() {
            // Load indices
            let view = prim.indices().unwrap().view().unwrap();
            let (ind_size, ind_off) = (view.length(), view.offset());
            let indices = match (
                prim.indices().unwrap().data_type(),
                prim.indices().unwrap().dimensions(),
            ) {
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

            let mut vertices = Vec::new();
            for (((pos, normal), uv), tangent) in
                positions.into_iter().zip(normals).zip(uvs).zip(tangents)
            {
                vertices.push(render::VertexFull {
                    position: [pos.0, pos.1, pos.2],
                    normal: [normal.0, normal.1, normal.2],
                    color: [1.0, 1.0, 1.0],
                    uv: [uv.0, uv.1],
                    tangent: [tangent.0, tangent.1, tangent.2, tangent.3],
                });
            }

            meshes.push(Mesh {
                vertex_buffer: render::VertexBufferBuilder::new(&vertices).build(ctx),
                index_buffer: render::IndexBufferBuilder::new(&indices).build(ctx),
            });

            // Normal texture
            if let Some(normal_texture) = prim.material().normal_texture() {
                if let gltf::image::Source::View { view, .. } =
                    normal_texture.texture().source().source()
                {
                    let img_buf = &buffer[view.offset()..view.offset() + view.length()];
                    material.normal = Some(
                        render::TextureBuilder::new()
                            .usage(
                                wgpu::TextureUsages::TEXTURE_BINDING
                                    | wgpu::TextureUsages::COPY_SRC
                                    | wgpu::TextureUsages::COPY_DST,
                            )
                            .build_init(ctx, img_buf),
                    );
                }
            }

            // Albedo texture
            if let Some(base_color_texture) = prim
                .material()
                .pbr_metallic_roughness()
                .base_color_texture()
            {
                if let gltf::image::Source::View { view, .. } =
                    base_color_texture.texture().source().source()
                {
                    let img_buf = &buffer[view.offset()..view.offset() + view.length()];
                    material.albedo = Some(
                        render::TextureBuilder::new()
                            .usage(
                                wgpu::TextureUsages::TEXTURE_BINDING
                                    | wgpu::TextureUsages::COPY_SRC
                                    | wgpu::TextureUsages::COPY_DST,
                            )
                            .build_init(ctx, img_buf),
                    );
                }
            }

            // Metal
            if let Some(roughness_texture) = prim
                .material()
                .pbr_metallic_roughness()
                .metallic_roughness_texture()
            {
                if let gltf::image::Source::View { view, .. } =
                    roughness_texture.texture().source().source()
                {
                    let img_buf = &buffer[view.offset()..view.offset() + view.length()];
                    material.roughness = Some(
                        render::TextureBuilder::new()
                            .usage(
                                wgpu::TextureUsages::TEXTURE_BINDING
                                    | wgpu::TextureUsages::COPY_SRC
                                    | wgpu::TextureUsages::COPY_DST,
                            )
                            .build_init(ctx, img_buf),
                    );
                }
            }
        }
    }

    (Model { meshes }, material)
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
