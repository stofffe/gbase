use crate::{
    render::{self, ArcBuffer},
    Context,
};
use std::{marker::PhantomData, ops::RangeBounds};

pub trait VertexTrait: bytemuck::Pod + bytemuck::Zeroable {
    fn desc() -> wgpu::VertexBufferLayout<'static>;
}
use wgpu::util::DeviceExt;

//
// Vertex Buffer
//

pub enum VertexBufferSource<T: VertexTrait> {
    Data(Vec<T>),
    Empty(u64),
}

pub struct VertexBufferBuilder<T: VertexTrait> {
    source: VertexBufferSource<T>,
    // data: Vec<T>,
    label: Option<String>,
    usage: wgpu::BufferUsages,
}

impl<T: VertexTrait> VertexBufferBuilder<T> {
    pub fn new(source: VertexBufferSource<T>) -> Self {
        Self {
            source,
            // data: data.into(),
            label: None,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        }
    }
    pub fn build(self, ctx: &Context) -> VertexBuffer<T> {
        let device = render::device(ctx);

        match self.source {
            VertexBufferSource::Data(data) => {
                let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: self.label.as_deref(),
                    usage: self.usage,
                    contents: bytemuck::cast_slice(&data),
                });
                VertexBuffer {
                    buffer: ArcBuffer::new(buffer),
                    capacity: data.len(),
                    len: data.len() as u32,
                    ty: PhantomData::<T>,
                }
            }
            VertexBufferSource::Empty(capacity) => {
                let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                    label: self.label.as_deref(),
                    size: capacity * std::mem::size_of::<T>() as u64,
                    usage: self.usage | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
                VertexBuffer {
                    buffer: ArcBuffer::new(buffer),
                    capacity: capacity as usize,
                    len: 0,
                    ty: PhantomData::<T>,
                }
            }
        }
    }
}

impl<T: VertexTrait> VertexBufferBuilder<T> {
    pub fn label(mut self, value: String) -> Self {
        self.label = Some(value);
        self
    }
    pub fn usage(mut self, value: wgpu::BufferUsages) -> Self {
        self.usage = value;
        self
    }
}

pub struct VertexBuffer<T: VertexTrait> {
    buffer: ArcBuffer,
    capacity: usize,
    len: u32,
    ty: PhantomData<T>,
}

impl<T: VertexTrait> VertexBuffer<T> {
    pub fn write(&mut self, ctx: &Context, buffer: &[T]) {
        debug_assert!(
            buffer.len() <= self.capacity,
            "written buffer must be smaller than capacity"
        );

        let queue = render::queue(ctx);
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(buffer));
        self.len = buffer.len() as u32;
    }
    pub fn write_offset(&mut self, ctx: &Context, offset: u64, data: &[T]) {
        let queue = render::queue(ctx);
        queue.write_buffer(&self.buffer, offset, bytemuck::cast_slice(data));
        self.len = data.len() as u32;
    }

    pub fn desc(&self) -> wgpu::VertexBufferLayout<'static> {
        T::desc()
    }

    pub fn buf_ref(&self) -> &wgpu::Buffer {
        &self.buffer
    }
    pub fn buf(&self) -> ArcBuffer {
        self.buffer.clone()
    }

    pub fn slice(&self, bounds: impl RangeBounds<wgpu::BufferAddress>) -> wgpu::BufferSlice<'_> {
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
// Index Buffer
//
pub enum IndexBufferSource {
    Data(Vec<u32>),
    Empty(u64),
}

pub struct IndexBufferBuilder {
    source: IndexBufferSource,
    label: Option<String>,
    usage: wgpu::BufferUsages,
}

