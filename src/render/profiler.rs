use crate::time;

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

#[derive(Debug, Clone)]
pub struct GpuProfiler {
    new_times: Vec<GpuProfileQuery>,
    next_free_timestamp: u32,
    capacity: u32,

    readback_times: Vec<GpuProfileResult>,

    timestamp_query_set: wgpu::QuerySet,
    timestamp_query_buffer: wgpu::Buffer,
    timestamp_readback_buffer: wgpu::Buffer,

    enabled: bool,
}

impl GpuProfiler {
    pub(crate) fn new(device: &wgpu::Device, enabled: bool, capacity: u32) -> Self {
        if capacity > wgpu::QUERY_SET_MAX_QUERIES {
            tracing::warn!(
                "gpu profiler has max capacity of {}, using it instead of {}",
                wgpu::QUERY_SET_MAX_QUERIES,
                capacity
            );
        }
        let capacity = capacity.max(wgpu::QUERY_SET_MAX_QUERIES);

        let timestamp_query_set = device.create_query_set(&wgpu::QuerySetDescriptor {
            label: Some("gpu profiler timestamp query set"),
            ty: wgpu::QueryType::Timestamp,
            count: capacity,
        });

        let buffer_size = capacity as u64 * std::mem::size_of::<u64>() as u64;
        let timestamp_query_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("gpu profiler timestamp query buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let timestamp_readback_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("gpu profiler timestamp readback buffer"),
            size: buffer_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

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
            timestamp_readback_buffer,

            enabled,
        }
    }

    pub(crate) fn readback(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        profiler: time::ProfilerWrapper,
    ) {
        if !self.enabled {
            return;
        }
        if self.new_times.is_empty() {
            return;
        }

        let timestamp_count = self.next_free_timestamp;
        let readback_size = timestamp_count as u64 * std::mem::size_of::<u64>() as u64;

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("gpu profiler readback"),
        });
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
        queue.submit([encoder.finish()]);

        let timestamps = super::read_buffer_sync::<u64>(
            device,
            &self.timestamp_readback_buffer,
            0,
            readback_size,
        );

        let mut res = Vec::new();

        for query in self.new_times.iter() {
            let timestamp_diff = timestamps[query.timestamp_end as usize]
                - timestamps[query.timestamp_start as usize];
            let time_ns = timestamp_diff as f32 * queue.get_timestamp_period();
            let time_s = time_ns / 1_000_000_000.0;
            res.push(GpuProfileResult {
                label: query.label,
                time: time_s,
            });
        }

        self.readback_times = res;
        self.next_free_timestamp = 0;
        self.new_times.clear();

        // copy to profiler
        let mut profiler = profiler;
        for res in self.readback_times.iter() {
            profiler.add_gpu_sample(res.label, res.time);
        }
    }

    pub fn enable(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
    pub fn resize(&mut self, device: &wgpu::Device, capacity: u32) {
        *self = Self::new(device, self.enabled, capacity);
    }

    pub fn profile_compute_pass(
        &mut self,
        label: &'static str,
    ) -> Option<wgpu::ComputePassTimestampWrites<'_>> {
        if self.next_free_timestamp > self.capacity {
            tracing::warn!(
                "reached timestamp query capacity {}, ignoring {}",
                self.capacity,
                label,
            );
            return None;
        }
        if !self.enabled {
            return None;
        }

        let (start, end) = (self.next_free_timestamp, self.next_free_timestamp + 1);
        let timestamp_writes = wgpu::ComputePassTimestampWrites {
            query_set: &self.timestamp_query_set,
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
        if self.next_free_timestamp > self.capacity {
            tracing::warn!(
                "reached timestamp query capacity {}, ignoring {}",
                self.capacity,
                label,
            );
            return None;
        }
        if !self.enabled {
            return None;
        }

        let (start, end) = (self.next_free_timestamp, self.next_free_timestamp + 1);
        let timestamp_writes = wgpu::RenderPassTimestampWrites {
            query_set: &self.timestamp_query_set,
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

    pub fn readback_times(&self) -> Vec<GpuProfileResult> {
        self.readback_times.clone()
    }
}
