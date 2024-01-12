use crate::{render, Context};
use std::{marker::PhantomData, ops::RangeBounds};
use wgpu::util::DeviceExt;

pub trait VertexTrait: bytemuck::Pod + bytemuck::Zeroable {
    fn desc() -> wgpu::VertexBufferLayout<'static>;
}

//
// Vertex buffer
//

pub struct VertexBufferBuilder<T: VertexTrait> {
    label: Option<String>,
    vertices: Vec<T>,
    usage: wgpu::BufferUsages,
}

impl<T: VertexTrait> VertexBufferBuilder<T> {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            label: None,
            vertices: Vec::new(),
            usage: wgpu::BufferUsages::VERTEX,
        }
    }
    pub fn build(self, ctx: &Context) -> VertexBuffer<T> {
        debug_assert!(
            !self.vertices.is_empty(),
            "debug_assert: vertex buffer \"{}\" can not be empty",
            self.label.unwrap_or_default()
        );

        let device = render::device(ctx);
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: self.label.as_deref(),
            contents: bytemuck::cast_slice(&self.vertices),
            usage: self.usage,
        });
        VertexBuffer {
            buffer,
            len: self.vertices.len() as u32,
            ty: PhantomData::<T>,
        }
    }

    // setters
    pub fn label(mut self, value: &str) -> Self {
        self.label = Some(value.to_string());
        self
    }
    pub fn usages(mut self, value: wgpu::BufferUsages) -> Self {
        self.usage = value;
        self
    }
    pub fn vertices(mut self, value: &[T]) -> Self {
        self.vertices = value.to_vec();
        self
    }
}

/// A static vertex buffer
pub struct VertexBuffer<T: VertexTrait> {
    buffer: wgpu::Buffer,
    len: u32,
    ty: PhantomData<T>,
}

impl<T: VertexTrait> VertexBuffer<T> {
    pub fn desc(&self) -> wgpu::VertexBufferLayout<'static> {
        T::desc()
    }

    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> u32 {
        self.len
    }

    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    pub fn slice(&self, bounds: impl RangeBounds<wgpu::BufferAddress>) -> wgpu::BufferSlice {
        self.buffer.slice(bounds)
    }
}

//
// Index buffer
//

pub struct IndexBufferBuilder {
    label: Option<String>,
    indices: Vec<u32>,
}

impl IndexBufferBuilder {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            label: None,
            indices: Vec::new(),
        }
    }
    pub fn build(self, ctx: &Context) -> IndexBuffer {
        debug_assert!(
            !self.indices.is_empty(),
            "debug_assert: index buffer \"{}\" can not be empty",
            self.label.unwrap_or_default()
        );

        let device = render::device(ctx);
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: self.label.as_deref(),
            contents: bytemuck::cast_slice(&self.indices),
            usage: wgpu::BufferUsages::INDEX,
        });

        IndexBuffer {
            buffer,
            len: self.indices.len() as u32,
        }
    }
    pub fn label(mut self, value: &str) -> Self {
        self.label = Some(value.to_string());
        self
    }
    pub fn indices(mut self, value: &[u32]) -> Self {
        self.indices = value.to_vec();
        self
    }
}

/// A static index buffer
pub struct IndexBuffer {
    buffer: wgpu::Buffer,
    len: u32,
}

impl IndexBuffer {
    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> u32 {
        self.len
    }
    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }
    pub fn slice(&self, bounds: impl RangeBounds<wgpu::BufferAddress>) -> wgpu::BufferSlice {
        self.buffer.slice(bounds)
    }
    pub fn format(&self) -> wgpu::IndexFormat {
        wgpu::IndexFormat::Uint32
    }
}

//
// Batch builder
//

pub struct BatchBufferBuilder<T: VertexTrait> {
    label: Option<String>,
    vertices_size: u32,
    indices_size: u32,
    ty: PhantomData<T>,
}

impl<T: VertexTrait> BatchBufferBuilder<T> {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            label: None,
            vertices_size: 3,
            indices_size: 3,
            ty: PhantomData,
        }
    }

    pub fn build(self, ctx: &Context) -> BatchBuffer<T> {
        let device = render::device(ctx);
        let vertices_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: self.label.as_deref(),
            size: self.vertices_size as u64 * std::mem::size_of::<T>() as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let indices_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: self.label.as_deref(),
            size: self.indices_size as u64 * std::mem::size_of::<u32>() as u64,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        BatchBuffer {
            vertices: Vec::with_capacity(self.vertices_size as usize),
            indices: Vec::with_capacity(self.indices_size as usize),
            vertices_buf: vertices_buffer,
            indices_buf: indices_buffer,
        }
    }
    pub fn label(mut self, value: &str) -> Self {
        self.label = Some(value.to_string());
        self
    }
    pub fn vertices_size(mut self, value: u32) -> Self {
        self.vertices_size = value;
        self
    }
    pub fn indices_size(mut self, value: u32) -> Self {
        self.indices_size = value;
        self
    }
}

