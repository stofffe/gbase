use crate::Context;
use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::{Arc, Mutex},
    time::Instant,
};

const SAMPLES: usize = 20;

#[derive(Clone)]
pub struct Profiler {
    inner: Arc<Mutex<ProfilerInner>>,
}

// TODO: replace duration

impl Profiler {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(ProfilerInner::new())),
        }
    }

    pub fn add_sample(&mut self, label: &'static str, time: f32) {
        self.inner.lock().unwrap().add_sample(label, time);
    }

    pub fn finish(&mut self) {
        self.inner.lock().unwrap().finish();
    }
    pub fn extract(&self) -> Vec<(&'static str, f32)> {
        self.inner.lock().unwrap().get_samples()
    }
    pub fn add_total_frame_time_sample(&mut self, time: f32) {
        self.inner.lock().unwrap().add_total_frame_time_sample(time);
    }
    pub fn get_total_frame_time(&self) -> f32 {
        self.inner.lock().unwrap().get_total_frame_time()
    }
}

/// Averages profiling samples over time
struct ProfilerInner {
    samples: HashMap<&'static str, VecDeque<f32>>,
    recent: HashSet<&'static str>,

    total_frame_time: VecDeque<f32>,
}

impl ProfilerInner {
    pub fn new() -> Self {
        Self {
            samples: HashMap::new(),
            recent: HashSet::new(),
            total_frame_time: VecDeque::new(),
        }
    }

    fn add_sample(&mut self, label: &'static str, time: f32) {
        let queue = self.samples.entry(label).or_default();
        queue.push_back(time);
        if queue.len() > SAMPLES {
            queue.pop_front();
        }

        self.recent.insert(label);
    }

    fn add_total_frame_time_sample(&mut self, time: f32) {
        self.total_frame_time.push_back(time);
        if self.total_frame_time.len() > SAMPLES {
            self.total_frame_time.pop_front();
        }
    }

    fn finish(&mut self) {
        let mut filtered_samples = HashMap::new();
        for (label, value) in self.samples.drain() {
            if self.recent.contains(&label) {
                filtered_samples.insert(label, value);
            }
        }
        self.samples = filtered_samples;
        self.recent.clear();
    }

    fn get_samples(&self) -> Vec<(&'static str, f32)> {
        let mut filtered_samples = Vec::new();
        for (&label, queue) in self.samples.iter() {
            if self.recent.contains(label) {
                let average = queue.iter().sum::<f32>() / queue.len() as f32;
                filtered_samples.push((label, average));
            }
        }
        filtered_samples.sort_by_key(|(label, _)| *label);
        filtered_samples
    }

    fn get_total_frame_time(&self) -> f32 {
        self.total_frame_time.iter().sum::<f32>() / self.total_frame_time.len() as f32
    }
}

pub struct ProfileTimer {
    profiler: Profiler,
    label: &'static str,
    start: Instant,
}

impl ProfileTimer {
    pub fn new(ctx: &Context, label: &'static str) -> Self {
        Self {
            label,
            start: Instant::now(),
            profiler: ctx.time.profiler.clone(),
        }
    }

    pub fn scoped(ctx: &mut Context, label: &'static str, code: impl FnOnce(&mut Context)) {
        let _guard = Self::new(ctx, label);
        code(ctx);
    }

    pub fn finish(self) {
        drop(self);
    }
}

impl Drop for ProfileTimer {
    fn drop(&mut self) {
        self.profiler
            .add_sample(self.label, self.start.elapsed().as_secs_f32());
    }
}

// #[derive(Clone, Debug)]
// pub struct ProfileResult {
//     pub label: &'static str,
//     pub time_ms: f32,
// }
//
// pub struct CpuProfileTimer {
//     label: &'static str,
//     start: Instant,
// }
//
// impl CpuProfileTimer {
//     pub fn new(label: &'static str) -> Self {
//         let start = Instant::now();
//         Self { label, start }
//     }
// }
//
// pub struct CpuProfiler {
//     times: Vec<ProfileResult>,
// }
//
// impl CpuProfiler {
//     pub fn new() -> Self {
//         Self { times: Vec::new() }
//     }
//
//     pub fn profile(&mut self, timer: CpuProfileTimer) {
//         self.times.push(ProfileResult {
//             label: timer.label,
//             time_ms: timer.start.elapsed().as_secs_f32() * 1000.0,
//         });
//     }
//
//     pub fn readback(&mut self) -> Vec<ProfileResult> {
//         let times = self.times.clone();
//         self.times.clear();
//         times
//     }
// }
