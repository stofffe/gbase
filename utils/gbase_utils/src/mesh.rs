use std::collections::{BTreeMap, BTreeSet};

use gbase::{
    log, render,
    wgpu::{self, util::DeviceExt},
    Context,
};

//
// CPU
//

#[derive(Debug, Clone)]
pub struct Mesh {
    pub primitive_topology: wgpu::PrimitiveTopology,
    pub attributes: BTreeMap<VertexAttributeId, VertexAttributeValues>,
    pub indices: Option<Vec<u32>>,
}

impl Mesh {
    pub fn new(primitive_topology: wgpu::PrimitiveTopology) -> Self {
        Self {
            primitive_topology,
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

    pub fn get_attribute(&self, id: VertexAttributeId) -> Option<&VertexAttributeValues> {
        self.attributes.get(&id)
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
        let mut layouts = Vec::new();

        for attr in self.attributes.keys() {
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
        self.indices.as_ref().map(|indices| {
            render::device(ctx).create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(indices),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::INDEX,
            })
        })
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

    pub fn require_exact_attributes(mut self, attributes: &BTreeSet<VertexAttributeId>) -> Self {
        // remove
        self.attributes = self
            .clone()
            .attributes
            .into_iter()
            .filter(|(id, _)| attributes.contains(id))
            .collect::<BTreeMap<VertexAttributeId, VertexAttributeValues>>();

        // add
        for attr in attributes {
            if !self.attributes.contains_key(attr) {
                match attr {
                    VertexAttributeId::Normal => {
                        log::warn!(
                            "normal attribute could not be found, generating for each vertex"
                        );
                        self.generate_normals();
                    }
                    VertexAttributeId::Tangent => {
                        log::warn!(
                            "tangent attribute could not be found, generating for each vertex"
                        );
                        self.generate_tangents();
                    }
                    VertexAttributeId::Color(i) => {
                        log::warn!(
                        "color attribute could not be found, generating [1,1,1] for each vertex"
                    );
                        self.generate_colors(*i, [1.0, 1.0, 1.0]);
                    }
                    id => {
                        panic!("vertex attributes does not contain required {:?}", id);
                    }
                }
            }
        }

        self
    }

    pub fn generate_normals(&mut self) {
        assert!(matches!(
            self.primitive_topology,
            wgpu::PrimitiveTopology::TriangleList
        ));
        if self.indices.is_some() {
            self.generate_smoothed_normals();
        } else {
            self.generate_flat_normals();
        }
    }
    pub fn generate_flat_normals(&mut self) {
        todo!()
    }
    pub fn generate_smoothed_normals(&mut self) {
        todo!()
    }

    pub fn generate_tangents(&mut self) {
        todo!()
    }
    pub fn generate_colors(&mut self, color_index: u32, color: [f32; 3]) {
        let Some(count) = self.vertex_count() else {
            log::error!("trying to generate colors for mesh without vertices");
            return;
        };

        let mut colors = Vec::with_capacity(count as usize);
        for _ in 0..count {
            colors.push(color);
        }

        self.add_attribute(
            VertexAttributeId::Color(color_index),
            VertexAttributeValues::Float32x3(colors),
        );
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

impl VertexAttributeId {
    pub fn format(&self) -> wgpu::VertexFormat {
        match self {
            VertexAttributeId::Position => wgpu::VertexFormat::Float32x3,
            VertexAttributeId::Normal => wgpu::VertexFormat::Float32x3,
            VertexAttributeId::Tangent => wgpu::VertexFormat::Float32x4,
            VertexAttributeId::Uv(_) => wgpu::VertexFormat::Float32x2,
            VertexAttributeId::Color(_) => wgpu::VertexFormat::Float32x3,
        }
    }
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
}

//
// GPU
//

pub struct GpuMesh {
    pub attribute_buffer: render::ArcBuffer,
    pub attribute_ranges: BTreeMap<VertexAttributeId, (u64, u64)>,
    pub index_buffer: Option<render::ArcBuffer>,
    // TODO: add vertex and index count?
    pub vertex_count: u32,
    pub index_count: Option<u32>,
}

impl GpuMesh {
    pub fn new(ctx: &Context, mesh: &crate::Mesh) -> Self {
        let mut cursor = 0;
        let mut combined_bytes = Vec::new();
        let mut attribute_ranges = BTreeMap::new();
        for (&id, values) in mesh.attributes.iter() {
            let start = cursor;
            for value in values.as_bytes() {
                combined_bytes.push(*value);
                cursor += 1;
            }
            let end = cursor;
            attribute_ranges.insert(id, (start, end));
        }

        let mut index_buffer = None;
        if let Some(indices) = &mesh.indices {
            let buffer =
                render::RawBufferBuilder::new(render::RawBufferSource::Data(indices.clone()))
                    .usage(wgpu::BufferUsages::INDEX)
                    .build(ctx);
            index_buffer = Some(buffer.buffer());
        }

        let buffer =
            render::RawBufferBuilder::new(render::RawBufferSource::Data(combined_bytes)).build(ctx);

        let vertex_count = mesh.vertex_count().expect("must have at least one vertex");
        let index_count = mesh.index_count();

        Self {
            attribute_buffer: buffer.buffer(),
            attribute_ranges,
            index_buffer,
            vertex_count,
            index_count,
        }
    }
}

//
// Mesh builder
//

pub struct MeshBuilder {
    positions: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    normals: Vec<[f32; 3]>,
    tangents: Vec<[f32; 4]>,
    indices: Vec<u32>,
}

impl MeshBuilder {
    pub fn quad() -> Self {
        const POSITIONS: [[f32; 3]; 4] = [
            [-0.5, -0.5, 0.0], // bottom left
            [0.5, -0.5, 0.0],  // bottom right
            [-0.5, 0.5, 0.0],  // top left
            [0.5, 0.5, 0.0],   // top right
        ];
        const NORMALS: [[f32; 3]; 4] = [
            [0.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
        ];
        const UVS: [[f32; 2]; 4] = [
            [0.0, 1.0], // bottom left
            [1.0, 1.0], // bottom right
            [0.0, 0.0], // top left
            [1.0, 0.0], // top right
        ];
        const TANGENTS: [[f32; 4]; 4] = [
            [1.0, 0.0, 0.0, 1.0],
            [1.0, 0.0, 0.0, 1.0],
            [1.0, 0.0, 0.0, 1.0],
            [1.0, 0.0, 0.0, 1.0],
        ];
        const INDICES: [u32; 6] = [
            0, 1, 3, //
            0, 3, 2, //
        ];

        let mut positions = Vec::new();
        let mut normals = Vec::new();
        let mut uvs = Vec::new();
        let mut tangents = Vec::new();
        let mut indices = Vec::new();

        let ind_offset = positions.len() as u32;

        for pos in POSITIONS {
            positions.push(pos);
        }
        for normal in NORMALS {
            normals.push(normal);
        }
        for uv in UVS {
            uvs.push(uv);
        }
        for tangent in TANGENTS {
            tangents.push(tangent);
        }
        for ind in INDICES {
            indices.push(ind + ind_offset);
        }

        Self {
            positions,
            uvs,
            normals,
            tangents,
            indices,
        }
    }

    // pub fn cube(mut self) -> Self {
    //     const POSITIONS: [[f32; 3]; 24] = [
    //         // front
    //         [-0.5, -0.5, 0.5], // bottom left
    //         [0.5, -0.5, 0.5],  // bottom right
    //         [-0.5, 0.5, 0.5],  // top left
    //         [0.5, 0.5, 0.5],   // top right
    //         // back
    //         [0.5, -0.5, -0.5],  // bottom left
    //         [-0.5, -0.5, -0.5], // bottom right
    //         [0.5, 0.5, -0.5],   // top left
    //         [-0.5, 0.5, -0.5],  // top right
    //         // left
    //         [-0.5, -0.5, -0.5], // bottom left
    //         [-0.5, -0.5, 0.5],  // bottom right
    //         [-0.5, 0.5, -0.5],  // bottom left
    //         [-0.5, 0.5, 0.5],   // bottom right
    //         // right
    //         [0.5, -0.5, 0.5],  // bottom left
    //         [0.5, -0.5, -0.5], // bottom right
    //         [0.5, 0.5, 0.5],   // bottom left
    //         [0.5, 0.5, -0.5],  // bottom right
    //         // bottom
    //         [-0.5, -0.5, -0.5], // bottom left
    //         [0.5, -0.5, -0.5],  // bottom right
    //         [-0.5, -0.5, 0.5],  // bottom left
    //         [0.5, -0.5, 0.5],   // bottom right
    //         // top
    //         [-0.5, 0.5, 0.5],  // bottom left
    //         [0.5, 0.5, 0.5],   // bottom right
    //         [-0.5, 0.5, -0.5], // bottom left
    //         [0.5, 0.5, -0.5],  // bottom right
    //     ];
    //     const NORMALS: [[f32; 3]; 24] = [
    //         // front
    //         [0.0, 0.0, 1.0], // bottom left
    //         [0.0, 0.0, 1.0], // bottom right
    //         [0.0, 0.0, 1.0], // top left
    //         [0.0, 0.0, 1.0], // top right
    //         // back
    //         [0.0, 0.0, -1.0], // bottom left
    //         [0.0, 0.0, -1.0], // bottom right
    //         [0.0, 0.0, -1.0], // top left
    //         [0.0, 0.0, -1.0], // top right
    //         // left
    //         [-1.0, 0.0, 0.0], // bottom left
    //         [-1.0, 0.0, 0.0], // bottom right
    //         [-1.0, 0.0, 0.0], // top left
    //         [-1.0, 0.0, 0.0], // top right
    //         // right
    //         [1.0, 0.0, 0.0], // bottom left
    //         [1.0, 0.0, 0.0], // bottom right
    //         [1.0, 0.0, 0.0], // top left
    //         [1.0, 0.0, 0.0], // top right
    //         // bottom
    //         [0.0, -1.0, 0.0], // bottom left
    //         [0.0, -1.0, 0.0], // bottom right
    //         [0.0, -1.0, 0.0], // top left
    //         [0.0, -1.0, 0.0], // top right
    //         // top
    //         [0.0, 1.0, 0.0], // bottom left
    //         [0.0, 1.0, 0.0], // bottom right
    //         [0.0, 1.0, 0.0], // top left
    //         [0.0, 1.0, 0.0], // top right
    //     ];
    //
    //     const UVS: [[f32; 2]; 24] = [
    //         // front
    //         [0.0, 1.0], // bottom left
    //         [1.0, 1.0], // bottom right
    //         [0.0, 0.0], // top left
    //         [1.0, 0.0], // top right
    //         // back
    //         [0.0, 0.0], // bottom left
    //         [1.0, 0.0], // bottom right
    //         [0.0, 1.0], // top left
    //         [1.0, 1.0], // top right
    //         // left
    //         [0.0, 1.0], // bottom left
    //         [1.0, 1.0], // bottom right
    //         [0.0, 0.0], // top left
    //         [1.0, 0.0], // top right
    //         // right
    //         [0.0, 1.0], // bottom left
    //         [1.0, 1.0], // bottom right
    //         [0.0, 0.0], // top left
    //         [1.0, 0.0], // top right
    //         // bottom
    //         [0.0, 1.0], // bottom left
    //         [1.0, 1.0], // bottom right
    //         [0.0, 0.0], // top left
    //         [1.0, 0.0], // top right
    //         // top
    //         [0.0, 1.0], // bottom left
    //         [1.0, 1.0], // bottom right
    //         [0.0, 0.0], // top left
    //         [1.0, 0.0], // top right
    //     ];
    //
    //     const TANGENTS: [[f32; 4]; 24] = [
    //         // front
    //         [1.0, 0.0, 0.0, 1.0], // bottom left
    //         [1.0, 0.0, 0.0, 1.0], // bottom right
    //         [1.0, 0.0, 0.0, 1.0], // top left
    //         [1.0, 0.0, 0.0, 1.0], // top right
    //         // back
    //         [1.0, 0.0, 0.0, 1.0], // bottom left
    //         [1.0, 0.0, 0.0, 1.0], // bottom right
    //         [1.0, 0.0, 0.0, 1.0], // top left
    //         [1.0, 0.0, 0.0, 1.0], // top right
    //         // left
    //         [0.0, 0.0, 1.0, 1.0], // bottom left
    //         [0.0, 0.0, 1.0, 1.0], // bottom right
    //         [0.0, 0.0, 1.0, 1.0], // top left
    //         [0.0, 0.0, 1.0, 1.0], // top right
    //         // right
    //         [0.0, 0.0, -1.0, 1.0], // bottom left
    //         [0.0, 0.0, -1.0, 1.0], // bottom right
    //         [0.0, 0.0, -1.0, 1.0], // top left
    //         [0.0, 0.0, -1.0, 1.0], // top right
    //         // bottom
    //         [1.0, 0.0, 0.0, 1.0], // bottom left
    //         [1.0, 0.0, 0.0, 1.0], // bottom right
    //         [1.0, 0.0, 0.0, 1.0], // top left
    //         [1.0, 0.0, 0.0, 1.0], // top right
    //         // top
    //         [1.0, 0.0, 0.0, 1.0], // bottom left
    //         [1.0, 0.0, 0.0, 1.0], // bottom right
    //         [1.0, 0.0, 0.0, 1.0], // top left
    //         [1.0, 0.0, 0.0, 1.0], // top right
    //     ];
    //
    //     const INDICES: [u32; 36] = [
    //         // front
    //         0, 1, 3, //
    //         0, 3, 2, //
    //         // back
    //         4, 5, 7, //
    //         4, 7, 6, //
    //         // left
    //         8, 9, 11, //
    //         8, 11, 10, //
    //         // right
    //         12, 13, 15, //
    //         12, 15, 14, //
    //         // bottom
    //         16, 17, 19, //
    //         16, 19, 18, //
    //         // top
    //         20, 21, 23, //
    //         20, 23, 22, //
    //     ];
    //
    //     const VERTS: usize = 24;
    //     const INDS: usize = 36;
    //     for i in 0..VERTS {
    //         self.vertices.push(render::VertexFull {
    //             position: POSITIONS[i],
    //             color: [1.0, 1.0, 1.0, 1.0],
    //             normal: NORMALS[i],
    //             uv: UVS[i],
    //             tangent: TANGENTS[i],
    //         });
    //     }
    //
    //     let start_index = self.vertices.len() - VERTS;
    //     for i in 0..INDS {
    //         self.indices.push(start_index as u32 + INDICES[i]);
    //     }
    //
    //     self
    // }

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

    pub fn build(self) -> crate::Mesh {
        let mut mesh = crate::Mesh::new(wgpu::PrimitiveTopology::TriangleList);
        mesh.add_attribute(
            VertexAttributeId::Position,
            VertexAttributeValues::Float32x3(self.positions),
        );
        mesh.add_attribute(
            VertexAttributeId::Normal,
            VertexAttributeValues::Float32x3(self.normals),
        );
        mesh.add_attribute(
            VertexAttributeId::Uv(0),
            VertexAttributeValues::Float32x2(self.uvs),
        );
        mesh.add_attribute(
            VertexAttributeId::Tangent,
            VertexAttributeValues::Float32x4(self.tangents),
        );
        mesh.set_indices(self.indices);

        mesh
    }
}
