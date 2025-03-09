use std::{f32::consts::PI, marker::PhantomData};

use crate::{CameraUniform, TransformUniform};
use gbase::{
    render::{self, Vertex, VertexFull, VertexTrait},
    wgpu, Context,
};

//
// Mesh builder
//

pub struct MeshBuilder<T: VertexTrait> {
    vertices: Vec<T>,
    indices: Vec<u32>,
}

impl MeshBuilder<render::VertexFull> {
    pub fn from_glb(ctx: &Context, bytes: &[u8]) -> Self {
        let (document, buffers, images) =
            gltf::import_slice(bytes).expect("could not import from slice");

        let mut scenes = document.scenes();
        if scenes.len() > 1 {
            panic!("glb files with multiple scenes not supported");
        }

        let scene = scenes.next().expect("no scenes found");

        Self {
            vertices: todo!(),
            indices: todo!(),
        }
    }

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
    pub fn new(ctx: &mut Context) -> Self {
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
            .build_run_submit(ctx, |mut pass| {
                pass.set_pipeline(&self.pipeline);
                pass.set_vertex_buffer(0, mesh.vertices().slice(..));
                pass.set_index_buffer(mesh.indices().slice(..), mesh.indices().format());
                pass.set_bind_group(0, Some(bindgroup.as_ref()), &[]);
                pass.draw_indexed(0..mesh.indices().len(), 0, 0..1);
            });
    }
}
