use crate::{render, Context};

use super::Instant;

// pub struct ProfileTimer {
//     name: &'static str,
//     start: Instant,
// }
//
// impl ProfileTimer {
//     pub fn new(name: &'static str) -> Self {
//         let start = Instant::now();
//         Self { name, start }
//     }
//
//     pub fn log(self) {
//         drop(self);
//     }
// }
//
// impl Drop for ProfileTimer {
//     fn drop(&mut self) {
//         let time = self.start.elapsed().as_secs_f64() * 1000.0;
//         log::info!("[PROFILE] {:.5} ms: {}", time, self.name);
//     }
// }

#[derive(Clone, Debug)]
pub struct ProfileResult {
    pub label: &'static str,
    pub time_ms: f32,
}

pub struct CpuProfileTimer {
    label: &'static str,
    start: Instant,
}

impl CpuProfileTimer {
    pub fn new(label: &'static str) -> Self {
        let start = Instant::now();
        Self { label, start }
    }
}

pub struct CpuProfiler {
    times: Vec<ProfileResult>,
}

impl CpuProfiler {
    pub fn new() -> Self {
        Self { times: Vec::new() }
    }

    pub fn profile(&mut self, timer: CpuProfileTimer) {
        self.times.push(ProfileResult {
            label: timer.label,
            time_ms: timer.start.elapsed().as_secs_f32() * 1000.0,
        });
    }

    pub fn readback(&mut self) -> Vec<ProfileResult> {
        let times = self.times.clone();
        self.times.clear();
        times
    }
}

pub struct GpuProfiler {
    times: Vec<(&'static str, u32, u32)>,
    next_free_timestamp: u32,
    capacity: u32,

    timestamp_query_set: wgpu::QuerySet,
    timestamp_query_buffer: wgpu::Buffer,
    timestamp_readback_buffer: wgpu::Buffer,
}

impl GpuProfiler {
    pub fn new(ctx: &Context, capacity: u32) -> Self {
        let timestamp_query_set = render::device(ctx).create_query_set(&wgpu::QuerySetDescriptor {
            label: None,
            ty: wgpu::QueryType::Timestamp,
            count: capacity,
        });

        let buffer_size = capacity as u64 * std::mem::size_of::<u64>() as u64;
        let timestamp_query_buffer = render::device(ctx).create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: buffer_size,
            usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let timestamp_readback_buffer =
            render::device(ctx).create_buffer(&wgpu::BufferDescriptor {
                label: None,
                size: buffer_size,
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });

        let times = Vec::with_capacity(capacity as usize);
        let previous_timestamp = 0;

        Self {
            times,
            next_free_timestamp: previous_timestamp,
            capacity,

            timestamp_query_set,
            timestamp_query_buffer,
            timestamp_readback_buffer,
        }
    }

    pub fn readback(&mut self, ctx: &Context) -> Vec<(&'static str, f32)> {
        if self.times.is_empty() {
            log::warn!("trying to read back empty timestamp query set");
            return Vec::new();
        }

        let timestamp_count = self.next_free_timestamp;
        let readback_size = timestamp_count as u64 * std::mem::size_of::<u64>() as u64;

        let mut encoder = render::EncoderBuilder::new().build(ctx);
        encoder.resolve_query_set(
            &self.timestamp_query_set,
            0..timestamp_count,
            &self.timestamp_query_buffer,
            0,
        );
        encoder.copy_buffer_to_buffer(
            &self.timestamp_query_buffer,
            0,
            &self.timestamp_readback_buffer,
            0,
            readback_size,
        );

        let queue = render::queue(ctx);
        queue.submit([encoder.finish()]);

        let timestamps =
            render::read_buffer_sync::<u64>(ctx, &self.timestamp_readback_buffer, 0, readback_size);

        let mut res = Vec::new();

        for &(name, start, end) in self.times.iter() {
            let timestamp_diff = timestamps[end as usize] - timestamps[start as usize];
            let time_ns = timestamp_diff as f32 * render::queue(ctx).get_timestamp_period();
            let time_ms = time_ns / 1_000_000.0;
            res.push((name, time_ms));
        }

        self.times.clear();
        self.next_free_timestamp = 0;

        res
    }

    pub fn profile_compute(&mut self, label: &'static str) -> wgpu::ComputePassTimestampWrites<'_> {
        debug_assert!(
            self.next_free_timestamp + 1 < self.capacity,
            "max capacity of timestamp query set reached: {}",
            self.capacity
        );

        let (start, end) = (self.next_free_timestamp, self.next_free_timestamp + 1);
        let timestamp_writes = wgpu::ComputePassTimestampWrites {
            query_set: &self.timestamp_query_set,
            beginning_of_pass_write_index: Some(start),
            end_of_pass_write_index: Some(end),
        };
        self.times.push((label, start, end));
        self.next_free_timestamp += 2;
        timestamp_writes
    }

    pub fn profile_render(&mut self, label: &'static str) -> wgpu::RenderPassTimestampWrites<'_> {
        debug_assert!(
            self.next_free_timestamp + 1 < self.capacity,
            "max capacity of timestamp query set reached: {}",
            self.capacity
        );

        let (start, end) = (self.next_free_timestamp, self.next_free_timestamp + 1);
        let timestamp_writes = wgpu::RenderPassTimestampWrites {
            query_set: &self.timestamp_query_set,
            beginning_of_pass_write_index: Some(start),
            end_of_pass_write_index: Some(end),
        };
        self.times.push((label, start, end));
        self.next_free_timestamp += 2;
        timestamp_writes
    }
}
