use crate::{texture_builder_from_image_bytes, PbrMaterial};
use gbase::{
    glam::Mat4,
    render::{self, Image, SamplerBuilder},
    tracing, wgpu,
};

pub fn parse_glb(glb_bytes: &[u8]) -> Vec<GltfPrimitive> {
    let mut meshes = Vec::new();

    let glb = gltf::Glb::from_slice(glb_bytes).expect("could not import glb from slice");
    let info = gltf::Gltf::from_slice(glb_bytes).expect("could not import info from slice");
    let buffer = glb.bin.expect("could not get glb buffer");

    let mut scenes = info.scenes();
    if scenes.len() > 1 {
        panic!("glb files with multiple scenes not supported");
    }

    let scene = scenes.next().expect("no scenes found");

    // root nodes
    let mut node_stack = Vec::new();
    for root in scene.nodes() {
        node_stack.push((root, Mat4::IDENTITY));
    }

    while let Some((node, transform)) = node_stack.pop() {
        // tracing::info!("visiting {}", node.name().unwrap_or("---"));
        if node.camera().is_some() {
            tracing::error!("camera decoding not supported");
        }

        // TODO: not used rn
        let local_transform = transform * Mat4::from_cols_array_2d(&node.transform().matrix());

        if let Some(mesh) = node.mesh() {
            // each primitive has its own material
            // so its basically out Mesh
            for primitive in mesh.primitives() {
                let topology = match primitive.mode() {
                    gltf::mesh::Mode::Points => wgpu::PrimitiveTopology::PointList,
                    gltf::mesh::Mode::Lines => wgpu::PrimitiveTopology::LineList,
                    gltf::mesh::Mode::LineStrip => wgpu::PrimitiveTopology::LineStrip,
                    gltf::mesh::Mode::Triangles => wgpu::PrimitiveTopology::TriangleList,
                    gltf::mesh::Mode::TriangleStrip => wgpu::PrimitiveTopology::TriangleStrip,
                    mode => panic!("primite mode {:?} not supported", mode),
                };
                let mut mesh = render::Mesh::new(topology);

                // parse vertex attributes
                for (sem, attr) in primitive.attributes() {
                    let view = attr.view().expect("buffer view not found");

                    let offset = view.offset();
                    let length = view.length();

                    let bytes = &buffer[offset..offset + length];

                    match sem {
                        gltf::Semantic::Positions => mesh.set_attribute(
                            render::VertexAttributeId::Position,
                            render::VertexAttributeValues::Float32x3(
                                bytemuck::cast_slice::<u8, [f32; 3]>(bytes).to_vec(),
                            ),
                        ),
                        gltf::Semantic::Normals => mesh.set_attribute(
                            render::VertexAttributeId::Normal,
                            render::VertexAttributeValues::Float32x3(
                                bytemuck::cast_slice::<u8, [f32; 3]>(bytes).to_vec(),
                            ),
                        ),
                        gltf::Semantic::Tangents => mesh.set_attribute(
                            render::VertexAttributeId::Tangent,
                            render::VertexAttributeValues::Float32x4(
                                bytemuck::cast_slice::<u8, [f32; 4]>(bytes).to_vec(),
                            ),
                        ),
                        gltf::Semantic::TexCoords(i) => mesh.set_attribute(
                            render::VertexAttributeId::Uv(i),
                            render::VertexAttributeValues::Float32x2(
                                bytemuck::cast_slice::<u8, [f32; 2]>(bytes).to_vec(),
                            ),
                        ),
                        gltf::Semantic::Colors(i) => mesh.set_attribute(
                            render::VertexAttributeId::Color(i),
                            render::VertexAttributeValues::Float32x3(
                                bytemuck::cast_slice::<u8, [f32; 3]>(bytes).to_vec(),
                            ),
                        ),
                        gltf::Semantic::Joints(_) => {
                            // TODO: gotta check u16x4 vs u32x4
                            tracing::warn!("joints not supported in gltf");
                        }
                        gltf::Semantic::Weights(_) => {
                            // f32x4
                            tracing::warn!("weigths not supported in gltf");
                        } // extras?
                    }
                }
                if !mesh.validate() {
                    tracing::error!("mesh validation failed");
                }

                // parse indices
                let indices_attr = primitive.indices().expect("could not get indices");
                let view = indices_attr.view().expect("buffer view not found");

                assert!(
                    indices_attr.dimensions() == gltf::accessor::Dimensions::Scalar,
                    "indices expected {:?} got {:?}",
                    gltf::accessor::Dimensions::Scalar,
                    indices_attr.dimensions()
                );
                assert!(
                    matches!(view.buffer().source(), gltf::buffer::Source::Bin),
                    "buffer source URI not supported"
                );
                assert!(
                    view.stride().is_none(),
                    "attribute data with stride not supported"
                );

                let offset = view.offset();
                let length = view.length();

                let indices = match indices_attr.data_type() {
                    gltf::accessor::DataType::U8 => buffer[offset..offset + length]
                        .iter()
                        .map(|&i| i as u32)
                        .collect::<Vec<_>>(),
                    gltf::accessor::DataType::U16 => {
                        bytemuck::cast_slice::<u8, u16>(&buffer[offset..offset + length])
                            .to_vec()
                            .iter()
                            .map(|&i| i as u32)
                            .collect::<Vec<_>>()
                    }
                    gltf::accessor::DataType::U32 => {
                        bytemuck::cast_slice::<u8, u32>(&buffer[offset..offset + length]).to_vec()
                    }
                    data_type => panic!("unsupported data type for indices: {:?}", data_type),
                };
                mesh.set_indices(indices);

                // material

                let material = primitive.material();
                let pbr = material.pbr_metallic_roughness();

                fn must_load_texture(
                    texture: &gltf::texture::Texture<'_>,
                    buffer: &[u8],
                ) -> Vec<u8> {
                    let image = texture.source();
                    let gltf::image::Source::View { view, mime_type } = image.source() else {
                        panic!("image source URI not supported");
                    };

                    // tracing::info!("loading image with mime type {}", mime_type);
                    assert!(
                        mime_type == "image/jpeg" || mime_type == "image/png",
                        "mime type must be image/jpeg or image/png got {}",
                        mime_type
                    );

                    let offset = view.offset();
                    let length = view.length();
                    buffer[offset..offset + length].to_vec()
                }

                // NOTE: all textures have a corresponding TEXCOORD_{i}
                let base_color_texture = pbr.base_color_texture().map(|info| {
                    assert!(
                        info.tex_coord() == 0,
                        "non 0 TEXCOORD not supported (albedo)"
                    );
                    let samp = info.texture().sampler();
                    let texture = must_load_texture(&info.texture(), &buffer);
                    Image {
                        texture: texture_builder_from_image_bytes(&texture)
                            .expect("could not load"),
                        sampler: SamplerBuilder::new()
                            .min_mag_filter(
                                samp.min_filter()
                                    // TODO: handle mipmap filters
                                    .map_or(wgpu::FilterMode::Linear, |filter| match filter {
                                        gltf::texture::MinFilter::Nearest
                                        | gltf::texture::MinFilter::NearestMipmapLinear
                                        | gltf::texture::MinFilter::NearestMipmapNearest => {
                                            wgpu::FilterMode::Nearest
                                        }
                                        gltf::texture::MinFilter::Linear
                                        | gltf::texture::MinFilter::LinearMipmapNearest
                                        | gltf::texture::MinFilter::LinearMipmapLinear => {
                                            wgpu::FilterMode::Linear
                                        }
                                    }),
                                samp.mag_filter()
                                    .map_or(wgpu::FilterMode::Linear, |filter| match filter {
                                        gltf::texture::MagFilter::Nearest => {
                                            wgpu::FilterMode::Nearest
                                        }
                                        gltf::texture::MagFilter::Linear => {
                                            wgpu::FilterMode::Linear
                                        }
                                    }),
                            )
                            .address_mode_separate(
                                match samp.wrap_s() {
                                    gltf::texture::WrappingMode::ClampToEdge => {
                                        wgpu::AddressMode::ClampToEdge
                                    }
                                    gltf::texture::WrappingMode::MirroredRepeat => {
                                        wgpu::AddressMode::MirrorRepeat
                                    }
                                    gltf::texture::WrappingMode::Repeat => {
                                        wgpu::AddressMode::Repeat
                                    }
                                },
                                match samp.wrap_t() {
                                    gltf::texture::WrappingMode::ClampToEdge => {
                                        wgpu::AddressMode::ClampToEdge
                                    }
                                    gltf::texture::WrappingMode::MirroredRepeat => {
                                        wgpu::AddressMode::MirrorRepeat
                                    }
                                    gltf::texture::WrappingMode::Repeat => {
                                        wgpu::AddressMode::Repeat
                                    }
                                },
                                wgpu::AddressMode::default(),
                            ),
                    }
                });

                let color_factor = pbr.base_color_factor(); // scaling / replacement

                let metallic_roughness_texture = pbr.metallic_roughness_texture().map(|info| {
                    assert!(
                        info.tex_coord() == 0,
                        "non 0 TEXCOORD not supported (metallic rougness)"
                    );
                    let samp = info.texture().sampler();
                    let texture = must_load_texture(&info.texture(), &buffer);
                    Image {
                        texture: texture_builder_from_image_bytes(&texture)
                            .expect("could not load"),
                        sampler: SamplerBuilder::new()
                            .min_mag_filter(
                                samp.min_filter()
                                    .map_or(wgpu::FilterMode::Linear, |filter| match filter {
                                        gltf::texture::MinFilter::Nearest
                                        | gltf::texture::MinFilter::NearestMipmapLinear
                                        | gltf::texture::MinFilter::NearestMipmapNearest => {
                                            wgpu::FilterMode::Nearest
                                        }
                                        gltf::texture::MinFilter::Linear
                                        | gltf::texture::MinFilter::LinearMipmapNearest
                                        | gltf::texture::MinFilter::LinearMipmapLinear => {
                                            wgpu::FilterMode::Linear
                                        }
                                    }),
                                samp.mag_filter()
                                    .map_or(wgpu::FilterMode::Linear, |filter| match filter {
                                        gltf::texture::MagFilter::Nearest => {
                                            wgpu::FilterMode::Nearest
                                        }
                                        gltf::texture::MagFilter::Linear => {
                                            wgpu::FilterMode::Linear
                                        }
                                    }),
                            )
                            .address_mode_separate(
                                match samp.wrap_s() {
                                    gltf::texture::WrappingMode::ClampToEdge => {
                                        wgpu::AddressMode::ClampToEdge
                                    }
                                    gltf::texture::WrappingMode::MirroredRepeat => {
                                        wgpu::AddressMode::MirrorRepeat
                                    }
                                    gltf::texture::WrappingMode::Repeat => {
                                        wgpu::AddressMode::Repeat
                                    }
                                },
                                match samp.wrap_t() {
                                    gltf::texture::WrappingMode::ClampToEdge => {
                                        wgpu::AddressMode::ClampToEdge
                                    }
                                    gltf::texture::WrappingMode::MirroredRepeat => {
                                        wgpu::AddressMode::MirrorRepeat
                                    }
                                    gltf::texture::WrappingMode::Repeat => {
                                        wgpu::AddressMode::Repeat
                                    }
                                },
                                wgpu::AddressMode::default(),
                            ),
                    }
                });
                // let metallic_roughness_texture = match metallic_roughness_texture {
                //     Some(tex) => image_cache.allocate(tex),
                //     None => self.pixel_cache(image_cache, METALLIC_ROUGHNESS_DEFAULT),
                // };
                let metallic_factor = pbr.metallic_factor(); // scaling / replacement
                let roughness_factor = pbr.roughness_factor(); // scaling / replacement

                let mut normal_scale = 1.0;
                let normal_texture = material.normal_texture().map(|info| {
                    assert!(
                        info.tex_coord() == 0,
                        "non 0 TEXCOORD not supported (normal)"
                    );
                    let samp = info.texture().sampler();
                    let texture = must_load_texture(&info.texture(), &buffer);
                    normal_scale = info.scale();
                    Image {
                        texture: texture_builder_from_image_bytes(&texture)
                            .expect("could not load"),
                        sampler: SamplerBuilder::new()
                            .min_mag_filter(
                                samp.min_filter()
                                    .map_or(wgpu::FilterMode::Linear, |filter| match filter {
                                        gltf::texture::MinFilter::Nearest
                                        | gltf::texture::MinFilter::NearestMipmapLinear
                                        | gltf::texture::MinFilter::NearestMipmapNearest => {
                                            wgpu::FilterMode::Nearest
                                        }
                                        gltf::texture::MinFilter::Linear
                                        | gltf::texture::MinFilter::LinearMipmapNearest
                                        | gltf::texture::MinFilter::LinearMipmapLinear => {
                                            wgpu::FilterMode::Linear
                                        }
                                    }),
                                samp.mag_filter()
                                    .map_or(wgpu::FilterMode::Linear, |filter| match filter {
                                        gltf::texture::MagFilter::Nearest => {
                                            wgpu::FilterMode::Nearest
                                        }
                                        gltf::texture::MagFilter::Linear => {
                                            wgpu::FilterMode::Linear
                                        }
                                    }),
                            )
                            .address_mode_separate(
                                match samp.wrap_s() {
                                    gltf::texture::WrappingMode::ClampToEdge => {
                                        wgpu::AddressMode::ClampToEdge
                                    }
                                    gltf::texture::WrappingMode::MirroredRepeat => {
                                        wgpu::AddressMode::MirrorRepeat
                                    }
                                    gltf::texture::WrappingMode::Repeat => {
                                        wgpu::AddressMode::Repeat
                                    }
                                },
                                match samp.wrap_t() {
                                    gltf::texture::WrappingMode::ClampToEdge => {
                                        wgpu::AddressMode::ClampToEdge
                                    }
                                    gltf::texture::WrappingMode::MirroredRepeat => {
                                        wgpu::AddressMode::MirrorRepeat
                                    }
                                    gltf::texture::WrappingMode::Repeat => {
                                        wgpu::AddressMode::Repeat
                                    }
                                },
                                wgpu::AddressMode::default(),
                            ),
                    }
                });
                // let normal_texture = match normal_texture {
                //     Some(tex) => image_cache.allocate(tex),
                //     None => self.pixel_cache(image_cache, NORMAL_DEFAULT),
                // };

                let mut occlusion_strength = 1.0;
                let occlusion_texture = material.occlusion_texture().map(|info| {
                    assert!(
                        info.tex_coord() == 0,
                        "non 0 TEXCOORD not supported (normal)"
                    );
                    let samp = info.texture().sampler();
                    let texture = must_load_texture(&info.texture(), &buffer);
                    occlusion_strength = info.strength();
                    Image {
                        texture: texture_builder_from_image_bytes(&texture)
                            .expect("could not load"),
                        sampler: SamplerBuilder::new()
                            .min_mag_filter(
                                samp.min_filter()
                                    .map_or(wgpu::FilterMode::Linear, |filter| match filter {
                                        gltf::texture::MinFilter::Nearest
                                        | gltf::texture::MinFilter::NearestMipmapLinear
                                        | gltf::texture::MinFilter::NearestMipmapNearest => {
                                            wgpu::FilterMode::Nearest
                                        }
                                        gltf::texture::MinFilter::Linear
                                        | gltf::texture::MinFilter::LinearMipmapNearest
                                        | gltf::texture::MinFilter::LinearMipmapLinear => {
                                            wgpu::FilterMode::Linear
                                        }
                                    }),
                                samp.mag_filter()
                                    .map_or(wgpu::FilterMode::Linear, |filter| match filter {
                                        gltf::texture::MagFilter::Nearest => {
                                            wgpu::FilterMode::Nearest
                                        }
                                        gltf::texture::MagFilter::Linear => {
                                            wgpu::FilterMode::Linear
                                        }
                                    }),
                            )
                            .address_mode_separate(
                                match samp.wrap_s() {
                                    gltf::texture::WrappingMode::ClampToEdge => {
                                        wgpu::AddressMode::ClampToEdge
                                    }
                                    gltf::texture::WrappingMode::MirroredRepeat => {
                                        wgpu::AddressMode::MirrorRepeat
                                    }
                                    gltf::texture::WrappingMode::Repeat => {
                                        wgpu::AddressMode::Repeat
                                    }
                                },
                                match samp.wrap_t() {
                                    gltf::texture::WrappingMode::ClampToEdge => {
                                        wgpu::AddressMode::ClampToEdge
                                    }
                                    gltf::texture::WrappingMode::MirroredRepeat => {
                                        wgpu::AddressMode::MirrorRepeat
                                    }
                                    gltf::texture::WrappingMode::Repeat => {
                                        wgpu::AddressMode::Repeat
                                    }
                                },
                                wgpu::AddressMode::default(),
                            ),
                    }
                });

                let emissive_factor = material.emissive_factor();
                let emissive_texture = material.emissive_texture().map(|info| {
                    assert!(
                        info.tex_coord() == 0,
                        "non 0 TEXCOORD not supported (normal)"
                    );
                    let samp = info.texture().sampler();
                    let texture = must_load_texture(&info.texture(), &buffer);
                    Image {
                        texture: texture_builder_from_image_bytes(&texture)
                            .expect("could not load"),
                        sampler: SamplerBuilder::new()
                            .min_mag_filter(
                                samp.min_filter()
                                    .map_or(wgpu::FilterMode::Linear, |filter| match filter {
                                        gltf::texture::MinFilter::Nearest
                                        | gltf::texture::MinFilter::NearestMipmapLinear
                                        | gltf::texture::MinFilter::NearestMipmapNearest => {
                                            wgpu::FilterMode::Nearest
                                        }
                                        gltf::texture::MinFilter::Linear
                                        | gltf::texture::MinFilter::LinearMipmapNearest
                                        | gltf::texture::MinFilter::LinearMipmapLinear => {
                                            wgpu::FilterMode::Linear
                                        }
                                    }),
                                samp.mag_filter()
                                    .map_or(wgpu::FilterMode::Linear, |filter| match filter {
                                        gltf::texture::MagFilter::Nearest => {
                                            wgpu::FilterMode::Nearest
                                        }
                                        gltf::texture::MagFilter::Linear => {
                                            wgpu::FilterMode::Linear
                                        }
                                    }),
                            )
                            .address_mode_separate(
                                match samp.wrap_s() {
                                    gltf::texture::WrappingMode::ClampToEdge => {
                                        wgpu::AddressMode::ClampToEdge
                                    }
                                    gltf::texture::WrappingMode::MirroredRepeat => {
                                        wgpu::AddressMode::MirrorRepeat
                                    }
                                    gltf::texture::WrappingMode::Repeat => {
                                        wgpu::AddressMode::Repeat
                                    }
                                },
                                match samp.wrap_t() {
                                    gltf::texture::WrappingMode::ClampToEdge => {
                                        wgpu::AddressMode::ClampToEdge
                                    }
                                    gltf::texture::WrappingMode::MirroredRepeat => {
                                        wgpu::AddressMode::MirrorRepeat
                                    }
                                    gltf::texture::WrappingMode::Repeat => {
                                        wgpu::AddressMode::Repeat
                                    }
                                },
                                wgpu::AddressMode::default(),
                            ),
                    }
                });

                // let occlusion_texture = match occlusion_texture {
                //     Some(tex) => image_cache.allocate(tex),
                //     None => self.pixel_cache(image_cache, OCCLUSION_DEFAULT),
                // };

                let material = PbrMaterial {
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
                };

                // tracing::info!("{:#?}", new_transform.to_scale_rotation_translation());

                meshes.push(GltfPrimitive {
                    mesh,
                    material,
                    transform: local_transform,
                });
            }
        }

        // recursively visit children
        for child in node.children() {
            node_stack.push((child, local_transform));
        }
    }

    meshes
}

//
// Gltf types
//

#[derive(Debug, Clone)]
pub struct GltfPrimitive {
    pub mesh: render::Mesh,
    pub material: PbrMaterial,
    pub transform: Mat4,
}
