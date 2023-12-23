use std::marker::PhantomData;
use wgpu::util::DeviceExt;

pub trait VertexTrait: bytemuck::Pod + bytemuck::Zeroable {
    fn desc() -> wgpu::VertexBufferLayout<'static>;
}

pub struct VertexBuffer<T: VertexTrait> {
    pub buffer: wgpu::Buffer,
    ty: PhantomData<T>,
}

impl<T: VertexTrait> VertexBuffer<T> {
    // TODO add label?
    pub fn new(device: &wgpu::Device, vertices: &[T]) -> Self {
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        Self {
            buffer,
            ty: PhantomData::<T>,
        }
    }

    pub fn desc(&self) -> wgpu::VertexBufferLayout<'static> {
        T::desc()
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
