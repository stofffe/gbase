mod timer;

pub use timer::*;

#[cfg(target_arch = "wasm32")]
pub use instant::Instant;
use std::collections::VecDeque;
#[cfg(not(target_arch = "wasm32"))]
pub use std::time::Instant;

use crate::Context;

// TODO: contextbuilder options
pub const FIXED_UPDATE_TIME: f32 = 1.0 / 50.0;
pub const FIXED_UPADTE_MAX_TIME: f32 = 0.25;
const TOTAL_FRAME_TIME_SAMPLES: usize = 25;

pub(crate) struct TimeContext {
    // dt
    start_time: Instant,
    last_time: Instant,
    delta_time: f32,
    time_since_start: f32,

    // frame time
    total_frame_time_samples: VecDeque<f32>,
    total_frame_time: f32,

    pub(crate) fixed_accumulator: f32,
}

impl Default for TimeContext {
    fn default() -> Self {
        let start_time = Instant::now();
        Self {
            start_time,
            last_time: start_time,
            delta_time: 0.0,
            fixed_accumulator: 0.0,

            time_since_start: 0.0,

            total_frame_time_samples: VecDeque::with_capacity(TOTAL_FRAME_TIME_SAMPLES),
            total_frame_time: 0.0,
        }
    }
}

impl TimeContext {
    pub(crate) fn update_delta_time(&mut self) {
        let now = Instant::now();

        self.delta_time = now.duration_since(self.last_time).as_secs_f32();

        let clamped_delta_time = self.delta_time.min(FIXED_UPADTE_MAX_TIME); // TODO: is this problematic?
        self.fixed_accumulator += clamped_delta_time;

        // frame time
        self.total_frame_time_samples.push_back(self.delta_time);
        if self.total_frame_time_samples.len() > TOTAL_FRAME_TIME_SAMPLES {
            self.total_frame_time_samples.pop_front();
        }
        self.total_frame_time = self.total_frame_time_samples.iter().sum::<f32>()
            / self.total_frame_time_samples.len() as f32;
        self.time_since_start = now.duration_since(self.start_time).as_secs_f32();

        self.last_time = now;
    }
}

//
// Commands
//

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
    ctx.time.total_frame_time
}

/// Returns the frame time (in seconds) for the last 100 frames
pub fn fps(ctx: &Context) -> f32 {
    1.0 / frame_time(ctx)
}
