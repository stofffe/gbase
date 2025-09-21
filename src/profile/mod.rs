mod profiler;
use std::mem;

use crate::{Context, ContextBuilder};
use profiler::GpuProfiler;

pub struct ProfileContext {
    pub gpu_profiler: GpuProfiler,

    #[cfg(feature = "trace_tracy")]
    pub tracy_cpu_client: tracy_client::Client,
    #[cfg(feature = "trace_tracy")]
    pub tracy_gpu_client: tracy_client::GpuContext,
}

impl ProfileContext {
    pub fn new(
        context_builder: &ContextBuilder,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
    ) -> Self {
        let gpu_profiler = GpuProfiler::new(
            device,
            context_builder.gpu_profiler_capacity,
            context_builder.gpu_profiler_enabled,
        );

        #[cfg(feature = "trace_tracy")]
        let tracy_gpu_client = {
            let gpu_period = queue.get_timestamp_period();
            let gpu_timestamp = gather_gpu_timestamp_sync(device, queue);
            tracing::error!("sync timestamp {:?}", gpu_timestamp);
            tracy_client::Client::start()
                .new_gpu_context(
                    Some("tracy_gpu_context"),
                    tracy_client::GpuContextType::Invalid,
                    gpu_timestamp as i64,
                    gpu_period,
                )
                .expect("could not create tracy gpu context")
        };
        #[cfg(feature = "trace_tracy")]
        let tracy_cpu_client = tracy_client::Client::start();

        Self {
            gpu_profiler,

            #[cfg(feature = "trace_tracy")]
            tracy_cpu_client,
            #[cfg(feature = "trace_tracy")]
            tracy_gpu_client,
        }
    }
}

/// Get a gpu timestamp sync
///
/// Executes a noop compute shader to force a gpu tick
fn gather_gpu_timestamp_sync(device: &wgpu::Device, queue: &wgpu::Queue) -> u64 {
    let query_set = device.create_query_set(&wgpu::QuerySetDescriptor {
        label: Some("tracy gpu-sync timestamp"),
        ty: wgpu::QueryType::Timestamp,
        count: 1,
    });

    let query_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("tracy gpu-sync query buffer"),
        size: std::mem::size_of::<u64>() as u64,
        usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::QUERY_RESOLVE,
        mapped_at_creation: false,
    });

    let readback_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("tracy gpu-sync readback buffer"),
        size: std::mem::size_of::<u64>() as u64,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("tracy gpu-sync buffer"),
    });

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("empty compute shader"),
        source: wgpu::ShaderSource::Wgsl("@compute @workgroup_size(1) fn main() {}".into()),
    });

    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("empty compute pipeline"),
        layout: None,
        module: &shader,
        entry_point: Some("main"),
        compilation_options: wgpu::PipelineCompilationOptions {
            constants: &[],
            zero_initialize_workgroup_memory: false,
        },
        cache: None,
    });

    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("tracy gpu-sync compute pass"),
            timestamp_writes: Some(wgpu::ComputePassTimestampWrites {
                query_set: &query_set,
                beginning_of_pass_write_index: Some(0), // might wanna use end instead
                end_of_pass_write_index: None,
            }),
        });
        pass.set_pipeline(&pipeline);
        pass.dispatch_workgroups(1, 1, 1);
    }

    encoder.resolve_query_set(&query_set, 0..1, &query_buffer, 0);

    encoder.copy_buffer_to_buffer(
        &query_buffer,
        0,
        &readback_buffer,
        0,
        mem::size_of::<u64>() as u64,
    );

    queue.submit(Some(encoder.finish()));

    // readback timestamp query result
    let buffer_slice = readback_buffer.slice(0..mem::size_of::<u64>() as u64);
    buffer_slice.map_async(wgpu::MapMode::Read, |_| {});
    device
        .poll(wgpu::MaintainBase::Wait)
        .expect("could not poll");
    let data = buffer_slice.get_mapped_range();
    let gpu_timestamp: Vec<u64> = bytemuck::cast_slice(&data).to_vec();
    drop(data);
    readback_buffer.unmap();

    gpu_timestamp[0]
}

//
// Commands
//

pub fn enable_gpu_profiling(ctx: &mut Context, enabled: bool) {
    ctx.profile.gpu_profiler.set_enabled(enabled);
}
pub fn gpu_profiler(ctx: &mut Context) -> &mut GpuProfiler {
    &mut ctx.profile.gpu_profiler
}
