use std::{
    collections::{HashMap, HashSet, VecDeque},
    sync::{Arc, Mutex},
};

use tracing::span::Id;

use crate::{time, Context};

const SAMPLES: usize = 25;

#[derive(Debug, Clone)]
pub struct ProfilerWrapper {
    inner: Arc<Mutex<Profiler>>,
}

// TODO: replace duration

impl ProfilerWrapper {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(Profiler::new())),
        }
    }
    pub fn finish(&mut self) {
        self.inner.lock().unwrap().finish();
    }

    pub fn add_cpu_sample(&mut self, label: &'static str, time: f32) {
        self.inner.lock().unwrap().add_cpu_sample(label, time);
    }
    pub fn get_cpu_samples(&self) -> Vec<(&'static str, f32)> {
        self.inner.lock().unwrap().get_cpu_samples()
    }
    pub fn add_gpu_sample(&mut self, label: &'static str, time: f32) {
        self.inner.lock().unwrap().add_gpu_sample(label, time);
    }
    pub fn get_gpu_samples(&self) -> Vec<(&'static str, f32)> {
        self.inner.lock().unwrap().get_gpu_samples()
    }
}

/// Averages profiling samples over time
#[derive(Debug)]
struct Profiler {
    cpu_samples: HashMap<&'static str, VecDeque<f32>>,
    cpu_recent: HashSet<&'static str>,

    gpu_samples: HashMap<&'static str, VecDeque<f32>>,
    gpu_recent: HashSet<&'static str>,
}

impl Profiler {
    pub fn new() -> Self {
        Self {
            cpu_samples: HashMap::new(),
            cpu_recent: HashSet::new(),
            gpu_samples: HashMap::new(),
            gpu_recent: HashSet::new(),
        }
    }

    fn finish(&mut self) {
        // cpu
        let mut filtered_samples = HashMap::new();
        for (label, value) in self.cpu_samples.drain() {
            if self.cpu_recent.contains(&label) {
                filtered_samples.insert(label, value);
            }
        }
        self.cpu_samples = filtered_samples;
        self.cpu_recent.clear();

        // gpu
        let mut filtered_samples = HashMap::new();
        for (label, value) in self.gpu_samples.drain() {
            if self.gpu_recent.contains(&label) {
                filtered_samples.insert(label, value);
            }
        }
        self.gpu_samples = filtered_samples;
        self.gpu_recent.clear();
    }

    // cpu
    fn add_cpu_sample(&mut self, label: &'static str, time: f32) {
        let queue = self.cpu_samples.entry(label).or_default();
        queue.push_back(time);
        if queue.len() > SAMPLES {
            queue.pop_front();
        }

        self.cpu_recent.insert(label);
    }
    fn get_cpu_samples(&self) -> Vec<(&'static str, f32)> {
        let mut filtered_samples = Vec::new();
        for (&label, queue) in self.cpu_samples.iter() {
            if self.cpu_recent.contains(label) {
                let average = queue.iter().sum::<f32>() / queue.len() as f32;
                filtered_samples.push((label, average));
            }
        }
        filtered_samples.sort_by_key(|(label, _)| *label);
        filtered_samples
    }

    // gpu
    fn add_gpu_sample(&mut self, label: &'static str, time: f32) {
        let queue = self.gpu_samples.entry(label).or_default();
        queue.push_back(time);
        if queue.len() > SAMPLES {
            queue.pop_front();
        }

        self.gpu_recent.insert(label);
    }
    fn get_gpu_samples(&self) -> Vec<(&'static str, f32)> {
        let mut filtered_samples = Vec::new();
        for (&label, queue) in self.gpu_samples.iter() {
            if self.gpu_recent.contains(label) {
                let average = queue.iter().sum::<f32>() / queue.len() as f32;
                filtered_samples.push((label, average));
            }
        }
        filtered_samples.sort_by_key(|(label, _)| *label);
        filtered_samples
    }
}

//
// Tracing layer
//

/// Converts tracing spans to timestamps in `Profiler`
pub(crate) struct CpuProfilingLayer {
    profiler: ProfilerWrapper,
    spans: std::sync::Mutex<HashMap<tracing::span::Id, time::Instant>>,
}

impl CpuProfilingLayer {
    pub fn new(profiler: ProfilerWrapper) -> Self {
        Self {
            spans: std::sync::Mutex::new(HashMap::new()),
            profiler,
        }
    }
}

impl<S> tracing_subscriber::Layer<S> for CpuProfilingLayer
where
    S: tracing::subscriber::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    fn on_new_span(
        &self,
        _attrs: &tracing::span::Attributes<'_>,
        id: &Id,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let mut spans = self.spans.lock().unwrap();
        spans.insert(id.clone(), time::Instant::now());
    }

    fn on_exit(&self, id: &Id, _ctx: tracing_subscriber::layer::Context<'_, S>) {
        let mut spans = self.spans.lock().unwrap();
        if let Some(start) = spans.remove(id) {
            let duration = start.elapsed();

            let label = _ctx
                .span(id)
                .map(|span| span.metadata().name())
                .unwrap_or("[NO LABEL]");

            self.profiler
                .clone()
                .add_cpu_sample(label, duration.as_secs_f32());
        }
    }
}

//
// Timer
//

/// Times a timespan and sends to Profiler
pub struct ProfileTimer {
    profiler: ProfilerWrapper,
    label: &'static str,
    start: time::Instant,
}

impl ProfileTimer {
    pub fn new(ctx: &Context, label: &'static str) -> Self {
        Self {
            label,
            start: time::Instant::now(),
            profiler: ctx.profile.cpu_profiler.clone(),
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
            .add_cpu_sample(self.label, self.start.elapsed().as_secs_f32());
    }
}
