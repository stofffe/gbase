use crate::{CameraUniform, TransformUniform};
use gbase::{
    glam::Mat4,
    log,
    render::{self, VertexFull, VertexTrait},
    wgpu, Context,
};
use std::marker::PhantomData;

pub fn parse_glb(ctx: &Context, glb_bytes: &[u8]) -> Vec<Mesh<render::VertexFull>> {
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

        let new_transform = transform * Mat4::from_cols_array_2d(&node.transform().matrix());

        if let Some(mesh) = node.mesh() {
            // each primitive has its own material
            // so its basically out Mesh
            for primitive in mesh.primitives() {
                if !matches!(primitive.mode(), gltf::mesh::Mode::Triangles) {
                    panic!("glb loader doesnt support {:?}", primitive.mode());
                }

                for a in primitive.attributes() {
                    log::warn!("{:?}", a.0)
                }

                fn must_parse_attr<'a>(
                    p: &'a gltf::Primitive<'a>,
                    semantic: gltf::Semantic,
                    assert_type: gltf::accessor::DataType,
                    assert_dimensions: gltf::accessor::Dimensions,
                    buffer: &[u8],
                ) -> Vec<u8> {
                    let attr = p
                        .attributes()
                        .find(|(sem, _)| *sem == semantic)
                        .map(|(_, acc)| acc)
                        .unwrap_or_else(|| panic!("attribute not found {:?}", semantic));

                    let view = attr.view().expect("buffer view not found");

                    assert!(
                        attr.data_type() == assert_type,
                        "attribute expected {:?} got {:?}",
                        assert_type,
                        attr.data_type()
                    );
                    assert!(
                        attr.dimensions() == assert_dimensions,
                        "attribute expected {:?} got {:?}",
                        assert_dimensions,
                        attr.dimensions()
                    );
                    assert!(
                        matches!(view.buffer().source(), gltf::buffer::Source::Bin),
                        "buffer source URI not supported"
                    );
                    assert!(
                        view.stride().is_none(),
                        "attribute data with stride not supported"
                    );

                    let offset = attr.offset() + view.offset();
                    let length = view.length();

                    buffer[offset..offset + length].to_vec()
                }

                let positions = must_parse_attr(
                    &primitive,
                    gltf::Semantic::Positions,
                    gltf::accessor::DataType::F32,
                    gltf::accessor::Dimensions::Vec3,
                    &buffer,
                );
                // let colors = must_parse_attr(
                //     &primitive,
                //     gltf::Semantic::Colors(0),
                //     gltf::accessor::DataType::F32,
                //     gltf::accessor::Dimensions::Vec3,
                //     &buffer,
                // );
                let uvs = must_parse_attr(
                    &primitive,
                    gltf::Semantic::TexCoords(0),
                    gltf::accessor::DataType::F32,
                    gltf::accessor::Dimensions::Vec2,
                    &buffer,
                );
                let normals = must_parse_attr(
                    &primitive,
                    gltf::Semantic::Normals,
                    gltf::accessor::DataType::F32,
                    gltf::accessor::Dimensions::Vec3,
                    &buffer,
                );
                let tangents = must_parse_attr(
                    &primitive,
                    gltf::Semantic::Tangents,
                    gltf::accessor::DataType::F32,
                    gltf::accessor::Dimensions::Vec4,
                    &buffer,
                );

                let positions_f32 = bytemuck::cast_slice::<u8, f32>(&positions)
                    .chunks(3)
                    .collect::<Vec<_>>();
                // let colors_f32 = bytemuck::cast_slice::<u8, f32>(&colors)
                //     .chunks(3)
                //     .collect::<Vec<_>>();
                let uvs_f32 = bytemuck::cast_slice::<u8, f32>(&uvs)
                    .chunks(2)
                    .collect::<Vec<_>>();
                let normals_f32 = bytemuck::cast_slice::<u8, f32>(&normals)
                    .chunks(3)
                    .collect::<Vec<_>>();
                let tangents_f32 = bytemuck::cast_slice::<u8, f32>(&tangents)
                    .chunks(4)
                    .collect::<Vec<_>>();

                let mut vertices = Vec::new();
                for i in 0..positions_f32.len() {
                    let vertex = render::VertexFull {
                        position: [
                            positions_f32[i][0],
                            positions_f32[i][1],
                            positions_f32[i][2],
                        ],
                        color: [1.0, 1.0, 1.0, 1.0],
                        // color: [colors_f32[i][0], colors_f32[i][1], colors_f32[i][2], 1.0],
                        normal: [normals_f32[i][0], normals_f32[i][1], normals_f32[i][2]],
                        uv: [uvs_f32[i][0], uvs_f32[i][1]],
                        tangent: [
                            tangents_f32[i][0],
                            tangents_f32[i][1],
                            tangents_f32[i][2],
                            tangents_f32[i][3],
                        ],
                    };
                    vertices.push(vertex);
                }

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

                let mesh = MeshBuilder { vertices, indices }.build(ctx);

                meshes.push(mesh);
            }
        }

        // recursively visit children
        for child in node.children() {
            node_stack.push((child, new_transform));
        }
    }

    meshes
}

