use crate::{
    render::{self, next_id},
    Context,
};
use bytemuck::NoUninit;
use encase::{internal::WriteInto, ShaderType};
use render::ArcBuffer;
use std::{marker::PhantomData, ops::RangeBounds};

//
// Raw Buffer
//

pub struct RawBufferBuilder<T: bytemuck::NoUninit> {
    size: u64,
    label: Option<String>,
    usage: wgpu::BufferUsages,
    ty: PhantomData<T>,
}

impl<T: NoUninit> RawBufferBuilder<T> {
    pub fn new(size: u64) -> Self {
        Self {
            size,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            label: None,
            ty: PhantomData,
        }
    }

    pub fn build(self, ctx: &mut Context) -> RawBuffer<T> {
        let device = render::device(ctx);
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: self.label.as_deref(),
            size: self.size,
            usage: self.usage,
            mapped_at_creation: false,
        });

        RawBuffer {
            buffer: ArcBuffer::new(next_id(ctx), buffer),
            ty: PhantomData::<T>,
        }
    }
}

impl<T: NoUninit> RawBufferBuilder<T> {
    pub fn label(mut self, value: impl Into<String>) -> Self {
        self.label = Some(value.into());
        self
    }
    pub fn usage(mut self, value: wgpu::BufferUsages) -> Self {
        self.usage = value;
        self
    }
}

/// Buffer for storing data without any alignment
pub struct RawBuffer<T: bytemuck::NoUninit> {
    buffer: ArcBuffer,
    ty: PhantomData<T>,
}

impl<T: bytemuck::NoUninit> RawBuffer<T> {
    pub fn write(&self, ctx: &Context, buffer: &[impl bytemuck::NoUninit]) {
        render::queue(ctx).write_buffer(&self.buffer, 0, bytemuck::cast_slice(buffer));
    }
    pub fn write_offset(&self, ctx: &Context, offset: u64, buffer: &[impl bytemuck::NoUninit]) {
        render::queue(ctx).write_buffer(&self.buffer, offset, bytemuck::cast_slice(buffer));
    }
}

impl<T: bytemuck::NoUninit> RawBuffer<T> {
    pub fn buffer(&self) -> ArcBuffer {
        self.buffer.clone()
    }
    pub fn buffer_ref(&self) -> &wgpu::Buffer {
        &self.buffer
    }
    pub fn slice(&self, bounds: impl RangeBounds<wgpu::BufferAddress>) -> wgpu::BufferSlice<'_> {
        self.buffer.slice(bounds)
    }
    // TODO: mapped read?
}

//
// Uniform buffer
//

pub struct UniformBufferBuilder<T: ShaderType + WriteInto> {
    label: Option<String>,
    usage: wgpu::BufferUsages,
    ty: PhantomData<T>,
}

impl<T: ShaderType + WriteInto> UniformBufferBuilder<T> {
    pub fn new() -> Self {
        Self {
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            label: None,
            ty: PhantomData,
        }
    }

    pub fn build(self, ctx: &mut Context) -> UniformBuffer<T> {
        let device = render::device(ctx);

        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: self.label.as_deref(),
            size: u64::from(T::min_size()),
            usage: self.usage,
            mapped_at_creation: false,
        });

        UniformBuffer {
            buffer: ArcBuffer::new(next_id(ctx), buffer),
            ty: PhantomData,
        }
    }
}

impl<T: ShaderType + WriteInto> UniformBufferBuilder<T> {
    pub fn label(mut self, value: impl Into<String>) -> Self {
        self.label = Some(value.into());
        self
    }
    pub fn usage(mut self, value: wgpu::BufferUsages) -> Self {
        self.usage = value;
        self
    }
}

/// Buffer for storing uniform buffers
#[derive(Debug, Clone)]
pub struct UniformBuffer<T: ShaderType + WriteInto> {
    buffer: ArcBuffer,
    ty: PhantomData<T>,
}

impl<T: ShaderType + WriteInto> UniformBuffer<T> {
    pub fn write(&self, ctx: &Context, uniform: &T) {
        let mut buffer = encase::UniformBuffer::new(Vec::new());
        buffer
            .write(&uniform)
            .expect("could not write to transform buffer");
        render::queue(ctx).write_buffer(&self.buffer, 0, &buffer.into_inner());
    }

