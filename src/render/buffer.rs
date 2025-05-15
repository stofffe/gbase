use crate::{render, Context};
use bytemuck::NoUninit;
use encase::{internal::WriteInto, ShaderType};
use render::ArcBuffer;
use std::{marker::PhantomData, ops::RangeBounds};
use wgpu::util::DeviceExt;

//
// Raw Buffer
//

pub enum RawBufferSource<T: bytemuck::NoUninit> {
    Size(u64),
    Data(Vec<T>),
}

// TODO: add type to this
pub struct RawBufferBuilder<T: bytemuck::NoUninit> {
    source: RawBufferSource<T>,
    label: Option<String>,
    usage: wgpu::BufferUsages,
}

impl<T: NoUninit> RawBufferBuilder<T> {
    pub fn new(source: RawBufferSource<T>) -> Self {
        Self {
            source,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            label: None,
        }
    }

    pub fn build(self, ctx: &Context) -> RawBuffer<T> {
        let device = render::device(ctx);
        let buffer = match self.source {
            RawBufferSource::Size(size) => device.create_buffer(&wgpu::BufferDescriptor {
                label: self.label.as_deref(),
                size,
                usage: self.usage,
                mapped_at_creation: false,
            }),
            RawBufferSource::Data(data) => {
                device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: self.label.as_deref(),
                    usage: self.usage,
                    contents: bytemuck::cast_slice(&data),
                })
            }
        };

        RawBuffer {
            buffer: ArcBuffer::new(buffer),
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

pub enum UniformBufferSource<T: ShaderType + WriteInto> {
    Data(T),
    Empty,
}

pub struct UniformBufferBuilder<T: ShaderType + WriteInto> {
    source: UniformBufferSource<T>,
    label: Option<String>,
    usage: wgpu::BufferUsages,
}

impl<T: ShaderType + WriteInto> UniformBufferBuilder<T> {
    pub fn new(source: UniformBufferSource<T>) -> Self {
        Self {
            source,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            label: None,
        }
    }

    pub fn build(self, ctx: &Context) -> UniformBuffer<T> {
        let device = render::device(ctx);

        match self.source {
            UniformBufferSource::Data(data) => {
                let mut buffer = encase::UniformBuffer::new(Vec::new());
                buffer.write(&data).expect("could not write to buffer");
                let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: self.label.as_deref(),
                    usage: self.usage,
                    contents: &buffer.into_inner(),
                });

                UniformBuffer {
                    buffer: ArcBuffer::new(buffer),
                    ty: PhantomData,
                }
            }
            UniformBufferSource::Empty => {
                let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                    label: self.label.as_deref(),
                    size: u64::from(T::min_size()),
                    usage: self.usage,
                    mapped_at_creation: false,
                });

                UniformBuffer {
                    buffer: ArcBuffer::new(buffer),
                    ty: PhantomData,
                }
            }
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

/// DEBUG
///
/// Reads a mapped buffer
///
/// Panics if buffer is not mapped
pub fn read_buffer_sync<T: bytemuck::AnyBitPattern>(
    ctx: &Context,
    buffer: &wgpu::Buffer,
) -> Vec<T> {
    debug_assert!(buffer.usage().contains(wgpu::BufferUsages::MAP_READ));

    let device = render::device(ctx);

    let buffer_slice = buffer.slice(..);
    let (sc, rc) = std::sync::mpsc::channel();
    buffer_slice.map_async(wgpu::MapMode::Read, move |res| {
        sc.send(res).unwrap();
    });
    device.poll(wgpu::MaintainBase::Wait);
    let _ = rc.recv().unwrap();
    let data = buffer_slice.get_mapped_range();
    let result: Vec<T> = bytemuck::cast_slice(&data).to_vec();
    drop(data);
    buffer.unmap();
    result
}
