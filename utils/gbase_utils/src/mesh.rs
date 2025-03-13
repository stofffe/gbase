use crate::{CameraUniform, TransformUniform};
use gbase::{
    filesystem,
    glam::Mat4,
    log,
    render::{self, TextureWithView, VertexFull, VertexTrait},
    wgpu::{
        self,
        util::{DeviceExt, RenderEncoder},
    },
    Context,
};
use std::{collections::BTreeMap, marker::PhantomData};

//
// Mesh
//

//
// Material
//

//
// Glb
//

pub fn parse_glb(ctx: &Context, glb_bytes: &[u8]) -> Vec<(GltfPrimitive, GltfMaterial)> {
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
        log::info!("visiting {}", node.name().unwrap_or("---"));
        if node.camera().is_some() {
            log::error!("camera decoding not supported");
        }

        // TODO: not used rn
        let new_transform = transform * Mat4::from_cols_array_2d(&node.transform().matrix());

        if let Some(mesh) = node.mesh() {
            // each primitive has its own material
            // so its basically out Mesh
            for primitive in mesh.primitives() {
                if !matches!(primitive.mode(), gltf::mesh::Mode::Triangles) {
                    panic!("glb loader doesnt support {:?}", primitive.mode());
                }

                let mut mesh = GltfPrimitive::new();

                // parse vertex attributes
                for (sem, attr) in primitive.attributes() {
                    let view = attr.view().expect("buffer view not found");

                    let offset = attr.offset() + view.offset();
                    let length = view.length();
                    let bytes = &buffer[offset..offset + length];

                    match sem {
                        gltf::Semantic::Positions => {
                            mesh.attributes.insert(
                                VertexAttributeId::Position,
                                VertexAttributeValues::Float32x3(
                                    bytemuck::cast_slice::<u8, [f32; 3]>(bytes).to_vec(),
                                ),
                            );
                        }
                        gltf::Semantic::Normals => {
                            mesh.attributes.insert(
                                VertexAttributeId::Normal,
                                VertexAttributeValues::Float32x3(
                                    bytemuck::cast_slice::<u8, [f32; 3]>(bytes).to_vec(),
                                ),
                            );
                        }
                        gltf::Semantic::Tangents => {
                            mesh.attributes.insert(
                                VertexAttributeId::Tangent,
                                VertexAttributeValues::Float32x4(
                                    bytemuck::cast_slice::<u8, [f32; 4]>(bytes).to_vec(),
                                ),
                            );
                        }
                        gltf::Semantic::TexCoords(i) => {
                            mesh.attributes.insert(
                                VertexAttributeId::Uv(i),
                                VertexAttributeValues::Float32x2(
                                    bytemuck::cast_slice::<u8, [f32; 2]>(bytes).to_vec(),
                                ),
                            );
                        }
                        gltf::Semantic::Colors(i) => {
                            mesh.attributes.insert(
                                VertexAttributeId::Color(i),
                                VertexAttributeValues::Float32x3(
                                    bytemuck::cast_slice::<u8, [f32; 3]>(bytes).to_vec(),
                                ),
                            );
                        }
                        gltf::Semantic::Joints(_) => {
                            // TODO: gotta check u16x4 vs u32x4
                            log::warn!("joints not supported in gltf");
                        }
                        gltf::Semantic::Weights(_) => {
                            // f32x4
                            log::warn!("weigths not supported in gltf");
                        } // extras?
                    }
                }
                if !mesh.validate() {
                    log::error!("mesh validation failed");
                }

                // parse indices
                let indices_attr = primitive.indices().expect("could not get indices");
                let view = indices_attr.view().expect("buffer view not found");

                assert!(
                    indices_attr.data_type() == gltf::accessor::DataType::U16,
                    "attribute expected {:?} got {:?}",
                    gltf::accessor::DataType::U32,
                    indices_attr.data_type()
                );
                assert!(
                    indices_attr.dimensions() == gltf::accessor::Dimensions::Scalar,
                    "attribute expected {:?} got {:?}",
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

                let offset = indices_attr.offset() + view.offset();
                let length = view.length();

                let indices = bytemuck::cast_slice::<u8, u16>(&buffer[offset..offset + length])
                    .to_vec()
                    .iter()
                    .map(|&i| i as u32)
                    .collect::<Vec<_>>();

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

                    log::info!("loading image with mime type {}", mime_type);
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
                    let texture = must_load_texture(&info.texture(), &buffer);
                    let sampler = GltfSampler {
                        mag_filter: info.texture().sampler().mag_filter(),
                        min_filter: info.texture().sampler().min_filter(),
                        wrap_s: info.texture().sampler().wrap_s(),
                        wrap_t: info.texture().sampler().wrap_t(),
                    };
                    GltfTexture { texture, sampler }
                });

                let color_factor = pbr.base_color_factor(); // scaling / replacement

                let metallic_roughness_texture = pbr.metallic_roughness_texture().map(|info| {
                    assert!(
                        info.tex_coord() == 0,
                        "non 0 TEXCOORD not supported (metallic rougness)"
                    );
                    let texture = must_load_texture(&info.texture(), &buffer);
                    let sampler = GltfSampler {
                        mag_filter: info.texture().sampler().mag_filter(),
                        min_filter: info.texture().sampler().min_filter(),
                        wrap_s: info.texture().sampler().wrap_s(),
                        wrap_t: info.texture().sampler().wrap_t(),
                    };
                    GltfTexture { texture, sampler }
                });
                let metallic_factor = pbr.metallic_factor(); // scaling / replacement
                let roughness_factor = pbr.roughness_factor(); // scaling / replacement

                let mut normal_scale = 1.0;
                let normal_texture = material.normal_texture().map(|info| {
                    assert!(
                        info.tex_coord() == 0,
                        "non 0 TEXCOORD not supported (normal)"
                    );
                    let texture = must_load_texture(&info.texture(), &buffer);
                    let sampler = GltfSampler {
                        mag_filter: info.texture().sampler().mag_filter(),
                        min_filter: info.texture().sampler().min_filter(),
                        wrap_s: info.texture().sampler().wrap_s(),
                        wrap_t: info.texture().sampler().wrap_t(),
                    };
                    normal_scale = info.scale();
                    GltfTexture { texture, sampler }
                });

                let mut occlusion_strength = 1.0;
                let occlusion_texture = material.occlusion_texture().map(|info| {
                    assert!(
                        info.tex_coord() == 0,
                        "non 0 TEXCOORD not supported (normal)"
                    );
                    let texture = must_load_texture(&info.texture(), &buffer);
                    let sampler = GltfSampler {
                        mag_filter: info.texture().sampler().mag_filter(),
                        min_filter: info.texture().sampler().min_filter(),
                        wrap_s: info.texture().sampler().wrap_s(),
                        wrap_t: info.texture().sampler().wrap_t(),
                    };
                    occlusion_strength = info.strength();
                    GltfTexture { texture, sampler }
                });

                let material = GltfMaterial {
                    base_color_texture,
                    color_factor,
                    metallic_roughness_texture,
                    roughness_factor,
                    metallic_factor,
                    occlusion_texture,
                    occlusion_strength,
                    normal_texture,
                    normal_scale,
                };

                meshes.push((mesh, material));
            }
        }

        // recursively visit children
        for child in node.children() {
            node_stack.push((child, new_transform));
        }
    }

    meshes
}

