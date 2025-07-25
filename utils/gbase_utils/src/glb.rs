use crate::{texture_builder_from_image_bytes, Transform3D};
use gbase::{
    asset::{Asset, AssetCache, AssetHandle, LoadContext},
    glam::{Quat, Vec3},
    render::{self, Image, Mesh, SamplerBuilder},
    tracing, wgpu,
};
use std::collections::{BTreeMap, HashMap};

pub struct GltfLoadCache {
    nodes: HashMap<usize, AssetHandle<GltfNode>>,
    named_nodes: HashMap<Box<str>, AssetHandle<GltfNode>>,

    meshes: HashMap<usize, AssetHandle<GltfMesh>>,
    named_meshes: HashMap<Box<str>, AssetHandle<GltfMesh>>,

    materials: HashMap<usize, AssetHandle<Material>>,
}

impl GltfLoadCache {
    fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            named_nodes: HashMap::new(),
            meshes: HashMap::new(),
            named_meshes: HashMap::new(),
            materials: HashMap::new(),
        }
    }
}

// TODO: have local cache for duplicate materials/primitives

pub fn parse_gltf_primitives(load_ctx: &LoadContext, bytes: &[u8]) -> Vec<GltfPrimitive> {
    let mut gltf_cache = GltfLoadCache::new();
    let glb = gltf::Glb::from_slice(bytes).expect("could not import glb from slice");
    let info = gltf::Gltf::from_slice(bytes).expect("could not import info from slice");
    let buffer = glb.bin.expect("could not get glb buffer");

    let mut primitives = Vec::new();
    for mesh in info.meshes() {
        let name = mesh
            .name()
            .map(|name| name.to_string())
            .unwrap_or(format!("Mesh{}", mesh.index()));

        for primitive in mesh.primitives() {
            let primitive =
                parse_gltf_primitive(load_ctx, &buffer, &mut gltf_cache, primitive, &name);
            primitives.push(primitive);
        }
    }

    primitives
}

pub fn parse_gltf_file(load_ctx: &LoadContext, bytes: &[u8]) -> Gltf {
    let mut gltf_cache = GltfLoadCache::new();
    let glb = gltf::Glb::from_slice(bytes).expect("could not import glb from slice");
    let info = gltf::Gltf::from_slice(bytes).expect("could not import info from slice");
    let buffer = glb.bin.expect("could not get glb buffer");

    // let mut nodes = Vec::new();
    for node in info.nodes() {
        parse_gltf_node(load_ctx, &buffer, &mut gltf_cache, node);
    }

    let GltfLoadCache {
        nodes,
        named_nodes,
        meshes,
        named_meshes,
        ..
    } = gltf_cache;

    Gltf {
        nodes: nodes.into_values().collect(),
        named_nodes,
        meshes: meshes.into_values().collect(),
        named_meshes,
    }
}

fn parse_gltf_node(
    load_ctx: &LoadContext,
    buffer: &[u8],
    gltf_cache: &mut GltfLoadCache,
    node: gltf::Node<'_>,
) -> AssetHandle<GltfNode> {
    if let Some(node_handle) = gltf_cache.nodes.get(&node.index()) {
        // tracing::error!("REUSE NODE {}", node.index());
        return *node_handle;
    }

    let name = node
        .name()
        .map(|name| name.to_string())
        .unwrap_or(format!("Node{}", node.index()));

    let mesh = node
        .mesh()
        .map(|mesh| parse_gltf_mesh(load_ctx, buffer, gltf_cache, mesh));

    let (translation, rotation, scale) = node.transform().decomposed();
    let transform = Transform3D::new(
        Vec3::from_array(translation),
        Quat::from_array(rotation),
        Vec3::from_array(scale),
    );

    let mut children = Vec::new();
    for child in node.children() {
        let child_node_handle = parse_gltf_node(load_ctx, buffer, gltf_cache, child);
        children.push(child_node_handle);
    }

    let node_handle = load_ctx.insert(GltfNode {
        name,
        mesh,
        transform,
        children,
    });

    gltf_cache.nodes.insert(node.index(), node_handle);
    if let Some(name) = node.name() {
        gltf_cache.named_nodes.insert(Box::from(name), node_handle);
    }
    node_handle
}

fn parse_gltf_mesh(
    load_ctx: &LoadContext,
    buffer: &[u8],
    gltf_cache: &mut GltfLoadCache,
    mesh: gltf::Mesh<'_>,
) -> AssetHandle<GltfMesh> {
    if let Some(mesh_handle) = gltf_cache.meshes.get(&mesh.index()) {
        // tracing::error!("REUSE MESH {}", mesh.index());
        return *mesh_handle;
    }

    let name = mesh
        .name()
        .map(|name| name.to_string())
        .unwrap_or(format!("Mesh{}", mesh.index()));

    let mut primitives = Vec::new();
    for primitive in mesh.primitives() {
        let primitive = parse_gltf_primitive(load_ctx, buffer, gltf_cache, primitive, &name);
        primitives.push(primitive);
    }

    let mesh_handle = load_ctx.insert(GltfMesh { name, primitives });
    if let Some(name) = mesh.name() {
        gltf_cache.named_meshes.insert(Box::from(name), mesh_handle);
    }
    gltf_cache.meshes.insert(mesh.index(), mesh_handle);
    mesh_handle
}

