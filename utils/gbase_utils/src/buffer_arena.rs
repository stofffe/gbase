// based of GGEZ GrowingBufferArena https://github.com/ggez/ggez

use std::marker::PhantomData;

use encase::{internal::WriteInto, ShaderType};
use gbase::{
    render::{self, ArcBuffer, BindGroupBindable},
    wgpu, Context,
};

fn align_to(alignment: u64, size: u64) -> u64 {
    debug_assert!(alignment.is_power_of_two());
    (size + alignment - 1) & !(alignment - 1)
}

//
// Generic arena
//

#[derive(Debug)]
pub struct GrowingBufferArena {
    buffers: Vec<(ArcBuffer, u64)>,
    alignment: u64,
    desc: wgpu::BufferDescriptor<'static>,
}

impl GrowingBufferArena {
    pub fn new(ctx: &mut Context, alignment: u64, desc: wgpu::BufferDescriptor<'static>) -> Self {
        let buffer = render::device(ctx).create_buffer(&desc);
        GrowingBufferArena {
            buffers: vec![(ArcBuffer::new(ctx, buffer), 0)],
            alignment,
            desc,
        }
    }

    pub fn new_uniform_alignment(ctx: &mut Context, desc: wgpu::BufferDescriptor<'static>) -> Self {
        const UNIFORM_BUFFER_MIN_ALIGNMENT: u64 = 256;

        let buffer = render::device(ctx).create_buffer(&desc);
        GrowingBufferArena {
            buffers: vec![(ArcBuffer::new(ctx, buffer), 0)],
            alignment: UNIFORM_BUFFER_MIN_ALIGNMENT,
            desc,
        }
    }

    pub fn allocate(&mut self, ctx: &mut Context, size: u64) -> BufferArenaAllocation {
        let size = align_to(self.alignment, size);
        debug_assert!(size <= self.desc.size);

        for (buffer, cursor) in &mut self.buffers {
            if size <= self.desc.size - *cursor {
                let offset = *cursor;
                *cursor += size;
                return BufferArenaAllocation {
                    buffer: buffer.clone(),
                    offset,
                    size,
                };
            }
        }

        self.grow(ctx);
        self.allocate(ctx, size)
    }

    pub fn allocate_with_uniform<T: ShaderType + WriteInto>(
        &mut self,
        ctx: &mut Context,
        uniform: &T,
    ) -> UniformBufferArenaAllocation<T> {
        let allocation = self.allocate(ctx, uniform.size().into());

        let mut buffer = encase::UniformBuffer::new(Vec::new());
        buffer
            .write(&uniform)
            .expect("could not write to uniform buffer");
        render::queue(ctx).write_buffer(
            &allocation.buffer,
            allocation.offset,
            &buffer.into_inner(),
        );

        UniformBufferArenaAllocation {
            allocation,
            ty: PhantomData,
        }
    }

    fn grow(&mut self, ctx: &mut Context) {
        let buffer = render::device(ctx).create_buffer(&self.desc);
        self.buffers.push((ArcBuffer::new(ctx, buffer), 0));
    }

    /// This frees **all** the allocations at once.
    pub fn free(&mut self) {
        for (_, cursor) in &mut self.buffers {
            *cursor = 0;
        }
    }
}

#[derive(Debug, Clone)]
pub struct BufferArenaAllocation {
    pub buffer: ArcBuffer,
    pub offset: u64,
    pub size: u64,
}

pub struct UniformBufferArenaAllocation<T: ShaderType + WriteInto> {
    pub allocation: BufferArenaAllocation,
    ty: PhantomData<T>,
}

impl<T: ShaderType + WriteInto> BindGroupBindable<T> for UniformBufferArenaAllocation<T> {
    fn bindgroup_entry(&self) -> render::BindGroupEntry {
        render::BindGroupEntry::BufferSlice {
            buffer: self.allocation.buffer.clone(),
            offset: self.allocation.offset,
            size: self.allocation.size,
        }
    }
}
