use crate::{
    glam::Vec3,
    render::{self, VertexBufferLayout},
    wgpu, Context,
};
use std::collections::{BTreeMap, BTreeSet};

//
// CPU
//

#[derive(Debug, Clone, Default)]
pub struct Mesh {
    primitive_topology: wgpu::PrimitiveTopology,
    attributes: BTreeMap<VertexAttributeId, VertexAttributeValues>,
    indices: Option<Vec<u32>>,
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

    pub fn get_indices(&self) -> Option<&Vec<u32>> {
        self.indices.as_ref()
    }

    pub fn remove_indices(&mut self) {
        self.indices = None;
    }

    pub fn set_attribute(&mut self, id: VertexAttributeId, values: VertexAttributeValues) {
        if let Some(vertex_count) = self.vertex_count() {
            if vertex_count != values.len() as u32 {
                tracing::warn!("inserting attribute with different vertex count");
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

    pub fn remove_all_attributes(&mut self) {
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

    pub fn extract_attributes(
        mut self,
        attributes: impl Into<BTreeSet<VertexAttributeId>>,
    ) -> Self {
        let attributes = attributes.into();
        // remove
        self.attributes = self
            .clone()
            .attributes
            .into_iter()
            .filter(|(id, _)| attributes.contains(id))
            .collect::<BTreeMap<VertexAttributeId, VertexAttributeValues>>();

        // add
        for attr in attributes {
            if !self.attributes.contains_key(&attr) {
                match attr {
                    VertexAttributeId::Normal => {
                        tracing::warn!(
                            "normal attribute could not be found, generating for each vertex"
                        );
                        self.generate_normals();
                    }
                    VertexAttributeId::Tangent => {
                        tracing::warn!(
                            "tangent attribute could not be found, generating for each vertex"
                        );
                        self.generate_tangents();
                    }
                    VertexAttributeId::Color(i) => {
                        tracing::warn!(
                        "color attribute could not be found, generating [1,1,1] for each vertex"
                    );
                        self.generate_colors(i, [1.0, 1.0, 1.0]);
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
            tracing::error!("trying to generate colors for mesh without vertices");
            return;
        };

        let mut colors = Vec::with_capacity(count as usize);
        for _ in 0..count {
            colors.push(color);
        }

        self.set_attribute(
            VertexAttributeId::Color(color_index),
            VertexAttributeValues::Float32x3(colors),
        );
    }

    /// panics if no position attribute
    pub fn calculate_bounding_box(&self) -> BoundingBox {
        let positions = self
            .get_attribute(VertexAttributeId::Position)
            .expect("position attribute needed for calculating bounding box");

        let VertexAttributeValues::Float32x3(positions) = positions else {
            panic!("positions must be float32x3")
        };

        let mut bounding_box = BoundingBox {
            min: Vec3::ONE * f32::MAX,
            max: Vec3::ONE * f32::MIN,
        };

        for pos in positions.iter() {
            bounding_box.min.x = bounding_box.min.x.min(pos[0]);
            bounding_box.min.y = bounding_box.min.y.min(pos[1]);
            bounding_box.min.z = bounding_box.min.z.min(pos[2]);

            bounding_box.max.x = bounding_box.max.x.max(pos[0]);
            bounding_box.max.y = bounding_box.max.y.max(pos[1]);
            bounding_box.max.z = bounding_box.max.z.max(pos[2]);
        }

        bounding_box
    }

    pub fn buffer_layout(&self) -> Vec<VertexBufferLayout> {
        let mut buffers = Vec::new();
        for (attr, _) in self.attributes.iter() {
            buffers.push(render::VertexBufferLayout::from_vertex_formats(
                wgpu::VertexStepMode::Vertex,
                vec![attr.format()],
            ));
        }
        buffers
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

    pub fn as_type<T: bytemuck::Pod>(&self) -> &[T] {
        bytemuck::cast_slice::<u8, T>(self.as_bytes())
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

// TODO: temp
#[derive(Debug, Clone)]
pub struct BoundingBox {
    pub min: Vec3,
    pub max: Vec3,
}

impl BoundingBox {
    // pub fn bounding_radius(&self) -> f32 {
    //     f32::max(self.min.length(), self.max.length())
    //     // let center = (self.min + self.max) * 0.5;
    //     // (self.max - center).length()
    // }
}

//
// GPU
//

#[derive(Clone)]
pub struct GpuMesh {
    pub attribute_buffer: render::ArcBuffer,
    pub attribute_ranges: BTreeMap<VertexAttributeId, (u64, u64)>,
    pub index_buffer: Option<render::ArcBuffer>,
    pub vertex_count: u32,
    pub index_count: Option<u32>,
    pub bounds: BoundingBox,
}

impl GpuMesh {
    pub fn new(ctx: &Context, mesh: &Mesh) -> Self {
        // layout attributes sequentially in the buffer
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

        let bounds = mesh.calculate_bounding_box();

        Self {
            attribute_buffer: buffer.buffer(),
            attribute_ranges,
            index_buffer,
            vertex_count,
            index_count,
            bounds,
        }
    }

    pub fn bind_to_render_pass(&self, render_pass: &mut wgpu::RenderPass<'_>) {
        for (i, (_, (start, end))) in self.attribute_ranges.iter().enumerate() {
            let slice = self.attribute_buffer.slice(start..end);
            render_pass.set_vertex_buffer(i as u32, slice);
        }
        if let Some(indices) = &self.index_buffer {
            render_pass.set_index_buffer(indices.as_ref().slice(..), wgpu::IndexFormat::Uint32);
        }
    }

    /// Draw with base vertex 0 and 1 instance
    pub fn draw_in_render_pass(&self, render_pass: &mut wgpu::RenderPass<'_>) {
        match self.index_count {
            Some(count) => {
                render_pass.draw_indexed(0..count, 0, 0..1);
            }
            None => {
                render_pass.draw(0..self.vertex_count, 0..1);
            }
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
    pub fn fullscreen_quad() -> Self {
        Self {
            positions: vec![
                [-1.0, -1.0, 0.0], // bottom left
                [1.0, -1.0, 0.0],  // bottom right
                [-1.0, 1.0, 0.0],  // top left
                [1.0, 1.0, 0.0],   // top right
            ],
            uvs: vec![
                [0.0, 1.0], // bottom left
                [1.0, 1.0], // bottom right
                [0.0, 0.0], // top left
                [1.0, 0.0], // top right
            ],
            normals: vec![
                [0.0, 0.0, 1.0],
                [0.0, 0.0, 1.0],
                [0.0, 0.0, 1.0],
                [0.0, 0.0, 1.0],
            ],
            tangents: vec![
                [1.0, 0.0, 0.0, 1.0],
                [1.0, 0.0, 0.0, 1.0],
                [1.0, 0.0, 0.0, 1.0],
                [1.0, 0.0, 0.0, 1.0],
            ],
            indices: vec![
                0, 1, 3, //
                0, 3, 2, //
            ],
        }
    }
    pub fn quad() -> Self {
        Self {
            positions: vec![
                [-0.5, -0.5, 0.0], // bottom left
                [0.5, -0.5, 0.0],  // bottom right
                [-0.5, 0.5, 0.0],  // top left
                [0.5, 0.5, 0.0],   // top right
            ],
            uvs: vec![
                [0.0, 1.0], // bottom left
                [1.0, 1.0], // bottom right
                [0.0, 0.0], // top left
                [1.0, 0.0], // top right
            ],
            normals: vec![
                [0.0, 0.0, 1.0],
                [0.0, 0.0, 1.0],
                [0.0, 0.0, 1.0],
                [0.0, 0.0, 1.0],
            ],
            tangents: vec![
                [1.0, 0.0, 0.0, 1.0],
                [1.0, 0.0, 0.0, 1.0],
                [1.0, 0.0, 0.0, 1.0],
                [1.0, 0.0, 0.0, 1.0],
            ],
            indices: vec![
                0, 1, 3, //
                0, 3, 2, //
            ],
        }
    }

    pub fn build(self) -> Mesh {
        let mut mesh = Mesh::new(wgpu::PrimitiveTopology::TriangleList);
        mesh.set_attribute(
            VertexAttributeId::Position,
            VertexAttributeValues::Float32x3(self.positions),
        );
        mesh.set_attribute(
            VertexAttributeId::Normal,
            VertexAttributeValues::Float32x3(self.normals),
        );
        mesh.set_attribute(
            VertexAttributeId::Uv(0),
            VertexAttributeValues::Float32x2(self.uvs),
        );
        mesh.set_attribute(
            VertexAttributeId::Tangent,
            VertexAttributeValues::Float32x4(self.tangents),
        );
        mesh.set_indices(self.indices);

        mesh
    }
}

// const POSITIONS: [[f32; 3]; 4] = [
//     [-0.5, -0.5, 0.0], // bottom left
//     [0.5, -0.5, 0.0],  // bottom right
//     [-0.5, 0.5, 0.0],  // top left
//     [0.5, 0.5, 0.0],   // top right
// ];
// const NORMALS: [[f32; 3]; 4] = [
//     [0.0, 0.0, 1.0],
//     [0.0, 0.0, 1.0],
//     [0.0, 0.0, 1.0],
//     [0.0, 0.0, 1.0],
// ];
// const UVS: [[f32; 2]; 4] = [
//     [0.0, 1.0], // bottom left
//     [1.0, 1.0], // bottom right
//     [0.0, 0.0], // top left
//     [1.0, 0.0], // top right
// ];
// const TANGENTS: [[f32; 4]; 4] = [
//     [1.0, 0.0, 0.0, 1.0],
//     [1.0, 0.0, 0.0, 1.0],
//     [1.0, 0.0, 0.0, 1.0],
//     [1.0, 0.0, 0.0, 1.0],
// ];
// const INDICES: [u32; 6] = [
//     0, 1, 3, //
//     0, 3, 2, //
// ];
//
// let mut positions = Vec::new();
// let mut normals = Vec::new();
// let mut uvs = Vec::new();
// let mut tangents = Vec::new();
// let mut indices = Vec::new();
//
// let ind_offset = positions.len() as u32;
//
// for pos in POSITIONS {
//     positions.push(pos);
// }
// for normal in NORMALS {
//     normals.push(normal);
// }
// for uv in UVS {
//     uvs.push(uv);
// }
// for tangent in TANGENTS {
//     tangents.push(tangent);
// }
// for ind in INDICES {
//     indices.push(ind + ind_offset);
// }
//
// Self {
//     positions,
//     uvs,
//     normals,
//     tangents,
//     indices,
// }
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
