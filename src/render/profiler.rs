use std::sync::mpsc;

use crate::{
    time::{self, ProfilerWrapper},
    Context,
};

const READBACK_BUFFER_SIZE: usize = 32;

#[derive(Debug, Clone)]
pub struct GpuProfileQuery {
    pub label: &'static str,
    pub timestamp_start: u32,
    pub timestamp_end: u32,
}

#[derive(Debug, Clone)]
pub struct GpuProfileResult {
    pub label: &'static str,
    pub time: f32,
}

#[derive(Debug)]
struct ReadbackBuffer {
    buffer: wgpu::Buffer,
    sender: mpsc::Sender<()>,
    receiver: mpsc::Receiver<()>,
    queries: Vec<GpuProfileQuery>,
}

#[derive(Debug)]
pub struct GpuProfiler {
    new_times: Vec<GpuProfileQuery>,
    next_free_timestamp: u32,
    capacity: u32,

    queries_supported: bool,
    queries_supported_inside_encoder: bool,
    queries_supported_inside_pass: bool,

    timestamp_query_set: Option<wgpu::QuerySet>,
    readback_times: Vec<GpuProfileResult>,
    timestamp_query_buffer: wgpu::Buffer,

    // readback
    readback_buffers: Vec<ReadbackBuffer>,
    readback_buffer_next_index: usize,
}

impl GpuProfiler {
    pub(crate) fn new(device: &wgpu::Device, capacity: u32) -> Self {
        if capacity > wgpu::QUERY_SET_MAX_QUERIES {
            tracing::warn!(
                "gpu profiler has max capacity of {}, using it instead of {}",
                wgpu::QUERY_SET_MAX_QUERIES,
                capacity
            );
        }
        let capacity = capacity.max(wgpu::QUERY_SET_MAX_QUERIES);

        // supported features
        let features = device.features();
        let queries_supported = features.contains(wgpu::Features::TIMESTAMP_QUERY);
        let queries_supported_inside_encoder =
            features.contains(wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS);
        let queries_supported_inside_pass =
            features.contains(wgpu::Features::TIMESTAMP_QUERY_INSIDE_PASSES);

        let timestamp_query_set = if queries_supported {
            Some(device.create_query_set(&wgpu::QuerySetDescriptor {
                label: Some("gpu profiler timestamp query set"),
                ty: wgpu::QueryType::Timestamp,
                count: capacity,
            }))
        } else {
            None
        };

        dbg!(&timestamp_query_set);

        let buffer_size = capacity as u64 * std::mem::size_of::<u64>() as u64;
        let timestamp_query_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("gpu profiler timestamp query buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let mut readback_buffers = Vec::new();
        for _i in 0..READBACK_BUFFER_SIZE {
            let timestamp_readback_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("gpu profiler timestamp readback buffer"),
                size: buffer_size,
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let (sender, receiver) = std::sync::mpsc::channel();
            readback_buffers.push(ReadbackBuffer {
                buffer: timestamp_readback_buffer,
                sender,
                receiver,
                queries: Vec::new(),
            });
        }

        let times = Vec::with_capacity(capacity as usize);
        let last_frame_times = Vec::with_capacity(capacity as usize);
        let next_free_timestamp = 0;

        Self {
            new_times: times,
            next_free_timestamp,
            capacity,
            readback_times: last_frame_times,

            timestamp_query_set,
            timestamp_query_buffer,

            queries_supported,
            queries_supported_inside_encoder,
            queries_supported_inside_pass,

            readback_buffers,
            readback_buffer_next_index: 0,
        }
    }

    pub(crate) fn readback_async(&mut self, device: &wgpu::Device, queue: &wgpu::Queue) {
        if !self.queries_supported || self.new_times.is_empty() {
            return;
        }
        let Some(query_set) = &self.timestamp_query_set else {
            return;
        };

        // get next available buffer
        let readback_buffer_index = self.readback_buffer_next_index % self.readback_buffers.len();
        let readback_buffer = &mut self.readback_buffers[readback_buffer_index];

        // get nex available timestamp
        let timestamp_count = self.next_free_timestamp;
        let readback_size = timestamp_count as u64 * std::mem::size_of::<u64>() as u64;

        // copy timestamp queries into buffer
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("gpu profiler readback"),
        });
        encoder.resolve_query_set(
            query_set,
            0..timestamp_count,
            &self.timestamp_query_buffer,
            0,
        );
        encoder.copy_buffer_to_buffer(
            &self.timestamp_query_buffer,
            0,
            &readback_buffer.buffer,
            0,
            readback_size,
        );
        queue.submit([encoder.finish()]);

