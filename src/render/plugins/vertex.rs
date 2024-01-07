use crate::{render, Context};
use std::{marker::PhantomData, ops::RangeBounds};
use wgpu::util::DeviceExt;

pub trait VertexTrait: bytemuck::Pod + bytemuck::Zeroable {
    fn desc() -> wgpu::VertexBufferLayout<'static>;
}

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
        //self.vertices.append(&mut value.to_vec()); append
        self.vertices = value.to_vec();
        self
    }
}

pub struct VertexBuffer<T: VertexTrait> {
    buffer: wgpu::Buffer,
    len: u32,
    ty: PhantomData<T>,
}

impl<T: VertexTrait> VertexBuffer<T> {
    // TODO add label?
    // Old
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

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
}

impl Vertex {
    const ATTRIBUTES: [wgpu::VertexAttribute; 1] = wgpu::vertex_attr_array![
        0=>Float32x3,
    ];
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

impl VertexTrait for Vertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        Vertex::desc()
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct VertexUV {
    pub position: [f32; 3],
    pub uv: [f32; 2],
}

impl VertexUV {
    const ATTRIBUTES: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![
        0=>Float32x3,
        1=>Float32x2,
    ];
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

impl VertexTrait for VertexUV {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        VertexUV::desc()
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct VertexColor {
    pub position: [f32; 3],
    pub color: [f32; 3],
}

impl VertexColor {
    const ATTRIBUTES: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![
        0=>Float32x3,
        1=>Float32x3,
    ];
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBUTES,
        }
    }
}

impl VertexTrait for VertexColor {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        VertexColor::desc()
    }
}
