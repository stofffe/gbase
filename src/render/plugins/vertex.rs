use crate::{render, Context};
use std::{marker::PhantomData, ops::RangeBounds};
use wgpu::util::DeviceExt;

pub trait VertexTrait: bytemuck::Pod + bytemuck::Zeroable {
    fn desc() -> wgpu::VertexBufferLayout<'static>;
}

pub enum BufferSource<'a, T> {
    Capacity(usize),
    Values(&'a [T]),
}

//
// Vertex buffer
//

pub struct VertexBufferBuilder<'a, T: VertexTrait> {
    source: BufferSource<'a, T>,

    label: Option<String>,
    usage: wgpu::BufferUsages,
}

impl<'a, T: VertexTrait> VertexBufferBuilder<'a, T> {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            label: None,
            source: BufferSource::Capacity(3),
            usage: wgpu::BufferUsages::VERTEX,
        }
    }
    pub fn build(self, ctx: &Context) -> VertexBuffer<T> {
        let device = render::device(ctx);
        let (buffer, capacity, len) = match self.source {
            BufferSource::Capacity(cap) => {
                let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                    label: self.label.as_deref(),
                    size: cap as u64 * std::mem::size_of::<T>() as u64,
                    usage: self.usage,
                    mapped_at_creation: false,
                });
                (buffer, cap, 0)
            }
            BufferSource::Values(vertices) => {
                let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: self.label.as_deref(),
                    contents: bytemuck::cast_slice(vertices),
                    usage: self.usage,
                });
                (buffer, vertices.len(), vertices.len() as u32)
            }
        };

        VertexBuffer {
            buffer,
            len,
            capacity,
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
    pub fn source(mut self, value: BufferSource<'a, T>) -> Self {
        self.source = value;
        self
    }
}

/// A static vertex buffer
pub struct VertexBuffer<T: VertexTrait> {
    buffer: wgpu::Buffer,
    capacity: usize,
    len: u32,
    ty: PhantomData<T>,
}

impl<T: VertexTrait> VertexBuffer<T> {
    pub fn desc(&self) -> wgpu::VertexBufferLayout<'static> {
        T::desc()
    }

    pub fn update_buffer(&mut self, ctx: &Context, buffer: &[T]) {
        debug_assert!(
            buffer.len() <= self.capacity,
            "buffer must be smaller than capacity"
        );
        let queue = render::queue(ctx);
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(buffer));
        self.len = buffer.len() as u32;
    }

    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    pub fn slice(&self, bounds: impl RangeBounds<wgpu::BufferAddress>) -> wgpu::BufferSlice {
        self.buffer.slice(bounds)
    }

    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> u32 {
        self.len
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

//
// Index buffer
//

pub struct IndexBufferBuilder<'a> {
    label: Option<String>,
    source: BufferSource<'a, u32>,
    usage: wgpu::BufferUsages,
}

impl<'a> IndexBufferBuilder<'a> {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            label: None,
            usage: wgpu::BufferUsages::INDEX,
            source: BufferSource::Capacity(3),
        }
    }
    pub fn build(self, ctx: &Context) -> IndexBuffer {
        let device = render::device(ctx);
        let (buffer, capacity, len) = match self.source {
            BufferSource::Capacity(cap) => {
                let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                    label: self.label.as_deref(),
                    size: cap as u64 * std::mem::size_of::<u32>() as u64,
                    usage: self.usage,
                    mapped_at_creation: false,
                });
                (buffer, cap, 0)
            }
            BufferSource::Values(indices) => {
                let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: self.label.as_deref(),
                    contents: bytemuck::cast_slice(indices),
                    usage: self.usage,
                });
                (buffer, indices.len(), indices.len() as u32)
            }
        };

        IndexBuffer {
            buffer,
            len,
            capacity,
        }
    }
    pub fn label(mut self, value: &str) -> Self {
        self.label = Some(value.to_string());
        self
    }
    pub fn usage(mut self, value: wgpu::BufferUsages) -> Self {
        self.usage = value;
        self
    }
    pub fn source(mut self, value: BufferSource<'a, u32>) -> Self {
        self.source = value;
        self
    }
}

/// A static index buffer
pub struct IndexBuffer {
    buffer: wgpu::Buffer,
    len: u32,
    capacity: usize,
}

