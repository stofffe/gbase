use crate::Transform3D;
use async_recursion::async_recursion;
use gbase::{
    asset::{AssetCache, AssetHandle, LoadContext, NamedInserter},
    glam::{Quat, Vec3},
    render::{self, Mesh, SamplerBuilder, TextureBuilder, VertexAttributeId},
    tracing,
    wgpu::{self},
};
use image::RgbaImage;
use std::collections::{BTreeMap, BTreeSet, HashMap};

pub struct GltfLoadCache {
    nodes: HashMap<usize, GltfNode>,
    named_nodes: HashMap<Box<str>, GltfNode>,

    meshes: HashMap<usize, GltfMesh>,
    named_meshes: HashMap<Box<str>, GltfMesh>,

    materials: HashMap<usize, AssetHandle<Material>>,

    images: HashMap<usize, TextureRef>,
    single_pixel_images: HashMap<[u8; 4], TextureRef>,
}

impl GltfLoadCache {
    fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            named_nodes: HashMap::new(),
            meshes: HashMap::new(),
            named_meshes: HashMap::new(),
            materials: HashMap::new(),
            images: HashMap::new(),
            single_pixel_images: HashMap::new(),
        }
    }
}

pub async fn parse_gltf_primitives(
    load_ctx: &mut LoadContext,
    bytes: &[u8],
    required_attributes: Option<&BTreeSet<VertexAttributeId>>,
) -> Vec<GltfPrimitive> {
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
            let primitive = parse_gltf_primitive(
                load_ctx,
                &buffer,
                &mut gltf_cache,
                primitive,
                &name,
                required_attributes,
            )
            .await;
            primitives.push(primitive);
        }
    }

    primitives
}