/// A dynamic vertex and index buffer
pub struct BatchBuffer<T: VertexTrait> {
    vertices: Vec<T>,
    indices: Vec<u32>,
    vertices_buf: wgpu::Buffer,
    indices_buf: wgpu::Buffer,
}

impl<T: VertexTrait> BatchBuffer<T> {
    pub fn vertices_desc(&self) -> wgpu::VertexBufferLayout<'static> {
        T::desc()
    }
    pub fn indices_format(&self) -> wgpu::IndexFormat {
        wgpu::IndexFormat::Uint32
    }

    /// Writes full verticies and indices into the buffers
    pub fn upload_buffers(&mut self, ctx: &Context) {
        let queue = render::queue(ctx);
        queue.write_buffer(&self.vertices_buf, 0, bytemuck::cast_slice(&self.vertices));
        queue.write_buffer(&self.indices_buf, 0, bytemuck::cast_slice(&self.indices));
    }

    #[inline]
    pub fn clear(&mut self) {
        self.vertices.clear();
        self.indices.clear();
    }

    #[inline]
    pub fn add_vertex(&mut self, vertex: T) {
        self.vertices.push(vertex);
    }
    pub fn add_index(&mut self, index: u32) {
        self.indices.push(index);
    }

    pub fn vertices_buffer(&self) -> &wgpu::Buffer {
        &self.vertices_buf
    }
    pub fn indices_buffer(&self) -> &wgpu::Buffer {
        &self.indices_buf
    }

    pub fn vertices_slice(
        &self,
        bounds: impl RangeBounds<wgpu::BufferAddress>,
    ) -> wgpu::BufferSlice {
        self.vertices_buf.slice(bounds)
    }
    pub fn indices_slice(
        &self,
        bounds: impl RangeBounds<wgpu::BufferAddress>,
    ) -> wgpu::BufferSlice {
        self.indices_buf.slice(bounds)
    }

    pub fn vertices_len(&self) -> u32 {
        self.vertices.len() as u32
    }
    pub fn indices_len(&self) -> u32 {
        self.indices.len() as u32
    }
}

//
// Vertex types
//

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
}

impl Vertex {
    const ATTRIBUTES: &'static [wgpu::VertexAttribute] = &wgpu::vertex_attr_array![
        0=>Float32x3,
    ];
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: Self::ATTRIBUTES,
        }
    }
}

impl VertexTrait for Vertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        Self::desc()
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct VertexUV {
    pub position: [f32; 3],
    pub uv: [f32; 2],
}

impl VertexUV {
    const ATTRIBUTES: &'static [wgpu::VertexAttribute] = &wgpu::vertex_attr_array![
        0=>Float32x3,
        1=>Float32x2,
    ];
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: Self::ATTRIBUTES,
        }
    }
}

impl VertexTrait for VertexUV {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        Self::desc()
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct VertexColor {
    pub position: [f32; 3],
    pub color: [f32; 3],
}

impl VertexColor {
    const ATTRIBUTES: &'static [wgpu::VertexAttribute] = &wgpu::vertex_attr_array![
        0=>Float32x3,
        1=>Float32x3,
    ];
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: Self::ATTRIBUTES,
        }
    }
}

impl VertexTrait for VertexColor {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        Self::desc()
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct VertexColorUV {
    pub position: [f32; 3],
    pub color: [f32; 3],
    pub uv: [f32; 2],
}

impl VertexColorUV {
    const ATTRIBUTES: &'static [wgpu::VertexAttribute] = &wgpu::vertex_attr_array![
        0=>Float32x3,
        1=>Float32x3,
        2=>Float32x2,
    ];
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: Self::ATTRIBUTES,
        }
    }
}

impl VertexTrait for VertexColorUV {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        Self::desc()
    }
}