impl IndexBufferBuilder {
    #[allow(clippy::new_without_default)]
    pub fn new(source: IndexBufferSource) -> Self {
        Self {
            source,
            label: None,
            usage: wgpu::BufferUsages::INDEX,
        }
    }
    pub fn build(self, ctx: &Context) -> IndexBuffer {
        let device = render::device(ctx);
        match self.source {
            IndexBufferSource::Data(data) => {
                let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: self.label.as_deref(),
                    contents: bytemuck::cast_slice(&data),
                    usage: self.usage,
                });

                IndexBuffer {
                    buffer,
                    capacity: data.len(),
                    len: data.len() as u32,
                }
            }
            IndexBufferSource::Empty(capacity) => {
                let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                    label: self.label.as_deref(),
                    size: capacity * std::mem::size_of::<u32>() as u64,
                    usage: self.usage | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
                IndexBuffer {
                    buffer,
                    capacity: capacity as usize,
                    len: 0,
                }
            }
        }
    }
    pub fn label(mut self, value: String) -> Self {
        self.label = Some(value);
        self
    }
    pub fn usage(mut self, value: wgpu::BufferUsages) -> Self {
        self.usage = value;
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
    pub fn write(&mut self, ctx: &Context, buffer: &[u32]) {
        debug_assert!(
            buffer.len() <= self.capacity,
            "written buffer must be smaller than capacity"
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

    pub fn slice(&self, bounds: impl RangeBounds<wgpu::BufferAddress>) -> wgpu::BufferSlice<'_> {
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

//pub struct DynamicVertexBufferBuilder<'a, T: VertexTrait> {
//    capacity: usize,
//
//    label: Option<&'a str>,
//    usage: wgpu::BufferUsages,
//    ty: PhantomData<T>,
//}
//
//impl<'a, T: VertexTrait> DynamicVertexBufferBuilder<'a, T> {
//    #[allow(clippy::new_without_default)]
//    pub fn new(capacity: usize) -> Self {
//        Self {
//            label: None,
//            capacity,
//            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
//            ty: PhantomData::<T>,
//        }
//    }
//    pub fn build(self, ctx: &Context) -> DynamicVertexBuffer<T> {
//        let device = render::device(ctx);
//        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
//            label: self.label,
//            size: self.capacity as u64 * std::mem::size_of::<T>() as u64,
//            usage: self.usage,
//            mapped_at_creation: false,
//        });
//
//        DynamicVertexBuffer {
//            buffer,
//            vertices: Vec::with_capacity(self.capacity),
//            capacity: self.capacity,
//            ty: PhantomData::<T>,
//        }
//    }
//
//    // setters
//    pub fn label(mut self, value: &'a str) -> Self {
//        self.label = Some(value);
//        self
//    }
//    pub fn usages(mut self, value: wgpu::BufferUsages) -> Self {
//        self.usage = value;
//        self
//    }
//    pub fn capacity(mut self, value: usize) -> Self {
//        self.capacity = value;
//        self
//    }
//}
//
//pub struct DynamicVertexBuffer<T: VertexTrait> {
//    buffer: wgpu::Buffer,
//    vertices: Vec<T>,
//    capacity: usize,
//    ty: PhantomData<T>,
//}
//
//impl<T: VertexTrait> DynamicVertexBuffer<T> {
//    pub fn desc(&self) -> wgpu::VertexBufferLayout<'static> {
//        T::desc()
//    }
//
//    pub fn clear(&mut self) {
//        self.vertices.clear();
//    }
//
//    pub fn add(&mut self, vertex: T) {
//        self.vertices.push(vertex);
//    }
//
//    pub fn update_buffer(&mut self, ctx: &Context) {
//        debug_assert!(
//            self.vertices.len() <= self.capacity,
//            "vertex buffer must be smaller than capacity"
//        );
//        let queue = render::queue(ctx);
//        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&self.vertices));
//    }
//
//    pub fn buffer(&self) -> &wgpu::Buffer {
//        &self.buffer
//    }
//
//    pub fn slice(&self, bounds: impl RangeBounds<wgpu::BufferAddress>) -> wgpu::BufferSlice<'_> {
//        self.buffer.slice(bounds)
//    }
//
//    #[allow(clippy::len_without_is_empty)]
//    pub fn len(&self) -> u32 {
//        self.vertices.len() as u32
//    }
//
//    pub fn capacity(&self) -> usize {
//        self.capacity
//    }
//}
//
/////
///// Dynamic Index buffer
/////
//
//pub struct DynamicIndexBufferBuilder<'a> {
//    capacity: usize,
//
//    label: Option<&'a str>,
//    usage: wgpu::BufferUsages,
//}
//
//impl<'a> DynamicIndexBufferBuilder<'a> {
//    #[allow(clippy::new_without_default)]
//    pub fn new(capacity: usize) -> Self {
//        Self {
//            capacity,
//            label: None,
//            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
//        }
//    }
//    pub fn build(self, ctx: &Context) -> DynamicIndexBuffer {
//        let device = render::device(ctx);
//        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
//            label: self.label,
//            size: self.capacity as u64 * std::mem::size_of::<u32>() as u64,
//            usage: self.usage,
//            mapped_at_creation: false,
//        });
//
//        DynamicIndexBuffer {
//            buffer,
//            indices: Vec::with_capacity(self.capacity),
//            capacity: self.capacity,
//        }
//    }
//    pub fn label(mut self, value: &'a str) -> Self {
//        self.label = Some(value);
//        self
//    }
//    pub fn usage(mut self, value: wgpu::BufferUsages) -> Self {
//        self.usage = value;
//        self
//    }
//}
//
//pub struct DynamicIndexBuffer {
//    buffer: wgpu::Buffer,
//    indices: Vec<u32>,
//    capacity: usize,
//}
//
//impl DynamicIndexBuffer {
//    pub fn format(&self) -> wgpu::IndexFormat {
//        wgpu::IndexFormat::Uint32
//    }
//
//    pub fn clear(&mut self) {
//        self.indices.clear();
//    }
//
//    pub fn add(&mut self, index: u32) {
//        self.indices.push(index);
//    }
//
//    pub fn update_buffer(&mut self, ctx: &Context) {
//        debug_assert!(
//            self.indices.len() <= self.capacity,
//            "index buffer must be smaller than capacity"
//        );
//        let queue = render::queue(ctx);
//        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&self.indices));
//    }
//
//    pub fn buffer(&self) -> &wgpu::Buffer {
//        &self.buffer
//    }
//
//    pub fn slice(&self, bounds: impl RangeBounds<wgpu::BufferAddress>) -> wgpu::BufferSlice<'_> {
//        self.buffer.slice(bounds)
//    }
//
//    #[allow(clippy::len_without_is_empty)]
//    pub fn len(&self) -> u32 {
//        self.indices.len() as u32
//    }
//
//    pub fn capacity(&self) -> usize {
//        self.capacity
//    }
//}

//
// Vertex implementations
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

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct VertexFull {
    pub position: [f32; 3],
    pub color: [f32; 4],
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    pub tangent: [f32; 4],
}

impl VertexFull {
    const ATTRIBUTES: &'static [wgpu::VertexAttribute] = &wgpu::vertex_attr_array![
        0=>Float32x3, // pos
        1=>Float32x4, // color
        2=>Float32x3, // normal
        3=>Float32x2, // uv
        4=>Float32x4, // tangent
    ];
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: Self::ATTRIBUTES,
        }
    }
}

impl VertexTrait for VertexFull {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        Self::desc()
    }
}

// pub fn build(self, ctx: &Context, size: impl Into<u64>) -> VertexBuffer<T> {
//     let device = render::device(ctx);
//     let size = size.into();
//     let buffer = device.create_buffer(&wgpu::BufferDescriptor {
//         label: self.label,
//         size,
//         usage: self.usage,
//         mapped_at_creation: false,
//     });
//
//     VertexBuffer {
//         buffer,
//         capacity: size as usize,
//         len: 0,
//         ty: PhantomData::<T>,
//     }
// }

// pub fn build_init(self, ctx: &Context, data: &[impl bytemuck::NoUninit]) -> VertexBuffer<T> {
//     let device = render::device(ctx);
//     let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
//         label: self.label,
//         usage: self.usage,
//         contents: bytemuck::cast_slice(data),
//     });
//     VertexBuffer {
//         buffer,
//         capacity: data.len(),
//         len: data.len() as u32,
//         ty: PhantomData::<T>,
//     }
// }