impl IndexBuffer {
    pub fn update_buffer(&mut self, ctx: &Context, buffer: &[u32]) {
        debug_assert!(
            buffer.len() <= self.capacity,
            "buffer must be smaller than capacity"
        );
        let queue = render::queue(ctx);
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(buffer));
        self.len = buffer.len() as u32;
    }
    pub fn format(&self) -> wgpu::IndexFormat {
        wgpu::IndexFormat::Uint32
    }

    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    pub fn slice(&self, bounds: impl RangeBounds<wgpu::BufferAddress>) -> wgpu::BufferSlice {
        self.buffer.slice(bounds)
    }

    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> u32 {
        self.len
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

///
/// Dynamic vertex buffer
///

pub struct DynamicVertexBufferBuilder<T: VertexTrait> {
    capacity: usize,

    label: Option<String>,
    usage: wgpu::BufferUsages,
    ty: PhantomData<T>,
}

impl<T: VertexTrait> DynamicVertexBufferBuilder<T> {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            label: None,
            capacity: 3,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            ty: PhantomData::<T>,
        }
    }
    pub fn build(self, ctx: &Context) -> DynamicVertexBuffer<T> {
        let device = render::device(ctx);
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: self.label.as_deref(),
            size: self.capacity as u64 * std::mem::size_of::<T>() as u64,
            usage: self.usage,
            mapped_at_creation: false,
        });

        DynamicVertexBuffer {
            buffer,
            vertices: Vec::with_capacity(self.capacity),
            capacity: self.capacity,
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
    pub fn capacity(mut self, value: usize) -> Self {
        self.capacity = value;
        self
    }
}

/// A static vertex buffer
pub struct DynamicVertexBuffer<T: VertexTrait> {
    buffer: wgpu::Buffer,
    vertices: Vec<T>,
    capacity: usize,
    ty: PhantomData<T>,
}

impl<T: VertexTrait> DynamicVertexBuffer<T> {
    pub fn desc(&self) -> wgpu::VertexBufferLayout<'static> {
        T::desc()
    }

    pub fn clear(&mut self) {
        self.vertices.clear();
    }

    pub fn add(&mut self, vertex: T) {
        self.vertices.push(vertex);
    }

    pub fn update_buffer(&mut self, ctx: &Context) {
        debug_assert!(
            self.vertices.len() <= self.capacity,
            "vertex buffer must be smaller than capacity"
        );
        let queue = render::queue(ctx);
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&self.vertices));
    }

    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    pub fn slice(&self, bounds: impl RangeBounds<wgpu::BufferAddress>) -> wgpu::BufferSlice {
        self.buffer.slice(bounds)
    }

    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> u32 {
        self.vertices.len() as u32
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

///
/// Dynamic Index buffer
///

pub struct DynamicIndexBufferBuilder {
    label: Option<String>,
    capacity: usize,
    usage: wgpu::BufferUsages,
}

impl DynamicIndexBufferBuilder {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            label: None,
            capacity: 3,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
        }
    }
    pub fn build(self, ctx: &Context) -> DynamicIndexBuffer {
        let device = render::device(ctx);
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: self.label.as_deref(),
            size: self.capacity as u64 * std::mem::size_of::<u32>() as u64,
            usage: self.usage,
            mapped_at_creation: false,
        });

        DynamicIndexBuffer {
            indices: Vec::with_capacity(self.capacity),
            buffer,
            capacity: self.capacity,
        }
    }
    pub fn label(mut self, value: &str) -> Self {
        self.label = Some(value.to_string());
        self
    }
    pub fn usage(mut self, value: wgpu::BufferUsages) -> Self {
        self.usage = value;
        self
    }
    pub fn capacity(mut self, value: usize) -> Self {
        self.capacity = value;
        self
    }
}

/// A static index buffer
pub struct DynamicIndexBuffer {
    buffer: wgpu::Buffer,
    indices: Vec<u32>,
    capacity: usize,
}

impl DynamicIndexBuffer {
    pub fn format(&self) -> wgpu::IndexFormat {
        wgpu::IndexFormat::Uint32
    }

    pub fn clear(&mut self) {
        self.indices.clear();
    }

    pub fn add(&mut self, index: u32) {
        self.indices.push(index);
    }