//
// Mesh builder
//
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

//
// Mesh
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
}

impl<T: VertexTrait> MeshRenderer<T> {
    pub fn new(ctx: &mut Context, depth_buffer: &render::DepthBuffer) -> Self {
        let shader =
            render::ShaderBuilder::new(include_str!("../assets/shaders/mesh.wgsl")).build(ctx);

        let bindgroup_layout = render::BindGroupLayoutBuilder::new()
            .entries(vec![
                // camera
                render::BindGroupLayoutEntry::new().uniform().vertex(),
                // transform
                render::BindGroupLayoutEntry::new().uniform().vertex(),
                // albedo texture
                render::BindGroupLayoutEntry::new()
                    .texture_float_filterable()
                    .fragment(),
                // albedo sampler
                render::BindGroupLayoutEntry::new()
                    .sampler_filtering()
                    .fragment(),
            ])
            .build(ctx);

        let pipeline_layout = render::PipelineLayoutBuilder::new()
            .bind_groups(vec![bindgroup_layout.clone()])
            .build(ctx);
        let pipeline = render::RenderPipelineBuilder::new(shader, pipeline_layout)
            .buffers(vec![T::desc()])
            .single_target(render::ColorTargetState::from_current_screen(ctx))
            .cull_mode(wgpu::Face::Back)
            .depth_stencil(depth_buffer.depth_stencil_state())
            .build(ctx);

        Self {
            pipeline,
            bindgroup_layout,

            vertex_type: PhantomData::<T>,
        }
    }

    pub fn render(
        &mut self,
        ctx: &mut Context,
        view: &wgpu::TextureView,
        camera: &render::UniformBuffer<CameraUniform>,
        mesh: &Mesh<render::VertexFull>,
        transform: &render::UniformBuffer<TransformUniform>,
        albedo: &render::TextureWithView,
        albedo_sampler: &render::ArcSampler,
        depth_buffer: &render::DepthBuffer,
    ) {
        let bindgroup = render::BindGroupBuilder::new(self.bindgroup_layout.clone())
            .entries(vec![
                // camera
                render::BindGroupEntry::Buffer(camera.buffer()),
                // model
                render::BindGroupEntry::Buffer(transform.buffer()),
                // albedo texture
                render::BindGroupEntry::Texture(albedo.view()),
                // albedo sampler
                render::BindGroupEntry::Sampler(albedo_sampler.clone()),
            ])
            .build(ctx);

        render::RenderPassBuilder::new()
            .color_attachments(&[Some(render::RenderPassColorAttachment::new(view))])
            .depth_stencil_attachment(depth_buffer.depth_render_attachment_load())
            .build_run_submit(ctx, |mut pass| {
                pass.set_pipeline(&self.pipeline);
                pass.set_vertex_buffer(0, mesh.vertices().slice(..));
                pass.set_index_buffer(mesh.indices().slice(..), mesh.indices().format());
                pass.set_bind_group(0, Some(bindgroup.as_ref()), &[]);
                pass.draw_indexed(0..mesh.indices().len(), 0, 0..1);
            });
    }
}