pub async fn parse_gltf_file(
    load_ctx: &mut LoadContext,
    bytes: &[u8],
    required_attributes: Option<&BTreeSet<VertexAttributeId>>,
) -> Gltf {
    let mut gltf_cache = GltfLoadCache::new();
    let glb = gltf::Glb::from_slice(bytes).expect("could not import glb from slice");
    let info = gltf::Gltf::from_slice(bytes).expect("could not import info from slice");
    let buffer = glb.bin.expect("could not get glb buffer");

    // let mut nodes = Vec::new();
    for node in info.nodes() {
        parse_gltf_node(
            load_ctx,
            &buffer,
            &mut gltf_cache,
            required_attributes,
            node,
        )
        .await;
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

// TODO: would be nicer to just implement loop based

#[cfg_attr(not(target_arch = "wasm32"), async_recursion)]
#[cfg_attr(target_arch = "wasm32", async_recursion(?Send))]
async fn parse_gltf_node(
    load_ctx: &mut LoadContext,
    buffer: &[u8],
    gltf_cache: &mut GltfLoadCache,
    required_attributes: Option<&BTreeSet<VertexAttributeId>>,
    node: gltf::Node<'async_recursion>,
) -> GltfNode {
    if let Some(node_handle) = gltf_cache.nodes.get(&node.index()) {
        // tracing::error!("REUSE NODE {}", node.index());
        return node_handle.clone();
    }

    let name = node
        .name()
        .map(|name| name.to_string())
        .unwrap_or(format!("Node{}", node.index()));

    let mesh = match node.mesh() {
        Some(mesh) => {
            Some(parse_gltf_mesh(load_ctx, buffer, gltf_cache, required_attributes, mesh).await)
        }
        None => None,
    };

    let (translation, rotation, scale) = node.transform().decomposed();
    let transform = Transform3D::new(
        Vec3::from_array(translation),
        Quat::from_array(rotation),
        Vec3::from_array(scale),
    );

    let mut children = Vec::new();
    for child in node.children() {
        let child_node_handle =
            parse_gltf_node(load_ctx, buffer, gltf_cache, required_attributes, child).await;
        children.push(child_node_handle);
    }

    let gltf_node = GltfNode {
        name,
        mesh,
        transform,
        children,
    };

    gltf_cache.nodes.insert(node.index(), gltf_node.clone());

    if let Some(name) = node.name() {
        gltf_cache
            .named_nodes
            .insert(Box::from(name), gltf_node.clone());
    }

    gltf_node
}

async fn parse_gltf_mesh(
    load_ctx: &mut LoadContext,
    buffer: &[u8],
    gltf_cache: &mut GltfLoadCache,
    required_attributes: Option<&BTreeSet<VertexAttributeId>>,
    mesh: gltf::Mesh<'_>,
) -> GltfMesh {
    if let Some(mesh_handle) = gltf_cache.meshes.get(&mesh.index()) {
        tracing::error!("REUSE MESH {}", mesh.index());
        return mesh_handle.clone();
    }

    let name = mesh
        .name()
        .map(|name| name.to_string())
        .unwrap_or(format!("Mesh{}", mesh.index()));

    let mut primitives = Vec::new();
    for primitive in mesh.primitives() {
        let primitive = parse_gltf_primitive(
            load_ctx,
            buffer,
            gltf_cache,
            primitive,
            &name,
            required_attributes,
        )
        .await; // TODO: maybe add this option
        primitives.push(primitive);
    }

    let gltf_mesh = GltfMesh { name, primitives };

    if let Some(name) = mesh.name() {
        gltf_cache
            .named_meshes
            .insert(Box::from(name), gltf_mesh.clone());
    }

    gltf_cache.meshes.insert(mesh.index(), gltf_mesh.clone());

    gltf_mesh
}

async fn parse_gltf_primitive(
    load_ctx: &mut LoadContext,
    buffer: &[u8],
    gltf_cache: &mut GltfLoadCache,
    primitive: gltf::Primitive<'_>,
    mesh_name: &str,
    required_attributes: Option<&BTreeSet<VertexAttributeId>>,
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

    let mut mesh = render::Mesh {
        primitive_topology,
        attributes,
        indices,
    };

    if let Some(attr) = required_attributes {
        mesh.extract_attributes(attr.clone()); // TODO: clone here?
    }

    // let name = format!("{}_Primitive{}", mesh_name, primitive.index());
    // TODO: this should not be needed
    let name = mesh_name.to_string();

    let material = parse_gltf_material(load_ctx, buffer, gltf_cache, primitive.material()).await;

    let mesh = load_ctx
        .insert_asset_scoped::<Mesh, NamedInserter>(name.clone(), mesh)
        .await;

    GltfPrimitive {
        name,
        material,
        mesh,
    }
}

pub async fn parse_gltf_material(
    load_ctx: &mut LoadContext,
    buffer: &[u8],
    gltf_cache: &mut GltfLoadCache,
    material: gltf::Material<'_>,
) -> AssetHandle<Material> {
    // TODO: have default material on None?
    if let Some(index) = material.index() {
        if let Some(material) = gltf_cache.materials.get(&index) {
            return material.clone();
        }
    }

    async fn load_texture(
        load_ctx: &mut LoadContext,
        gltf_cache: &mut GltfLoadCache,
        buffer: &[u8],
        texture: &gltf::texture::Texture<'_>,
        format: wgpu::TextureFormat, // TODO: why is this unused
    ) -> TextureRef {
        if let Some(image) = gltf_cache.images.get(&texture.index()) {
            tracing::info!("Loaded {:?} from gltf image cache", texture.index());
            return image.clone();
        }
        // TODO: look in cache using index

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

        // TODO: use insert instead

        let name = texture
            .name()
            .map(String::from)
            .unwrap_or_else(|| format!("glb texture {}", texture.index()));
        let image_buffer = image::load_from_memory(texture_buffer)
            .expect("could not load image")
            .to_rgba8();
        let image_handle = load_ctx
            .insert_asset_scoped::<RgbaImage, NamedInserter>(name, image_buffer)
            .await;

        // TODO: can you get the format from the gltf file?
        let texture_config =
            TextureBuilder::new().with_format(gbase::wgpu::TextureFormat::Rgba8Unorm);
        let sampler_config = SamplerBuilder::new()
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
                        | gltf::texture::MinFilter::LinearMipmapLinear => wgpu::FilterMode::Linear,
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
                    gltf::texture::WrappingMode::MirroredRepeat => wgpu::AddressMode::MirrorRepeat,
                    gltf::texture::WrappingMode::Repeat => wgpu::AddressMode::Repeat,
                },
                match sampler.wrap_t() {
                    gltf::texture::WrappingMode::ClampToEdge => wgpu::AddressMode::ClampToEdge,
                    gltf::texture::WrappingMode::MirroredRepeat => wgpu::AddressMode::MirrorRepeat,
                    gltf::texture::WrappingMode::Repeat => wgpu::AddressMode::Repeat,
                },
                wgpu::AddressMode::default(),
            );

        let texture_ref = TextureRef {
            image_handle,
            sampler_config,
            texture_config,
        };

        tracing::info!("Insert {:?} into gltf image cache", texture.index());
        gltf_cache
            .images
            .insert(texture.index(), texture_ref.clone());

        texture_ref
    }

    // TODO: use cache here aswell
    async fn create_single_pixel_texture(
        load_ctx: &mut LoadContext,
        gltf_cache: &mut GltfLoadCache,
        color: [u8; 4],
    ) -> TextureRef {
        if let Some(image) = gltf_cache.single_pixel_images.get(&color) {
            tracing::info!("Loaded {:?} from gltf single pixel cache", color);
            return image.clone();
        }

        let name = format!("single pixel rgb {:?}", color);
        let pixel_image = RgbaImage::from_pixel(1, 1, image::Rgba(color));
        let image_handle = load_ctx
            .insert_asset_scoped::<RgbaImage, NamedInserter>(name, pixel_image)
            .await;

        let sampler_config = render::SamplerBuilder::new()
            .min_mag_filter(wgpu::FilterMode::Nearest, wgpu::FilterMode::Nearest);
        let texture_config =
            render::TextureBuilder::new().with_format(wgpu::TextureFormat::Rgba8Unorm);

        let texture_ref = TextureRef {
            image_handle,
            sampler_config,
            texture_config,
        };

        gltf_cache
            .single_pixel_images
            .insert(color, texture_ref.clone());

        texture_ref
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
            load_texture(
                load_ctx,
                gltf_cache,
                buffer,
                &info.texture(),
                BASE_COLOR_FORMAT,
            )
            .await
        }
        None => create_single_pixel_texture(load_ctx, gltf_cache, BASE_COLOR_DEFAULT).await,
    };

    let roughness_factor = pbr.roughness_factor();
    let metallic_factor = pbr.metallic_factor();
    let metallic_roughness_texture = match pbr.metallic_roughness_texture() {
        Some(info) => {
            assert!(
                info.tex_coord() == 0,
                "non 0 TEXCOORD not supported (metallic roughness)"
            );
            load_texture(
                load_ctx,
                gltf_cache,
                buffer,
                &info.texture(),
                METALLIC_ROUGHNESS_FORMAT,
            )
            .await
        }
        None => create_single_pixel_texture(load_ctx, gltf_cache, METALLIC_ROUGHNESS_DEFAULT).await,
    };

    let (occlusion_texture, occlusion_strength) = match material.occlusion_texture() {
        Some(info) => {
            assert!(
                info.tex_coord() == 0,
                "non 0 TEXCOORD not supported (occlusion)"
            );
            let image = load_texture(
                load_ctx,
                gltf_cache,
                buffer,
                &info.texture(),
                OCCLUSION_FORMAT,
            )
            .await;
            (image, info.strength())
        }
        None => {
            let image = create_single_pixel_texture(load_ctx, gltf_cache, OCCLUSION_DEFAULT).await;
            (image, 1.0)
        }
    };

    let (normal_texture, normal_scale) = match material.normal_texture() {
        Some(info) => {
            assert!(
                info.tex_coord() == 0,
                "non 0 TEXCOORD not supported (normal)"
            );
            let image =
                load_texture(load_ctx, gltf_cache, buffer, &info.texture(), NORMAL_FORMAT).await;
            (image, info.scale())
        }
        None => {
            let image = create_single_pixel_texture(load_ctx, gltf_cache, NORMAL_DEFAULT).await;
            (image, 1.0)
        }
    };

    let emissive_factor = material.emissive_factor();
    let emissive_texture = match material.emissive_texture() {
        Some(info) => {
            assert!(
                info.tex_coord() == 0,
                "non 0 TEXCOORD not supported (emissive)"
            );
            load_texture(
                load_ctx,
                gltf_cache,
                buffer,
                &info.texture(),
                EMMISIVE_FORMAT,
            )
            .await
        }
        None => create_single_pixel_texture(load_ctx, gltf_cache, EMISSIVE_DEFAULT).await,
    };

    let name = material.name().expect("could not get material name");
    let material_handle = load_ctx
        .insert_asset_scoped::<Material, NamedInserter>(
            name,
            Material {
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
            },
        )
        .await;

    if let Some(index) = material.index() {
        gltf_cache.materials.insert(index, material_handle.clone());
    }

    material_handle
}