// Old
impl<T: VertexTrait> VertexBuffer<T> {
    pub fn new(device: &wgpu::Device, vertices: &[T]) -> Self {
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        Self {
            buffer,
            len: vertices.len() as u32,
            ty: PhantomData::<T>,
        }
    }
}
// //
// // Vertex Batch
// //
//
// pub struct VertexBufferBatchBuilder<T: VertexTrait> {
//     label: Option<String>,
//     vertices: Vec<T>,
//     size: u32,
//     usage: wgpu::BufferUsages,
// }
//
// impl<T: VertexTrait> VertexBufferBatchBuilder<T> {
//     #[allow(clippy::new_without_default)]
//     pub fn new() -> Self {
//         Self {
//             label: None,
//             vertices: Vec::new(),
//             usage: wgpu::BufferUsages::VERTEX,
//             size: 3,
//         }
//     }
//     pub fn build(self, ctx: &Context) -> VertexBufferBatch<T> {
//         let device = render::device(ctx);
//         let buffer = device.create_buffer(&wgpu::BufferDescriptor {
//             label: self.label.as_deref(),
//             size: self.size as u64 * std::mem::size_of::<T>() as u64,
//             usage: self.usage,
//             mapped_at_creation: false,
//         });
//         VertexBufferBatch {
//             verticies: self.vertices,
//             buffer,
//         }
//     }
//
//     // setters
//     pub fn label(mut self, value: &str) -> Self {
//         self.label = Some(value.to_string());
//         self
//     }
//     pub fn usages(mut self, value: wgpu::BufferUsages) -> Self {
//         self.usage = value;
//         self
//     }
//     pub fn size(mut self, value: u32) -> Self {
//         self.size = value;
//         self
//     }
// }
//
// pub struct VertexBufferBatch<T: VertexTrait> {
//     verticies: Vec<T>,
//     buffer: wgpu::Buffer,
// }
//
// impl<T: VertexTrait> VertexBufferBatch<T> {
//     /// Writes full verticies vec into the buffer
//     pub fn upload_buffer(&mut self, ctx: &Context) {
//         let queue = render::queue(ctx);
//         queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&self.verticies));
//     }
//
//     #[inline]
//     pub fn clear(&mut self) {
//         self.verticies.clear();
//     }
//
//     #[inline]
//     pub fn add_vertex(&mut self, vertex: T) {
//         self.verticies.push(vertex);
//     }
//
//     pub fn desc(&self) -> wgpu::VertexBufferLayout<'static> {
//         T::desc()
//     }
//
//     #[allow(clippy::len_without_is_empty)]
//     pub fn len(&self) -> u32 {
//         self.verticies.len() as u32
//     }
//
//     pub fn buffer(&self) -> &wgpu::Buffer {
//         &self.buffer
//     }
//
//     pub fn slice(&self, bounds: impl RangeBounds<wgpu::BufferAddress>) -> wgpu::BufferSlice {
//         self.buffer.slice(bounds)
//     }
// }
// //
// // Index batch
// //
//
// pub struct IndexBufferBatchBuilder {
//     label: Option<String>,
//     size: u32,
//     usage: wgpu::BufferUsages,
//     indices: Vec<u32>,
// }
//
// impl IndexBufferBatchBuilder {
//     #[allow(clippy::new_without_default)]
//     pub fn new() -> Self {
//         Self {
//             label: None,
//             indices: Vec::new(),
//             size: 3,
//             usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
//         }
//     }
//     pub fn build(self, ctx: &Context) -> IndexBufferBatch {
//         let device = render::device(ctx);
//         let buffer = device.create_buffer(&wgpu::BufferDescriptor {
//             label: self.label.as_deref(),
//             size: self.size as u64 * std::mem::size_of::<u32>() as u64,
//             usage: self.usage,
//             mapped_at_creation: false,
//         });
//         IndexBufferBatch {
//             indices: self.indices,
//             buffer,
//         }
//     }
//     pub fn label(mut self, value: &str) -> Self {
//         self.label = Some(value.to_string());
//         self
//     }
//     pub fn usage(mut self, value: wgpu::BufferUsages) -> Self {
//         self.usage = value;
//         self
//     }
//     pub fn size(mut self, value: u32) -> Self {
//         self.size = value;
//         self
//     }
// }
//
// pub struct IndexBufferBatch {
//     indices: Vec<u32>,
//     buffer: wgpu::Buffer,
// }
//
// impl IndexBufferBatch {
//     /// Writes full verticies vec into the buffer
//     pub fn upload_buffer(&mut self, ctx: &Context) {
//         let queue = render::queue(ctx);
//         queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&self.indices));
//     }
//
//     pub fn clear(&mut self) {
//         self.indices.clear();
//     }
//
//     pub fn add_index(&mut self, vertex: u32) {
//         self.indices.push(vertex);
//     }
//
//     #[allow(clippy::len_without_is_empty)]
//     pub fn len(&self) -> u32 {
//         self.indices.len() as u32
//     }
//     pub fn buffer(&self) -> &wgpu::Buffer {
//         &self.buffer
//     }
//     pub fn slice(&self, bounds: impl RangeBounds<wgpu::BufferAddress>) -> wgpu::BufferSlice {
//         self.buffer.slice(bounds)
//     }
//     pub fn format(&self) -> wgpu::IndexFormat {
//         wgpu::IndexFormat::Uint32
//     }
// }
