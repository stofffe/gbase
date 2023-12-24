use std::marker::PhantomData;

use wgpu::util::DeviceExt;

pub trait InstanceGpuTrait: bytemuck::Pod + bytemuck::Zeroable {
    const SIZE: u64;
    fn desc() -> wgpu::VertexBufferLayout<'static>;
}

pub trait InstaceTrait<G: InstanceGpuTrait> {
    fn to_gpu(&self) -> G;
}

pub struct InstanceBuffer<G: InstanceGpuTrait, T: InstaceTrait<G>> {
    pub vec: Vec<T>,
    pub buffer: wgpu::Buffer,
    ty: PhantomData<G>,
}

impl<G: InstanceGpuTrait, T: InstaceTrait<G>> InstanceBuffer<G, T> {
    /// Create empty instace buffer
    ///
    /// Size depends on ```size```
    pub fn new_empty(device: &wgpu::Device, size: u64) -> Self {
        let vec = Vec::with_capacity(size as usize);
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: G::SIZE * size,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self {
            vec,
            buffer,
            ty: PhantomData::<G>,
        }
    }

    /// Create instance buffer from existing vector
    ///
    /// Buffer size depends on vector
    pub fn new_data(device: &wgpu::Device, vec: Vec<T>) -> Self {
        let gpu_vec = vec.iter().map(T::to_gpu).collect::<Vec<_>>();
        let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(&gpu_vec),
            usage: wgpu::BufferUsages::VERTEX,
        });
        Self {
            vec,
            buffer,
            ty: PhantomData::<G>,
        }
    }

    pub fn update_buffer(&self, queue: &wgpu::Queue) {
        let gpu_vec = self.vec.iter().map(T::to_gpu).collect::<Vec<_>>();
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&gpu_vec));
    }

    pub fn desc(&self) -> wgpu::VertexBufferLayout<'static> {
        G::desc()
    }

    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> u32 {
        self.vec.len() as u32
    }
}

// struct Instances {
//     vec: Vec<Instance>,
//     buffer: wgpu::Buffer,
// }
//
// impl Instances {
//     fn new(device: &wgpu::Device, size: u64) -> Self {
//         let vec = Vec::with_capacity(size as usize);
//         let buffer = device.create_buffer(&wgpu::BufferDescriptor {
//             label: Some("instance buffer"),
//             size: GPUInstance::SIZE * size,
//             usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
//             mapped_at_creation: false,
//         });
//         Self { vec, buffer }
//     }
//
//     fn update_buffer(&self, ctx: &mut Context) {
//         let queue = render::queue(ctx);
//         let gpu_vec = self.vec.iter().map(Instance::to_gpu).collect::<Vec<_>>();
//         queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&gpu_vec));
//     }
// }
//
// struct Instance {
//     pos: Vec3,
//     rot: Vec2,
// }
//
// impl Instance {
//     fn to_gpu(&self) -> GPUInstance {
//         GPUInstance {
//             pos: self.pos.to_array(),
//             rot: self.rot.to_array(),
//         }
//     }
// }
//
// #[repr(C)]
// #[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
// struct GPUInstance {
//     pos: [f32; 3],
//     rot: [f32; 2],
// }
//
// impl GPUInstance {
//     const SIZE: u64 = std::mem::size_of::<Self>() as u64;
//     const ATTRIBUTES: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![
//         1=>Float32x3,
//         2=>Float32x2,
//     ];
//     pub fn desc() -> wgpu::VertexBufferLayout<'static> {
//         wgpu::VertexBufferLayout {
//             array_stride: Self::SIZE,
//             step_mode: wgpu::VertexStepMode::Instance,
//             attributes: &Self::ATTRIBUTES,
//         }
//     }
// }