#[derive(Debug, Clone, Default)]
pub struct GltfTexture {
    sampler: GltfSampler,
    texture: Vec<u8>,
}

#[derive(Debug, Clone, Default)]
pub struct GltfSampler {
    mag_filter: Option<gltf::texture::MagFilter>,
    min_filter: Option<gltf::texture::MinFilter>,
    wrap_s: gltf::texture::WrappingMode,
    wrap_t: gltf::texture::WrappingMode,
}

// TODO: shoudl use handles for textures to reuse
// TODO: emissive
#[derive(Debug, Clone, Default)]
pub struct GltfMaterial {
    base_color_texture: Option<GltfTexture>,
    color_factor: [f32; 4],

    metallic_roughness_texture: Option<GltfTexture>,
    roughness_factor: f32,
    metallic_factor: f32,

    occlusion_texture: Option<GltfTexture>,
    occlusion_strength: f32,

    normal_texture: Option<GltfTexture>,
    normal_scale: f32,
}

impl GltfMaterial {
    pub fn to_material(&self, ctx: &mut Context) -> Material {
        // https://registry.khronos.org/glTF/specs/2.0/glTF-2.0.html#materials-overview

        fn create_texture_or_default(
            ctx: &mut Context,
            texture: &Option<GltfTexture>,
            default: [u8; 4],
        ) -> render::TextureWithView {
            if let Some(tex) = texture {
                let samp = &tex.sampler;

                let texture = crate::texture_builder_from_image_bytes(&tex.texture)
                    .unwrap()
                    .build(ctx);

                // TODO: add mipmap support
                let min_filter = samp
                    .min_filter
                    .map_or(wgpu::FilterMode::Linear, |a| match a {
                        gltf::texture::MinFilter::Nearest
                        | gltf::texture::MinFilter::NearestMipmapLinear
                        | gltf::texture::MinFilter::NearestMipmapNearest => {
                            wgpu::FilterMode::Nearest
                        }
                        gltf::texture::MinFilter::Linear
                        | gltf::texture::MinFilter::LinearMipmapNearest
                        | gltf::texture::MinFilter::LinearMipmapLinear => wgpu::FilterMode::Linear,
                    });

                let mag_filter = samp
                    .mag_filter
                    .map_or(wgpu::FilterMode::Linear, |a| match a {
                        gltf::texture::MagFilter::Nearest => wgpu::FilterMode::Nearest,
                        gltf::texture::MagFilter::Linear => wgpu::FilterMode::Linear,
                    });

                let address_mode_u = match samp.wrap_s {
                    gltf::texture::WrappingMode::ClampToEdge => wgpu::AddressMode::ClampToEdge,
                    gltf::texture::WrappingMode::MirroredRepeat => wgpu::AddressMode::MirrorRepeat,
                    gltf::texture::WrappingMode::Repeat => wgpu::AddressMode::Repeat,
                };
                let address_mode_v = match samp.wrap_t {
                    gltf::texture::WrappingMode::ClampToEdge => wgpu::AddressMode::ClampToEdge,
                    gltf::texture::WrappingMode::MirroredRepeat => wgpu::AddressMode::MirrorRepeat,
                    gltf::texture::WrappingMode::Repeat => wgpu::AddressMode::Repeat,
                };

                let sampler = render::SamplerBuilder::new()
                    .min_mag_filter(min_filter, mag_filter)
                    .address_mode(address_mode_u, address_mode_v, wgpu::AddressMode::Repeat)
                    .build(ctx);

                let view = render::TextureViewBuilder::new(texture.clone()).build(ctx);

                TextureWithView::new(texture, view, sampler)
            } else {
                let texture =
                    render::TextureBuilder::new(render::TextureSource::Data(1, 1, default.into()))
                        .build(ctx);
                let sampler = render::SamplerBuilder::new()
                    .address_mode(
                        wgpu::AddressMode::Repeat,
                        wgpu::AddressMode::Repeat,
                        wgpu::AddressMode::Repeat,
                    )
                    .min_mag_filter(wgpu::FilterMode::Nearest, wgpu::FilterMode::Nearest)
                    .build(ctx);
                let view = render::TextureViewBuilder::new(texture.clone()).build(ctx);

                render::TextureWithView::new(texture, view, sampler)
            };

            let builder = match texture {
                Some(texture) => crate::texture_builder_from_image_bytes(&texture.texture).unwrap(),
                None => {
                    render::TextureBuilder::new(render::TextureSource::Data(1, 1, default.into()))
                }
            };

            builder
                .usage(wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST)
                .build(ctx)
                .with_default_sampler_and_view(ctx)
        }

        let base_color_texture =
            create_texture_or_default(ctx, &self.base_color_texture, [255, 255, 255, 255]);
        let metallic_roughness_texture =
            create_texture_or_default(ctx, &self.metallic_roughness_texture, [0, 255, 0, 0]);
        let normal_texture =
            create_texture_or_default(ctx, &self.normal_texture, [128, 128, 255, 0]);
        let occlusion_texture =
            create_texture_or_default(ctx, &self.occlusion_texture, [255, 0, 0, 0]);

        Material {
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum VertexAttributeId {
    Position,
    Normal,
    Tangent,
    Uv(u32),
    Color(u32),
}

#[derive(Clone, Debug)]
pub enum VertexAttributeValues {
    Float32(Vec<f32>),
    Float32x2(Vec<[f32; 2]>),
    Float32x3(Vec<[f32; 3]>),
    Float32x4(Vec<[f32; 4]>),

    Uint32(Vec<u32>),
    Uint32x2(Vec<[u32; 2]>),
    Uint32x3(Vec<[u32; 3]>),
    Uint32x4(Vec<[u32; 4]>),

    Sint32(Vec<i32>),
    Sint32x2(Vec<[i32; 2]>),
    Sint32x3(Vec<[i32; 3]>),
    Sint32x4(Vec<[i32; 4]>),
}

impl VertexAttributeValues {
    pub fn len(&self) -> usize {
        match self {
            VertexAttributeValues::Float32(vec) => vec.len(),
            VertexAttributeValues::Float32x2(vec) => vec.len(),
            VertexAttributeValues::Float32x3(vec) => vec.len(),
            VertexAttributeValues::Float32x4(vec) => vec.len(),
            VertexAttributeValues::Uint32(vec) => vec.len(),
            VertexAttributeValues::Uint32x2(vec) => vec.len(),
            VertexAttributeValues::Uint32x3(vec) => vec.len(),
            VertexAttributeValues::Uint32x4(vec) => vec.len(),
            VertexAttributeValues::Sint32(vec) => vec.len(),
            VertexAttributeValues::Sint32x2(vec) => vec.len(),
            VertexAttributeValues::Sint32x3(vec) => vec.len(),
            VertexAttributeValues::Sint32x4(vec) => vec.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn as_bytes(&self) -> &[u8] {
        match self {
            VertexAttributeValues::Float32(vec) => bytemuck::cast_slice(vec),
            VertexAttributeValues::Float32x2(vec) => bytemuck::cast_slice(vec),
            VertexAttributeValues::Float32x3(vec) => bytemuck::cast_slice(vec),
            VertexAttributeValues::Float32x4(vec) => bytemuck::cast_slice(vec),
            VertexAttributeValues::Uint32(vec) => bytemuck::cast_slice(vec),
            VertexAttributeValues::Uint32x2(vec) => bytemuck::cast_slice(vec),
            VertexAttributeValues::Uint32x3(vec) => bytemuck::cast_slice(vec),
            VertexAttributeValues::Uint32x4(vec) => bytemuck::cast_slice(vec),
            VertexAttributeValues::Sint32(vec) => bytemuck::cast_slice(vec),
            VertexAttributeValues::Sint32x2(vec) => bytemuck::cast_slice(vec),
            VertexAttributeValues::Sint32x3(vec) => bytemuck::cast_slice(vec),
            VertexAttributeValues::Sint32x4(vec) => bytemuck::cast_slice(vec),
        }
    }
    pub fn format(&self) -> wgpu::VertexFormat {
        match self {
            VertexAttributeValues::Float32(_) => wgpu::VertexFormat::Float32,
            VertexAttributeValues::Float32x2(_) => wgpu::VertexFormat::Float32x2,
            VertexAttributeValues::Float32x3(_) => wgpu::VertexFormat::Float32x3,
            VertexAttributeValues::Float32x4(_) => wgpu::VertexFormat::Float32x4,
            VertexAttributeValues::Uint32(_) => wgpu::VertexFormat::Uint32,
            VertexAttributeValues::Uint32x2(_) => wgpu::VertexFormat::Uint32x2,
            VertexAttributeValues::Uint32x3(_) => wgpu::VertexFormat::Uint32x3,
            VertexAttributeValues::Uint32x4(_) => wgpu::VertexFormat::Uint32x4,
            VertexAttributeValues::Sint32(_) => wgpu::VertexFormat::Sint32,
            VertexAttributeValues::Sint32x2(_) => wgpu::VertexFormat::Sint32x2,
            VertexAttributeValues::Sint32x3(_) => wgpu::VertexFormat::Sint32x3,
            VertexAttributeValues::Sint32x4(_) => wgpu::VertexFormat::Sint32x4,
        }
    }
}

#[derive(Debug, Clone)]
pub struct GltfPrimitive {
    attributes: BTreeMap<VertexAttributeId, VertexAttributeValues>,
    indices: Option<Vec<u32>>,
}

impl GltfPrimitive {
    pub fn new() -> Self {
        Self {
            attributes: BTreeMap::new(),
            indices: None,
        }
    }

    pub fn set_indices(&mut self, indices: Vec<u32>) {
        self.indices = Some(indices);
    }

    pub fn clear_indices(&mut self) {
        self.indices = None;
    }

    pub fn add_attribute(&mut self, id: VertexAttributeId, values: VertexAttributeValues) {
        if let Some(vertex_count) = self.vertex_count() {
            if vertex_count != values.len() as u32 {
                log::warn!("inserting attribute with different vertex count");
            }
        }
        self.attributes.insert(id, values);
    }

    pub fn remove_attribute(&mut self, id: VertexAttributeId) -> Option<VertexAttributeValues> {
        self.attributes.remove(&id)
    }

    pub fn clear_attributes(&mut self) {
        self.attributes.clear();
    }

    pub fn vertex_count(&self) -> Option<u32> {
        self.attributes
            .iter()
            .next()
            .map(|(_, values)| values.len() as u32)
    }

    pub fn index_count(&self) -> Option<u32> {
        self.indices.as_ref().map(|inds| inds.len() as u32)
    }

    pub fn layouts(&self) -> Vec<render::VertexBufferLayout> {
        let attributes = self.attributes.values().collect::<Vec<_>>();

        let mut layouts = Vec::new();

        for attr in attributes.iter() {
            let layout = render::VertexBufferLayout::from_vertex_formats(
                wgpu::VertexStepMode::Vertex,
                vec![attr.format()],
            );
            layouts.push(layout);
        }

        layouts
    }

    pub fn buffers(&self, ctx: &Context) -> Vec<wgpu::Buffer> {
        let mut buffers = Vec::new();
        for (_, values) in self.attributes.iter() {
            let buf = render::device(ctx).create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: values.as_bytes(),
                usage: wgpu::BufferUsages::VERTEX,
            });
            buffers.push(buf);
        }
        buffers
    }

    pub fn index_buffer(&self, ctx: &Context) -> Option<wgpu::Buffer> {
        match &self.indices {
            Some(indices) => Some(render::device(ctx).create_buffer_init(
                &wgpu::util::BufferInitDescriptor {
                    label: None,
                    contents: bytemuck::cast_slice(&indices),
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::INDEX,
                },
            )),
            None => None,
        }
    }

    /// Checks
    ///
    /// At least one attribute exists
    /// All attributes have the same length
    pub fn validate(&self) -> bool {
        if self.attributes.is_empty() {
            return false;
        }
        let first_attribute_len = self.attributes.iter().next().unwrap().1.len();
        for (_, values) in self.attributes.iter().skip(1) {
            if values.len() != first_attribute_len {
                return false;
            }
        }
        true
    }
}

//
// Generic Material
//

pub struct Material {
    base_color_texture: render::TextureWithView,
    color_factor: [f32; 4],

    metallic_roughness_texture: render::TextureWithView,
    roughness_factor: f32,
    metallic_factor: f32,

    occlusion_texture: render::TextureWithView,
    occlusion_strength: f32,

    normal_texture: render::TextureWithView,
    normal_scale: f32,
}

//
// Generic Mesh
//

#[derive()]
pub struct Mesh<T: VertexTrait> {
    vertices: render::VertexBuffer<T>,
    indices: render::IndexBuffer,
}

impl<T: VertexTrait> Mesh<T> {
    pub fn vertices(&self) -> &render::VertexBuffer<T> {
        &self.vertices
    }
    pub fn indices(&self) -> &render::IndexBuffer {
        &self.indices
    }
}

//
// Mesh renderer
//

pub struct MeshRenderer<T: VertexTrait> {
    pipeline: render::ArcRenderPipeline,
    bindgroup_layout: render::ArcBindGroupLayout,

    vertex_type: PhantomData<T>,

    mesh: GltfPrimitive,
    gltf_material: GltfMaterial,
    material: Material,
    buffers: Vec<wgpu::Buffer>,
    index_buffer: wgpu::Buffer,
}

impl<T: VertexTrait> MeshRenderer<T> {
    pub fn new(ctx: &mut Context, depth_buffer: &render::DepthBuffer) -> Self {
        let mesh_cube = crate::parse_glb(ctx, &filesystem::load_b!("models/ak47.glb").unwrap());
        let (mut mesh, gltf_material) = mesh_cube[0].clone();
        mesh.remove_attribute(VertexAttributeId::Color(0)); // temp

        let buffers = mesh.buffers(ctx);
        let index_buffer = mesh.index_buffer(ctx).expect("index buffer required");
        let material = gltf_material.to_material(ctx);

        let shader =
            render::ShaderBuilder::new(include_str!("../assets/shaders/mesh.wgsl")).build(ctx);

        let bindgroup_layout = render::BindGroupLayoutBuilder::new()
            .entries(vec![
                // camera
                render::BindGroupLayoutEntry::new().uniform().vertex(),
                // transform
                render::BindGroupLayoutEntry::new().uniform().vertex(),
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

        let pipeline_layout = render::PipelineLayoutBuilder::new()
            .bind_groups(vec![bindgroup_layout.clone()])
            .build(ctx);
        let pipeline = render::RenderPipelineBuilder::new(shader, pipeline_layout)
            .buffers(mesh.layouts())
            .single_target(render::ColorTargetState::from_current_screen(ctx))
            .cull_mode(wgpu::Face::Back)
            .depth_stencil(depth_buffer.depth_stencil_state())
            .build(ctx);

        Self {
            pipeline,
            bindgroup_layout,

            vertex_type: PhantomData::<T>,

            mesh,
            gltf_material,
            material,
            buffers,
            index_buffer,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn render(
        &mut self,
        ctx: &mut Context,
        view: &wgpu::TextureView,
        camera: &render::UniformBuffer<CameraUniform>,
        transform: &render::UniformBuffer<TransformUniform>,
        depth_buffer: &render::DepthBuffer,
    ) {
        let bindgroup = render::BindGroupBuilder::new(self.bindgroup_layout.clone())
            .entries(vec![
                // camera
                render::BindGroupEntry::Buffer(camera.buffer()),
                // model
                render::BindGroupEntry::Buffer(transform.buffer()),
                // base color texture
                render::BindGroupEntry::Texture(self.material.base_color_texture.view()),
                // base color sampler
                render::BindGroupEntry::Sampler(self.material.base_color_texture.sampler()),
                // normal texture
                render::BindGroupEntry::Texture(self.material.normal_texture.view()),
                // normal sampler
                render::BindGroupEntry::Sampler(self.material.normal_texture.sampler()),
                // metallic roughness texture
                render::BindGroupEntry::Texture(self.material.metallic_roughness_texture.view()),
                // metallic roughness sampler
                render::BindGroupEntry::Sampler(self.material.metallic_roughness_texture.sampler()),
                // occlusion roughness texture
                render::BindGroupEntry::Texture(self.material.occlusion_texture.view()),
                // occlusion roughness sampler
                render::BindGroupEntry::Sampler(self.material.occlusion_texture.sampler()),
            ])
            .build(ctx);

        render::RenderPassBuilder::new()
            .color_attachments(&[Some(render::RenderPassColorAttachment::new(view))])
            .depth_stencil_attachment(depth_buffer.depth_render_attachment_load())
            .build_run_submit(ctx, |mut pass| {
                pass.set_pipeline(&self.pipeline);

                for i in 0..self.buffers.len() {
                    pass.set_vertex_buffer(i as u32, self.buffers[i].slice(..));
                }
                pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                pass.set_bind_group(0, Some(bindgroup.as_ref()), &[]);

                if let Some(index_count) = self.mesh.index_count() {
                    pass.draw_indexed(0..index_count, 0, 0..1);
                } else {
                    pass.draw(0..self.mesh.vertex_count().unwrap(), 0..1);
                }
            });
    }
}

//
// Mesh builder
//

pub struct MeshBuilder<T: VertexTrait> {
    vertices: Vec<T>,
    indices: Vec<u32>,
}

impl MeshBuilder<render::VertexFull> {
    pub fn new() -> Self {
        Self {
            vertices: Vec::new(),
            indices: Vec::new(),
        }
    }

    pub fn quad(mut self) -> Self {
        const POSITIONS: [[f32; 3]; 4] = [
            [-0.5, -0.5, 0.0], // bottom left
            [0.5, -0.5, 0.0],  // bottom right
            [-0.5, 0.5, 0.0],  // top left
            [0.5, 0.5, 0.0],   // top right
        ];
        const UVS: [[f32; 2]; 4] = [
            [0.0, 1.0], // bottom left
            [1.0, 1.0], // bottom right
            [0.0, 0.0], // top left
            [1.0, 0.0], // top right
        ];
        const INDICES: [u32; 6] = [
            0, 1, 3, //
            0, 3, 2, //
        ];

        for i in 0..4 {
            self.vertices.push(VertexFull {
                position: POSITIONS[i],
                color: [1.0, 1.0, 1.0, 1.0],
                normal: [0.0, 0.0, 1.0],
                uv: UVS[i],
                tangent: [1.0, 0.0, 0.0, 1.0],
            });
        }

        let start_index = self.vertices.len() - POSITIONS.len();
        for ind in INDICES {
            self.indices.push(start_index as u32 + ind);
        }

        self
    }

    pub fn cube(mut self) -> Self {
        const POSITIONS: [[f32; 3]; 24] = [
            // front
            [-0.5, -0.5, 0.5], // bottom left
            [0.5, -0.5, 0.5],  // bottom right
            [-0.5, 0.5, 0.5],  // top left
            [0.5, 0.5, 0.5],   // top right
            // back
            [0.5, -0.5, -0.5],  // bottom left
            [-0.5, -0.5, -0.5], // bottom right
            [0.5, 0.5, -0.5],   // top left
            [-0.5, 0.5, -0.5],  // top right
            // left
            [-0.5, -0.5, -0.5], // bottom left
            [-0.5, -0.5, 0.5],  // bottom right
            [-0.5, 0.5, -0.5],  // bottom left
            [-0.5, 0.5, 0.5],   // bottom right
            // right
            [0.5, -0.5, 0.5],  // bottom left
            [0.5, -0.5, -0.5], // bottom right
            [0.5, 0.5, 0.5],   // bottom left
            [0.5, 0.5, -0.5],  // bottom right
            // bottom
            [-0.5, -0.5, -0.5], // bottom left
            [0.5, -0.5, -0.5],  // bottom right
            [-0.5, -0.5, 0.5],  // bottom left
            [0.5, -0.5, 0.5],   // bottom right
            // top
            [-0.5, 0.5, 0.5],  // bottom left
            [0.5, 0.5, 0.5],   // bottom right
            [-0.5, 0.5, -0.5], // bottom left
            [0.5, 0.5, -0.5],  // bottom right
        ];
        const NORMALS: [[f32; 3]; 24] = [
            // front
            [0.0, 0.0, 1.0], // bottom left
            [0.0, 0.0, 1.0], // bottom right
            [0.0, 0.0, 1.0], // top left
            [0.0, 0.0, 1.0], // top right
            // back
            [0.0, 0.0, -1.0], // bottom left
            [0.0, 0.0, -1.0], // bottom right
            [0.0, 0.0, -1.0], // top left
            [0.0, 0.0, -1.0], // top right
            // left
            [-1.0, 0.0, 0.0], // bottom left
            [-1.0, 0.0, 0.0], // bottom right
            [-1.0, 0.0, 0.0], // top left
            [-1.0, 0.0, 0.0], // top right
            // right
            [1.0, 0.0, 0.0], // bottom left
            [1.0, 0.0, 0.0], // bottom right
            [1.0, 0.0, 0.0], // top left
            [1.0, 0.0, 0.0], // top right
            // bottom
            [0.0, -1.0, 0.0], // bottom left
            [0.0, -1.0, 0.0], // bottom right
            [0.0, -1.0, 0.0], // top left
            [0.0, -1.0, 0.0], // top right
            // top
            [0.0, 1.0, 0.0], // bottom left
            [0.0, 1.0, 0.0], // bottom right
            [0.0, 1.0, 0.0], // top left
            [0.0, 1.0, 0.0], // top right
        ];

        const UVS: [[f32; 2]; 24] = [
            // front
            [0.0, 1.0], // bottom left
            [1.0, 1.0], // bottom right
            [0.0, 0.0], // top left
            [1.0, 0.0], // top right
            // back
            [0.0, 0.0], // bottom left
            [1.0, 0.0], // bottom right
            [0.0, 1.0], // top left
            [1.0, 1.0], // top right
            // left
            [0.0, 1.0], // bottom left
            [1.0, 1.0], // bottom right
            [0.0, 0.0], // top left
            [1.0, 0.0], // top right
            // right
            [0.0, 1.0], // bottom left
            [1.0, 1.0], // bottom right
            [0.0, 0.0], // top left
            [1.0, 0.0], // top right
            // bottom
            [0.0, 1.0], // bottom left
            [1.0, 1.0], // bottom right
            [0.0, 0.0], // top left
            [1.0, 0.0], // top right
            // top
            [0.0, 1.0], // bottom left
            [1.0, 1.0], // bottom right
            [0.0, 0.0], // top left
            [1.0, 0.0], // top right
        ];

        const TANGENTS: [[f32; 4]; 24] = [
            // front
            [1.0, 0.0, 0.0, 1.0], // bottom left
            [1.0, 0.0, 0.0, 1.0], // bottom right
            [1.0, 0.0, 0.0, 1.0], // top left
            [1.0, 0.0, 0.0, 1.0], // top right
            // back
            [1.0, 0.0, 0.0, 1.0], // bottom left
            [1.0, 0.0, 0.0, 1.0], // bottom right
            [1.0, 0.0, 0.0, 1.0], // top left
            [1.0, 0.0, 0.0, 1.0], // top right
            // left
            [0.0, 0.0, 1.0, 1.0], // bottom left
            [0.0, 0.0, 1.0, 1.0], // bottom right
            [0.0, 0.0, 1.0, 1.0], // top left
            [0.0, 0.0, 1.0, 1.0], // top right
            // right
            [0.0, 0.0, -1.0, 1.0], // bottom left
            [0.0, 0.0, -1.0, 1.0], // bottom right
            [0.0, 0.0, -1.0, 1.0], // top left
            [0.0, 0.0, -1.0, 1.0], // top right
            // bottom
            [1.0, 0.0, 0.0, 1.0], // bottom left
            [1.0, 0.0, 0.0, 1.0], // bottom right
            [1.0, 0.0, 0.0, 1.0], // top left
            [1.0, 0.0, 0.0, 1.0], // top right
            // top
            [1.0, 0.0, 0.0, 1.0], // bottom left
            [1.0, 0.0, 0.0, 1.0], // bottom right
            [1.0, 0.0, 0.0, 1.0], // top left
            [1.0, 0.0, 0.0, 1.0], // top right
        ];

        const INDICES: [u32; 36] = [
            // front
            0, 1, 3, //
            0, 3, 2, //
            // back
            4, 5, 7, //
            4, 7, 6, //
            // left
            8, 9, 11, //
            8, 11, 10, //
            // right
            12, 13, 15, //
            12, 15, 14, //
            // bottom
            16, 17, 19, //
            16, 19, 18, //
            // top
            20, 21, 23, //
            20, 23, 22, //
        ];

        const VERTS: usize = 24;
        const INDS: usize = 36;
        for i in 0..VERTS {
            self.vertices.push(render::VertexFull {
                position: POSITIONS[i],
                color: [1.0, 1.0, 1.0, 1.0],
                normal: NORMALS[i],
                uv: UVS[i],
                tangent: TANGENTS[i],
            });
        }

        let start_index = self.vertices.len() - VERTS;
        for i in 0..INDS {
            self.indices.push(start_index as u32 + INDICES[i]);
        }

        self
    }

    // pub fn circle(mut self, parts: u32) -> Self {
    //     self.vertices.push(Vertex {
    //         position: [0.0, 0.0, 0.0],
    //     });
    //     for i in 0..parts {
    //         let angle = (i as f32 / parts as f32) * PI * 2.0;
    //         self.vertices.push(Vertex {
    //             position: [0.5 * angle.cos(), 0.5 * angle.sin(), 0.0],
    //         });
    //     }
    //
    //     let start_index = self.vertices.len() as u32 - parts;
    //     let center_index = start_index - 1;
    //     for i in 0..parts {
    //         self.indices.push(center_index);
    //         self.indices.push(start_index + i);
    //         self.indices.push(start_index + (i + 1) % parts);
    //     }
    //
    //     self
    // }

    pub fn build(self, ctx: &Context) -> Mesh<render::VertexFull> {
        let vertices =
            render::VertexBufferBuilder::new(render::VertexBufferSource::Data(self.vertices))
                .build(ctx);
        let indices =
            render::IndexBufferBuilder::new(render::IndexBufferSource::Data(self.indices))
                .build(ctx);
        Mesh { vertices, indices }
    }
}

// // NOTE: bevy
// #[derive(Debug, Copy, Clone, PartialEq, Eq, Ord, PartialOrd, Hash)]
// pub struct VertexAttributeId(u64);
//
// impl VertexAttributeInfo {
//     pub const ATTRIBUTE_POSITION: VertexAttributeInfo =
//         VertexAttributeInfo::new("Vertex_Position", 0, VertexFormat::Float32x3);
//
//     /// The direction the vertex normal is facing in.
//     /// Use in conjunction with [`Mesh::insert_attribute`] or [`Mesh::with_inserted_attribute`].
//     ///
//     /// The format of this attribute is [`VertexFormat::Float32x3`].
//     pub const ATTRIBUTE_NORMAL: MeshVertexAttribute =
//         MeshVertexAttribute::new("Vertex_Normal", 1, VertexFormat::Float32x3);
//
//     /// Texture coordinates for the vertex. Use in conjunction with [`Mesh::insert_attribute`]
//     /// or [`Mesh::with_inserted_attribute`].
//     ///
//     /// Generally `[0.,0.]` is mapped to the top left of the texture, and `[1.,1.]` to the bottom-right.
//     ///
//     /// By default values outside will be clamped per pixel not for the vertex,
//     /// "stretching" the borders of the texture.
//     /// This behavior can be useful in some cases, usually when the borders have only
//     /// one color, for example a logo, and you want to "extend" those borders.
//     ///
//     /// For different mapping outside of `0..=1` range,
//     /// see [`ImageAddressMode`](bevy_image::ImageAddressMode).
//     ///
//     /// The format of this attribute is [`VertexFormat::Float32x2`].
//     pub const ATTRIBUTE_UV_0: MeshVertexAttribute =
//         MeshVertexAttribute::new("Vertex_Uv", 2, VertexFormat::Float32x2);
//
//     /// Alternate texture coordinates for the vertex. Use in conjunction with
//     /// [`Mesh::insert_attribute`] or [`Mesh::with_inserted_attribute`].
//     ///
//     /// Typically, these are used for lightmaps, textures that provide
//     /// precomputed illumination.
//     ///
//     /// The format of this attribute is [`VertexFormat::Float32x2`].
//     pub const ATTRIBUTE_UV_1: MeshVertexAttribute =
//         MeshVertexAttribute::new("Vertex_Uv_1", 3, VertexFormat::Float32x2);
//
//     /// The direction of the vertex tangent. Used for normal mapping.
//     /// Usually generated with [`generate_tangents`](Mesh::generate_tangents) or
//     /// [`with_generated_tangents`](Mesh::with_generated_tangents).
//     ///
//     /// The format of this attribute is [`VertexFormat::Float32x4`].
//     pub const ATTRIBUTE_TANGENT: MeshVertexAttribute =
//         MeshVertexAttribute::new("Vertex_Tangent", 4, VertexFormat::Float32x4);
//
//     /// Per vertex coloring. Use in conjunction with [`Mesh::insert_attribute`]
//     /// or [`Mesh::with_inserted_attribute`].
//     ///
//     /// The format of this attribute is [`VertexFormat::Float32x4`].
//     pub const ATTRIBUTE_COLOR: MeshVertexAttribute =
//         MeshVertexAttribute::new("Vertex_Color", 5, VertexFormat::Float32x4);
//
//     /// Per vertex joint transform matrix weight. Use in conjunction with [`Mesh::insert_attribute`]
//     /// or [`Mesh::with_inserted_attribute`].
//     ///
//     /// The format of this attribute is [`VertexFormat::Float32x4`].
//     pub const ATTRIBUTE_JOINT_WEIGHT: MeshVertexAttribute =
//         MeshVertexAttribute::new("Vertex_JointWeight", 6, VertexFormat::Float32x4);
//
//     /// Per vertex joint transform matrix index. Use in conjunction with [`Mesh::insert_attribute`]
//     /// or [`Mesh::with_inserted_attribute`].
//     ///
//     /// The format of this attribute is [`VertexFormat::Uint16x4`].
//     pub const ATTRIBUTE_JOINT_INDEX: MeshVertexAttribute =
//         MeshVertexAttribute::new("Vertex_JointIndex", 7, VertexFormat::Uint16x4);
// }
//
// // NOTE: bevy
// #[derive(Debug, Clone, Copy)]
// pub struct VertexAttributeInfo {
//     /// The friendly name of the vertex attribute
//     pub name: &'static str,
//     pub id: VertexAttributeId,
//     pub format: wgpu::VertexFormat,
// }