        // map buffer
        let sender = readback_buffer.sender.clone();
        readback_buffer
            .buffer
            .slice(..readback_size)
            .map_async(wgpu::MapMode::Read, move |_res| {
                sender.send(()).expect("could not send");
            });
        device
            .poll(wgpu::MaintainBase::Poll)
            .expect("could not poll");

        readback_buffer.queries = std::mem::take(&mut self.new_times);

        self.readback_buffer_next_index += 1;

        // reset timestamp query
        self.next_free_timestamp = 0;
        self.new_times.clear();
    }

    pub fn poll_readbacks(&mut self, queue: &wgpu::Queue, profiler: &mut ProfilerWrapper) {
        let mut query_results = Vec::new();

        for (i, readback) in self.readback_buffers.iter_mut().enumerate() {
            if readback.receiver.try_recv().is_ok() {
                let data = readback.buffer.slice(..).get_mapped_range();
                let timestamps: Vec<u64> = bytemuck::cast_slice(&data).to_vec();
                drop(data);
                readback.buffer.unmap();

                for query in readback.queries.iter() {
                    let diff = timestamps[query.timestamp_end as usize]
                        - timestamps[query.timestamp_start as usize];
                    let time_ns = diff as f32 * queue.get_timestamp_period();
                    let time_s = time_ns / 1_000_000_000.0;
                    query_results.push(GpuProfileResult {
                        label: query.label,
                        time: time_s,
                    });
                }
                readback.queries.clear();
            }
        }

        // copy to profiler
        for res in query_results.iter() {
            profiler.add_gpu_sample(res.label, res.time);
        }
    }

    pub fn resize(&mut self, device: &wgpu::Device, capacity: u32) {
        *self = Self::new(device, capacity);
    }

    pub fn profile_compute_pass(
        &mut self,
        label: &'static str,
    ) -> Option<wgpu::ComputePassTimestampWrites<'_>> {
        if !self.queries_supported {
            return None;
        }
        let Some(query_set) = &self.timestamp_query_set else {
            return None;
        };
        if self.next_free_timestamp > self.capacity {
            tracing::warn!(
                "reached timestamp query capacity {}, ignoring {}",
                self.capacity,
                label,
            );
            return None;
        }

        let (start, end) = (self.next_free_timestamp, self.next_free_timestamp + 1);
        let timestamp_writes = wgpu::ComputePassTimestampWrites {
            query_set,
            beginning_of_pass_write_index: Some(start),
            end_of_pass_write_index: Some(end),
        };
        self.new_times.push(GpuProfileQuery {
            label,
            timestamp_start: start,
            timestamp_end: end,
        });
        self.next_free_timestamp += 2;

        Some(timestamp_writes)
    }

    pub fn profile_render_pass(
        &mut self,
        label: &'static str,
    ) -> Option<wgpu::RenderPassTimestampWrites<'_>> {
        if !self.queries_supported {
            return None;
        }
        let Some(query_set) = &self.timestamp_query_set else {
            return None;
        };
        if self.next_free_timestamp > self.capacity {
            tracing::warn!(
                "reached timestamp query capacity {}, ignoring {}",
                self.capacity,
                label,
            );
            return None;
        }

        let (start, end) = (self.next_free_timestamp, self.next_free_timestamp + 1);
        self.next_free_timestamp += 2;
        let timestamp_writes = wgpu::RenderPassTimestampWrites {
            query_set,
            beginning_of_pass_write_index: Some(start),
            end_of_pass_write_index: Some(end),
        };
        self.new_times.push(GpuProfileQuery {
            label,
            timestamp_start: start,
            timestamp_end: end,
        });
        Some(timestamp_writes)
    }

    // TODO: inside encoder

    // TODO: inside pass

    pub fn readback_times(&self) -> Vec<GpuProfileResult> {
        self.readback_times.clone()
    }
}