    pub fn buffer(&self) -> ArcBuffer {
        self.buffer.clone()
    }
    pub fn buffer_ref(&self) -> &wgpu::Buffer {
        &self.buffer
    }
    pub fn slice(&self, bounds: impl RangeBounds<wgpu::BufferAddress>) -> wgpu::BufferSlice<'_> {
        self.buffer.slice(bounds)
    }
}

//
// Storage buffer
//

pub struct StorageBufferBuilder<T: ShaderType + WriteInto> {
    size: u64,
    label: Option<String>,
    usage: wgpu::BufferUsages,
    ty: PhantomData<T>,
}

impl<T: ShaderType + WriteInto> StorageBufferBuilder<T> {
    pub fn new(size: u64) -> Self {
        Self {
            size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            label: None,
            ty: PhantomData,
        }
    }

    pub fn build(self, ctx: &mut Context) -> StorageBuffer<T> {
        let device = render::device(ctx);

        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: self.label.as_deref(),
            size: self.size,
            usage: self.usage,
            mapped_at_creation: false,
        });

        StorageBuffer {
            buffer: ArcBuffer::new(next_id(ctx), buffer),
            ty: PhantomData,
        }
    }
}

impl<T: ShaderType + WriteInto> StorageBufferBuilder<T> {
    pub fn label(mut self, value: impl Into<String>) -> Self {
        self.label = Some(value.into());
        self
    }
    pub fn usage(mut self, value: wgpu::BufferUsages) -> Self {
        self.usage = value;
        self
    }
}

#[derive(Debug)]
pub struct StorageBuffer<T: ShaderType + WriteInto> {
    buffer: ArcBuffer,
    ty: PhantomData<T>,
}

impl<T: ShaderType + WriteInto> StorageBuffer<T> {
    pub fn write(&self, ctx: &Context, uniform: &T) {
        let mut buffer = encase::StorageBuffer::new(Vec::new());
        buffer
            .write(&uniform)
            .expect("could not write to transform buffer");
        render::queue(ctx).write_buffer(&self.buffer, 0, &buffer.into_inner());
    }

    pub fn buffer(&self) -> ArcBuffer {
        self.buffer.clone()
    }
    pub fn buffer_ref(&self) -> &wgpu::Buffer {
        &self.buffer
    }
    pub fn slice(&self, bounds: impl RangeBounds<wgpu::BufferAddress>) -> wgpu::BufferSlice<'_> {
        self.buffer.slice(bounds)
    }
}

/// DEBUG
///
/// Reads a mapped buffer
///
/// Panics if buffer is not mapped
pub fn read_buffer_sync<T: bytemuck::AnyBitPattern>(
    device: &wgpu::Device,
    buffer: &wgpu::Buffer,
    offset: u64,
    size: u64,
) -> Vec<T> {
    debug_assert!(buffer.usage().contains(wgpu::BufferUsages::MAP_READ));
    let buffer_slice = buffer.slice(offset..offset + size);
    buffer_slice.map_async(wgpu::MapMode::Read, |_| {});
    device
        .poll(wgpu::MaintainBase::Wait)
        .expect("could not poll");
    let data = buffer_slice.get_mapped_range();
    let result: Vec<T> = bytemuck::cast_slice(&data).to_vec();
    drop(data);
    buffer.unmap();
    result
}

// pub fn read_buffer_async<T: bytemuck::AnyBitPattern>(
//     device: &wgpu::Device,
//     buffer: &wgpu::Buffer,
//     offset: u64,
//     size: u64,
// ) -> Vec<T> {
//     debug_assert!(buffer.usage().contains(wgpu::BufferUsages::MAP_READ));
//     let buffer_slice = buffer.slice(offset..offset + size);
//     buffer_slice.map_async(wgpu::MapMode::Read, |_| {});
//     device
//         .poll(wgpu::MaintainBase::Wait)
//         .expect("could not poll");
//     let data = buffer_slice.get_mapped_range();
//     let result: Vec<T> = bytemuck::cast_slice(&data).to_vec();
//     drop(data);
//     buffer.unmap();
//     result
// }