#[derive(Debug, Clone)]
pub struct Gltf {
    pub nodes: Vec<GltfNode>,
    pub named_nodes: HashMap<Box<str>, GltfNode>,
    pub meshes: Vec<GltfMesh>,
    pub named_meshes: HashMap<Box<str>, GltfMesh>,
}

#[derive(Debug, Clone)]
pub struct GltfNode {
    pub name: String,
    pub mesh: Option<GltfMesh>,
    pub transform: Transform3D,
    pub children: Vec<GltfNode>,
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

// TODO: should not be defined here
#[derive(Hash, Clone, Debug)]
pub struct TextureRef {
    pub image_handle: AssetHandle<RgbaImage>,
    pub sampler_config: SamplerBuilder,
    pub texture_config: TextureBuilder,
}

// TODO: make textures optional
// TODO: should not be defined here
#[derive(Debug, Clone)]
pub struct Material {
    pub base_color_texture: TextureRef,
    pub color_factor: [f32; 4],

    pub metallic_roughness_texture: TextureRef,
    pub roughness_factor: f32,
    pub metallic_factor: f32,

    pub occlusion_texture: TextureRef,
    pub occlusion_strength: f32,

    pub normal_texture: TextureRef,
    pub normal_scale: f32,

    pub emissive_texture: TextureRef,
    pub emissive_factor: [f32; 3],
}

impl Material {
    pub fn default(cache: &mut AssetCache) -> Self {
        const BASE_COLOR_DEFAULT: [u8; 4] = [255, 255, 255, 255];
        const NORMAL_DEFAULT: [u8; 4] = [128, 128, 255, 0];
        const METALLIC_ROUGHNESS_DEFAULT: [u8; 4] = [0, 255, 0, 0];
        const OCCLUSION_DEFAULT: [u8; 4] = [255, 0, 0, 0];
        const EMISSIVE_DEFAULT: [u8; 4] = [0, 0, 0, 0];

        fn create_texture(cache: &mut AssetCache, color: [u8; 4], name: &str) -> TextureRef {
            let base_color_texture_image = cache.insert_asset::<RgbaImage, NamedInserter>(
                name,
                RgbaImage::from_pixel(1, 1, image::Rgba(color)),
            );
            TextureRef {
                image_handle: base_color_texture_image,
                texture_config: TextureBuilder::new().with_format(wgpu::TextureFormat::Rgba8Unorm),
                sampler_config: SamplerBuilder::new()
                    .min_mag_filter(wgpu::FilterMode::Nearest, wgpu::FilterMode::Nearest),
            }
        }

        let base_color_texture = create_texture(cache, BASE_COLOR_DEFAULT, "default base color");
        let normal_texture = create_texture(cache, NORMAL_DEFAULT, "default normal");
        let metallic_roughness_texture = create_texture(
            cache,
            METALLIC_ROUGHNESS_DEFAULT,
            "default metallic roughness",
        );
        let occlusion_texture = create_texture(cache, OCCLUSION_DEFAULT, "default base occlusion");
        let emissive_texture = create_texture(cache, EMISSIVE_DEFAULT, "default base emissive");

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
