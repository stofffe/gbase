use crate::{
    render::{self, ArcBuffer},
    Context,
};
use std::{marker::PhantomData, ops::RangeBounds};
use wgpu::util::DeviceExt;

pub trait VertexTrait: bytemuck::Pod + bytemuck::Zeroable {
    fn desc() -> render::VertexBufferLayout;
}

//
// Vertex Buffer
//

pub enum VertexBufferSource<T: VertexTrait> {
    Data(Vec<T>),
    Size(u64),
}

pub struct VertexBufferBuilder<T: VertexTrait> {
    source: VertexBufferSource<T>,
    label: Option<String>,
    usage: wgpu::BufferUsages,
}

impl<T: VertexTrait> VertexBufferBuilder<T> {
    pub fn new(source: VertexBufferSource<T>) -> Self {
        Self {
            source,
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
            VertexBufferSource::Size(capacity) => {
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

    pub fn desc(&self) -> render::VertexBufferLayout {
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

//
// Vertex implementations
//

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
}

impl VertexTrait for Vertex {
    fn desc() -> render::VertexBufferLayout {
        render::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: wgpu::vertex_attr_array![
                0=>Float32x3,
            ]
            .to_vec(),
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct VertexUV {
    pub position: [f32; 3],
    pub uv: [f32; 2],
}

impl VertexTrait for VertexUV {
    fn desc() -> render::VertexBufferLayout {
        render::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: wgpu::vertex_attr_array![
                0=>Float32x3,
                1=>Float32x2,
            ]
            .to_vec(),
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct VertexColor {
    pub position: [f32; 3],
    pub color: [f32; 3],
}

impl VertexTrait for VertexColor {
    fn desc() -> render::VertexBufferLayout {
        render::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: wgpu::vertex_attr_array![
                0=>Float32x3,
                1=>Float32x3,
            ]
            .to_vec(),
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct VertexColorUV {
    pub position: [f32; 3],
    pub color: [f32; 3],
    pub uv: [f32; 2],
}

impl VertexTrait for VertexColorUV {
    fn desc() -> render::VertexBufferLayout {
        render::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: wgpu::vertex_attr_array![
                0=>Float32x3,
                1=>Float32x3,
                2=>Float32x2,
            ]
            .to_vec(),
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct VertexNormal {
    pub position: [f32; 3],
    pub normal: [f32; 3],
}

impl VertexTrait for VertexNormal {
    fn desc() -> render::VertexBufferLayout {
        render::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: wgpu::vertex_attr_array![
                0=>Float32x3,
                1=>Float32x3
            ]
            .to_vec(),
        }
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

impl VertexTrait for VertexFull {
    fn desc() -> render::VertexBufferLayout {
        render::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: wgpu::vertex_attr_array![
                0=>Float32x3, // pos
                1=>Float32x4, // color
                2=>Float32x3, // normal
                3=>Float32x2, // uv
                4=>Float32x4, // tangent
            ]
            .to_vec(),
        }
    }
}