    pub fn update_buffer(&mut self, ctx: &Context) {
        debug_assert!(
            self.indices.len() <= self.capacity,
            "index buffer must be smaller than capacity"
        );
        let queue = render::queue(ctx);
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&self.indices));
    }

    pub fn buffer(&self) -> &wgpu::Buffer {
        &self.buffer
    }

    pub fn slice(&self, bounds: impl RangeBounds<wgpu::BufferAddress>) -> wgpu::BufferSlice {
        self.buffer.slice(bounds)
    }

    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> u32 {
        self.indices.len() as u32
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

// //
// // Batch builder
// //
//
// pub struct BatchBufferBuilder<T: VertexTrait> {
//     label: Option<String>,
//     vertices_size: u32,
//     indices_size: u32,
//     ty: PhantomData<T>,
// }
//
// impl<T: VertexTrait> BatchBufferBuilder<T> {
//     #[allow(clippy::new_without_default)]
//     pub fn new() -> Self {
//         Self {
//             label: None,
//             vertices_size: 3,
//             indices_size: 3,
//             ty: PhantomData,
//         }
//     }
//
//     pub fn build(self, ctx: &Context) -> BatchBuffer<T> {
//         let device = render::device(ctx);
//         let vertices_buffer = device.create_buffer(&wgpu::BufferDescriptor {
//             label: self.label.as_deref(),
//             size: self.vertices_size as u64 * std::mem::size_of::<T>() as u64,
//             usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
//             mapped_at_creation: false,
//         });
//         let indices_buffer = device.create_buffer(&wgpu::BufferDescriptor {
//             label: self.label.as_deref(),
//             size: self.indices_size as u64 * std::mem::size_of::<u32>() as u64,
//             usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
//             mapped_at_creation: false,
//         });
//
//         BatchBuffer {
//             vertices: Vec::with_capacity(self.vertices_size as usize),
//             indices: Vec::with_capacity(self.indices_size as usize),
//             vertices_buf: vertices_buffer,
//             indices_buf: indices_buffer,
//         }
//     }
//     pub fn label(mut self, value: &str) -> Self {
//         self.label = Some(value.to_string());
//         self
//     }
//     pub fn vertices_size(mut self, value: u32) -> Self {
//         self.vertices_size = value;
//         self
//     }
//     pub fn indices_size(mut self, value: u32) -> Self {
//         self.indices_size = value;
//         self
//     }
// }
//
// /// A dynamic vertex and index buffer
// pub struct BatchBuffer<T: VertexTrait> {
//     vertices: Vec<T>,
//     indices: Vec<u32>,
//     vertices_buf: wgpu::Buffer,
//     indices_buf: wgpu::Buffer,
// }
//
// impl<T: VertexTrait> BatchBuffer<T> {
//     pub fn vertices_desc(&self) -> wgpu::VertexBufferLayout<'static> {
//         T::desc()
//     }
//     pub fn indices_format(&self) -> wgpu::IndexFormat {
//         wgpu::IndexFormat::Uint32
//     }
//
//     /// Writes full verticies and indices into the buffers
//     pub fn upload_buffers(&mut self, ctx: &Context) {
//         let queue = render::queue(ctx);
//         queue.write_buffer(&self.vertices_buf, 0, bytemuck::cast_slice(&self.vertices));
//         queue.write_buffer(&self.indices_buf, 0, bytemuck::cast_slice(&self.indices));
//     }
//
//     #[inline]
//     pub fn clear(&mut self) {
//         self.vertices.clear();
//         self.indices.clear();
//     }
//
//     #[inline]
//     pub fn add_vertex(&mut self, vertex: T) {
//         self.vertices.push(vertex);
//     }
//     pub fn add_index(&mut self, index: u32) {
//         self.indices.push(index);
//     }
//
//     pub fn vertices_buffer(&self) -> &wgpu::Buffer {
//         &self.vertices_buf
//     }
//     pub fn indices_buffer(&self) -> &wgpu::Buffer {
//         &self.indices_buf
//     }
//
//     pub fn vertices_slice(
//         &self,
//         bounds: impl RangeBounds<wgpu::BufferAddress>,
//     ) -> wgpu::BufferSlice {
//         self.vertices_buf.slice(bounds)
//     }
//     pub fn indices_slice(
//         &self,
//         bounds: impl RangeBounds<wgpu::BufferAddress>,
//     ) -> wgpu::BufferSlice {
//         self.indices_buf.slice(bounds)
//     }
//
//     pub fn vertices_len(&self) -> u32 {
//         self.vertices.len() as u32
//     }
//     pub fn indices_len(&self) -> u32 {
//         self.indices.len() as u32
//     }
// }

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

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct VertexNormal {
    pub position: [f32; 3],
    pub normal: [f32; 3],
}

impl VertexNormal {
    const ATTRIBUTES: &'static [wgpu::VertexAttribute] = &wgpu::vertex_attr_array![
        0=>Float32x3,
        1=>Float32x3
    ];
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: Self::ATTRIBUTES,
        }
    }
}

impl VertexTrait for VertexNormal {
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
            capacity: vertices.len(),
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
