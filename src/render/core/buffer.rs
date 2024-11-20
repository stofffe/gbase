use crate::{render, Context};
use encase::{internal::WriteInto, ShaderType};
use render::ArcBuffer;
use std::{marker::PhantomData, ops::RangeBounds};
use wgpu::util::DeviceExt;

//
// Raw Buffer
//

// TODO: add type to this
pub struct RawBufferBuilder {
    label: Option<String>,
    usage: wgpu::BufferUsages,
}

impl RawBufferBuilder {
    pub fn new() -> Self {
        Self {
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            label: None,
        }
    }

    pub fn build(self, ctx: &Context, size: impl Into<u64>) -> RawBuffer {
        let device = render::device(ctx);
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: self.label.as_deref(),
            size: size.into(),
            usage: self.usage,
            mapped_at_creation: false,
        });

        RawBuffer {
            buffer: ArcBuffer::new(buffer),
        }
    }

    pub fn build_init(self, ctx: &Context, data: &[impl bytemuck::NoUninit]) -> RawBuffer {
        let device = render::device(ctx);

        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: self.label.as_deref(),
            usage: self.usage,
            contents: bytemuck::cast_slice(data),
        });

        RawBuffer {
            buffer: ArcBuffer::new(buffer),
        }
    }
}

impl RawBufferBuilder {
    pub fn label(mut self, value: String) -> Self {
        self.label = Some(value);
        self
    }
    pub fn usage(mut self, value: wgpu::BufferUsages) -> Self {
        self.usage = value;
        self
    }
}

pub struct RawBuffer {
    buffer: ArcBuffer,
}

impl RawBuffer {
    pub fn write(&self, ctx: &Context, buffer: &[impl bytemuck::NoUninit]) {
        render::queue(ctx).write_buffer(&self.buffer, 0, bytemuck::cast_slice(buffer));
    }
    pub fn write_offset(&self, ctx: &Context, offset: u64, buffer: &[impl bytemuck::NoUninit]) {
        render::queue(ctx).write_buffer(&self.buffer, offset, bytemuck::cast_slice(buffer));
    }
}

impl RawBuffer {
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
