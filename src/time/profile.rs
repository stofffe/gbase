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