fn parse_gltf_primitive(
    load_ctx: &LoadContext,
    buffer: &[u8],
    gltf_cache: &mut GltfLoadCache,
    primitive: gltf::Primitive<'_>,
    mesh_name: &str,
) -> GltfPrimitive {
    let primitive_topology = match primitive.mode() {
        gltf::mesh::Mode::Points => wgpu::PrimitiveTopology::PointList,
        gltf::mesh::Mode::Lines => wgpu::PrimitiveTopology::LineList,
        gltf::mesh::Mode::LineStrip => wgpu::PrimitiveTopology::LineStrip,
        gltf::mesh::Mode::Triangles => wgpu::PrimitiveTopology::TriangleList,
        gltf::mesh::Mode::TriangleStrip => wgpu::PrimitiveTopology::TriangleStrip,
        mode => panic!("primite mode {:?} not supported", mode),
    };

    let mut attributes = BTreeMap::new();
    for (sem, attr) in primitive.attributes() {
        let view = attr.view().expect("buffer view not found");

        let offset = view.offset();
        let length = view.length();

        let bytes = &buffer[offset..offset + length];

        // TODO: can access with attr.get()

        match sem {
            gltf::Semantic::Positions => {
                attributes.insert(
                    render::VertexAttributeId::Position,
                    render::VertexAttributeValues::Float32x3(
                        bytemuck::cast_slice::<u8, [f32; 3]>(bytes).to_vec(),
                    ),
                );
            }
            gltf::Semantic::Normals => {
                attributes.insert(
                    render::VertexAttributeId::Normal,
                    render::VertexAttributeValues::Float32x3(
                        bytemuck::cast_slice::<u8, [f32; 3]>(bytes).to_vec(),
                    ),
                );
            }
            gltf::Semantic::Tangents => {
                attributes.insert(
                    render::VertexAttributeId::Tangent,
                    render::VertexAttributeValues::Float32x4(
                        bytemuck::cast_slice::<u8, [f32; 4]>(bytes).to_vec(),
                    ),
                );
            }
            gltf::Semantic::TexCoords(i) => {
                attributes.insert(
                    render::VertexAttributeId::Uv(i),
                    render::VertexAttributeValues::Float32x2(
                        bytemuck::cast_slice::<u8, [f32; 2]>(bytes).to_vec(),
                    ),
                );
            }
            gltf::Semantic::Colors(i) => {
                attributes.insert(
                    render::VertexAttributeId::Color(i),
                    render::VertexAttributeValues::Float32x3(
                        bytemuck::cast_slice::<u8, [f32; 3]>(bytes).to_vec(),
                    ),
                );
            }
            gltf::Semantic::Joints(_) => {
                // TODO: gotta check u16x4 vs u32x4
                tracing::warn!("joints not supported in gltf");
            }
            gltf::Semantic::Weights(_) => {
                // f32x4
                tracing::warn!("weigths not supported in gltf");
            }
        }
    }

    //
    // Indices
    //

    let indices = primitive.indices().map(|indices| {
        let view = indices.view().expect("buffer view not found");

        assert!(
            indices.dimensions() == gltf::accessor::Dimensions::Scalar,
            "indices expected {:?} got {:?}",
            gltf::accessor::Dimensions::Scalar,
            indices.dimensions()
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

        let indices = match indices.data_type() {
            gltf::accessor::DataType::U8 => buffer[offset..offset + length]
                .iter()
                .map(|&i| i as u32)
                .collect::<Vec<_>>(),
            gltf::accessor::DataType::U16 => {
                bytemuck::cast_slice::<u8, u16>(&buffer[offset..offset + length])
                    .iter()
                    .map(|&i| i as u32)
                    .collect::<Vec<_>>()
            }
            gltf::accessor::DataType::U32 => {
                bytemuck::cast_slice::<u8, u32>(&buffer[offset..offset + length]).to_vec()
            }
            data_type => panic!("unsupported data type for indices: {:?}", data_type),
        };

        indices
    });

    let mesh = render::Mesh {
        primitive_topology,
        attributes,
        indices,
    };

    // let name = format!("{}_Primitive{}", mesh_name, primitive.index());
    let name = mesh_name.to_string();

    let material = parse_gltf_material(load_ctx, buffer, gltf_cache, primitive.material());

    GltfPrimitive {
        name,
        material,
        mesh: load_ctx.insert(mesh),
    }
}

pub fn parse_gltf_material(
    load_ctx: &LoadContext,
    buffer: &[u8],
    gltf_cache: &mut GltfLoadCache,
    material: gltf::Material<'_>,
) -> AssetHandle<Material> {
    // TODO: have default material on None?
    if let Some(index) = material.index() {
        // tracing::error!("REUSE MATERIAL {index}");
        if let Some(material) = gltf_cache.materials.get(&index) {
            return *material;
        }
    }

    fn load_image(
        buffer: &[u8],
        texture: &gltf::texture::Texture<'_>,
        format: wgpu::TextureFormat,
    ) -> Image {
        let image = texture.source();
        let gltf::image::Source::View { view, mime_type } = image.source() else {
            panic!("image source URI not supported");
        };

        assert!(
            mime_type == "image/jpeg" || mime_type == "image/png",
            "mime type must be image/jpeg or image/png got {}",
            mime_type
        );

        let offset = view.offset();
        let length = view.length();
        let texture_buffer = &buffer[offset..offset + length];
        let sampler = texture.sampler();

        let texture_builder = texture_builder_from_image_bytes(texture_buffer)
            .expect("could not load")
            .with_format(format);

        Image {
            texture: texture_builder,
            sampler: SamplerBuilder::new()
                .min_mag_filter(
                    sampler
                        .min_filter()
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
                    sampler
                        .mag_filter()
                        .map_or(wgpu::FilterMode::Linear, |filter| match filter {
                            gltf::texture::MagFilter::Nearest => wgpu::FilterMode::Nearest,
                            gltf::texture::MagFilter::Linear => wgpu::FilterMode::Linear,
                        }),
                )
                .address_mode_separate(
                    match sampler.wrap_s() {
                        gltf::texture::WrappingMode::ClampToEdge => wgpu::AddressMode::ClampToEdge,
                        gltf::texture::WrappingMode::MirroredRepeat => {
                            wgpu::AddressMode::MirrorRepeat
                        }
                        gltf::texture::WrappingMode::Repeat => wgpu::AddressMode::Repeat,
                    },
                    match sampler.wrap_t() {
                        gltf::texture::WrappingMode::ClampToEdge => wgpu::AddressMode::ClampToEdge,
                        gltf::texture::WrappingMode::MirroredRepeat => {
                            wgpu::AddressMode::MirrorRepeat
                        }
                        gltf::texture::WrappingMode::Repeat => wgpu::AddressMode::Repeat,
                    },
                    wgpu::AddressMode::default(),
                ),
        }
    }

    fn single_pixel_image(color: [u8; 4]) -> Image {
        Image {
            texture: render::TextureBuilder::new(render::TextureSource::Data(1, 1, color.to_vec()))
                .with_format(wgpu::TextureFormat::Rgba8Unorm),
            sampler: render::SamplerBuilder::new()
                .min_mag_filter(wgpu::FilterMode::Nearest, wgpu::FilterMode::Nearest),
        }
    }

    const BASE_COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
    const NORMAL_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
    const METALLIC_ROUGHNESS_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
    const OCCLUSION_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
    const EMMISIVE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

    const BASE_COLOR_DEFAULT: [u8; 4] = [255, 255, 255, 255];
    const NORMAL_DEFAULT: [u8; 4] = [128, 128, 255, 0];
    const METALLIC_ROUGHNESS_DEFAULT: [u8; 4] = [0, 255, 0, 0];
    const OCCLUSION_DEFAULT: [u8; 4] = [255, 0, 0, 0];
    const EMISSIVE_DEFAULT: [u8; 4] = [0, 0, 0, 0];

    let pbr = material.pbr_metallic_roughness();

    let color_factor = pbr.base_color_factor();
    let base_color_texture = match pbr.base_color_texture() {
        Some(info) => {
            assert!(
                info.tex_coord() == 0,
                "non 0 TEXCOORD not supported (albedo)"
            );
            let image = load_image(buffer, &info.texture(), BASE_COLOR_FORMAT);
            load_ctx.insert(image)
        }
        None => {
            let image = single_pixel_image(BASE_COLOR_DEFAULT);
            load_ctx.insert(image)
        }
    };

    let roughness_factor = pbr.roughness_factor();
    let metallic_factor = pbr.metallic_factor();
    let metallic_roughness_texture = match pbr.metallic_roughness_texture() {
        Some(info) => {
            assert!(
                info.tex_coord() == 0,
                "non 0 TEXCOORD not supported (metallic roughness)"
            );
            let image = load_image(buffer, &info.texture(), METALLIC_ROUGHNESS_FORMAT);
            load_ctx.insert(image)
        }
        None => {
            let image = single_pixel_image(METALLIC_ROUGHNESS_DEFAULT);
            load_ctx.insert(image)
        }
    };

    let (occlusion_texture, occlusion_strength) = match material.occlusion_texture() {
        Some(info) => {
            assert!(
                info.tex_coord() == 0,
                "non 0 TEXCOORD not supported (occlusion)"
            );
            let image = load_image(buffer, &info.texture(), OCCLUSION_FORMAT);
            (load_ctx.insert(image), info.strength())
        }
        None => {
            let image = single_pixel_image(OCCLUSION_DEFAULT);
            (load_ctx.insert(image), 1.0)
        }
    };

    let (normal_texture, normal_scale) = match material.normal_texture() {
        Some(info) => {
            assert!(
                info.tex_coord() == 0,
                "non 0 TEXCOORD not supported (normal)"
            );
            let image = load_image(buffer, &info.texture(), NORMAL_FORMAT);
            (load_ctx.insert(image), info.scale())
        }
        None => {
            let image = single_pixel_image(NORMAL_DEFAULT);
            (load_ctx.insert(image), 1.0)
        }
    };

    let emissive_factor = material.emissive_factor();
    let emissive_texture = match material.emissive_texture() {
        Some(info) => {
            assert!(
                info.tex_coord() == 0,
                "non 0 TEXCOORD not supported (emissive)"
            );
            let image = load_image(buffer, &info.texture(), EMMISIVE_FORMAT);
            load_ctx.insert(image)
        }
        None => {
            let image = single_pixel_image(EMISSIVE_DEFAULT);
            load_ctx.insert(image)
        }
    };

    let material_handle = load_ctx.insert(Material {
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
    });

    if let Some(index) = material.index() {
        gltf_cache.materials.insert(index, material_handle);
    }

    material_handle
}

impl Asset for Gltf {}

impl Asset for GltfMesh {}
impl Asset for Material {}
impl Asset for GltfNode {}

#[derive(Debug, Clone)]
pub struct Gltf {
    pub nodes: Vec<AssetHandle<GltfNode>>,
    pub named_nodes: HashMap<Box<str>, AssetHandle<GltfNode>>,
    pub meshes: Vec<AssetHandle<GltfMesh>>,
    pub named_meshes: HashMap<Box<str>, AssetHandle<GltfMesh>>,
}

#[derive(Debug, Clone)]
pub struct GltfNode {
    pub name: String,
    pub mesh: Option<AssetHandle<GltfMesh>>,
    pub transform: Transform3D,
    pub children: Vec<AssetHandle<GltfNode>>,
}

#[derive(Debug, Clone)]
pub struct GltfMesh {
    pub name: String,
    pub primitives: Vec<GltfPrimitive>,
}

#[derive(Debug, Clone)]
pub struct GltfPrimitive {
    pub name: String,
    pub mesh: AssetHandle<Mesh>,
    pub material: AssetHandle<Material>,
}

// TODO: make textures optional
#[derive(Debug, Clone)]
pub struct Material {
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

impl Material {
    pub fn default(cache: &mut AssetCache) -> Self {
        const BASE_COLOR_DEFAULT: [u8; 4] = [255, 255, 255, 255];
        const NORMAL_DEFAULT: [u8; 4] = [128, 128, 255, 0];
        const METALLIC_ROUGHNESS_DEFAULT: [u8; 4] = [0, 255, 0, 0];
        const OCCLUSION_DEFAULT: [u8; 4] = [255, 0, 0, 0];
        const EMISSIVE_DEFAULT: [u8; 4] = [0, 0, 0, 0];
        let base_color_texture = cache.insert(Image::new_pixel_texture(BASE_COLOR_DEFAULT));
        let metallic_roughness_texture =
            cache.insert(Image::new_pixel_texture(METALLIC_ROUGHNESS_DEFAULT));
        let occlusion_texture = cache.insert(Image::new_pixel_texture(OCCLUSION_DEFAULT));
        let normal_texture = cache.insert(Image::new_pixel_texture(NORMAL_DEFAULT));
        let emissive_texture = cache.insert(Image::new_pixel_texture(EMISSIVE_DEFAULT));
        Self {
            color_factor: [1.0, 1.0, 1.0, 1.0],
            base_color_texture,
            roughness_factor: 1.0,
            metallic_factor: 0.0,
            metallic_roughness_texture,
            occlusion_strength: 1.0,
            occlusion_texture,
            normal_scale: 1.0,
            normal_texture,
            emissive_factor: [0.0, 0.0, 0.0],
            emissive_texture,
        }
    }

    pub fn set_color_factor(&mut self, value: [f32; 4]) {
        self.color_factor = value;
    }
    pub fn with_color_factor(mut self, value: [f32; 4]) -> Self {
        self.set_color_factor(value);
        self
    }
}
