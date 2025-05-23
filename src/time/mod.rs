mod profile;
mod timer;

use crate::Context;
pub use profile::*;
pub use timer::*;

#[cfg(target_arch = "wasm32")]
use instant::Instant;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;

pub(crate) struct TimeContext {
    // dt
    start_time: Instant,
    last_time: Instant,
    delta_time: f32,
    time_since_start: f32,

    pub(crate) profiler: Profiler,
}

impl Default for TimeContext {
    fn default() -> Self {
        let start_time = Instant::now();
        Self {
            start_time,
            last_time: start_time,
            delta_time: 0.0,

            time_since_start: 0.0,

            profiler: Profiler::new(),
        }
    }
}

impl TimeContext {
    pub(crate) fn pre_update(&mut self) {
        let now = Instant::now();

        self.delta_time = now.duration_since(self.last_time).as_secs_f32();

        self.profiler.add_total_frame_time_sample(self.delta_time);

        self.time_since_start = now.duration_since(self.start_time).as_secs_f32();

        self.last_time = now;
    }

    pub(crate) fn post_update(&mut self) {
        self.profiler.finish();
    }
}

//
// Commands
//

pub fn profiler(ctx: &Context) -> Profiler {
    ctx.time.profiler.clone()
}

/// Returns the time since the start of the application
pub fn time_since_start(ctx: &Context) -> f32 {
    ctx.time.time_since_start
}

/// Return the current time (in seconds)
pub fn current_time(ctx: &Context) -> Instant {
    ctx.time.last_time
}

/// Returns the last delta time
pub fn delta_time(ctx: &Context) -> f32 {
    ctx.time.delta_time
}

/// Returns the frame time (in seconds) for the last 100 frames
pub fn frame_time(ctx: &Context) -> f32 {
    ctx.time.profiler.get_total_frame_time()
    // ctx.time.frame_time_avg
}

/// Returns the frame time (in seconds) for the last 100 frames
pub fn fps(ctx: &Context) -> f32 {
    1.0 / frame_time(ctx)
    // 1.0 / ctx.time.frame_time_avg
}
