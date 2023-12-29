#[cfg(target_arch = "wasm32")]
use instant::Instant;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;

use crate::Context;

const FPS_UPDATE_INTERVAL: f32 = 1.0;

pub(crate) struct TimeContext {
    // dt
    start_time: Instant,
    last_time: Instant,
    last_dt: f32,

    // frame time
    frames: u32,
    last_frame_time: Instant,
    last_ms: f32,

    time_since_start: f32,
}

impl Default for TimeContext {
    fn default() -> Self {
        let start_time = Instant::now();
        Self {
            start_time,
            last_time: start_time,
            last_dt: 0.0,

            frames: 0,
            last_frame_time: start_time,
            last_ms: 0.0,

            time_since_start: 0.0,
        }
    }
}

impl TimeContext {
    pub(crate) fn update_time(&mut self) {
        let now = Instant::now();

        // update dt
        self.last_dt = now.duration_since(self.last_time).as_secs_f32();

        // frame time
        self.frames += 1;
        let update_frame_time_interval = std::time::Duration::from_secs_f32(FPS_UPDATE_INTERVAL);
        if now >= self.last_frame_time + update_frame_time_interval {
            let frame_time = update_frame_time_interval.as_secs_f32() / self.frames as f32;
            self.last_ms = frame_time;
            self.last_frame_time = now;
            self.frames = 0;
        }

        // time since start
        self.time_since_start = Instant::now().duration_since(self.start_time).as_secs_f32();

        self.last_time = now;
    }

    pub(crate) fn time_since_start(&self) -> f32 {
        self.time_since_start
    }
}

pub struct Timer {
    d: std::time::Duration,
    start: Instant,
}

impl Timer {
    pub fn new(d: std::time::Duration) -> Self {
        Self {
            d,
            start: Instant::now(),
        }
    }

    pub fn ticked(&mut self) -> bool {
        let now = Instant::now();
        if now.duration_since(self.start) > self.d {
            self.start = now;
            return true;
        }
        false
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
    ctx.time.last_dt
}

/// Returns the frame time (in seconds) for the last 500ms
pub fn frame_time(ctx: &Context) -> f32 {
    ctx.time.last_ms
}

/// Returns the frame time (in seconds) for the last 500ms
pub fn fps(ctx: &Context) -> f32 {
    1.0 / ctx.time.last_ms
}

// /// Returns the current time at the start of the current frame
// pub fn current_time(ctx: &Context) -> instant::Instant {
//     ctx.time.current_time
// }

// target fps
// let goal = 1f32 / 60f32;
// let time_to_target = goal - self.last_dt;
// if time_to_target > 0.0 {
// spin_sleep::sleep(time::Duration::from_secs_f32(time_to_target));
// std::thread::sleep(time::Duration::from_secs_f32(time_to_target));
// println!(
//     "ms {}, target {}, sleep {}, elapsed {}",
//     self.last_ms,
//     goal,
//     time_to_target,
//     now.elapsed().as_secs_f32()
// );
// }
// println!("elapsed {}", now.elapsed().as_secs_f32());
// self.current_time = instant::Instant::now();

// frame target
// if let Some(frame_target) = self.frame_target {
//     let ms_per_frame = Duration::from_secs_f32(1.0 / frame_target);
//     let time_to_sleep = self.last_time + ms_per_frame - now;
//     // println!("MS {:?}", ms_per_frame);
//     // println!("TO SLEEP {:?}", time_to_sleep);
//     // TODO wasm support
//     spin_sleep::sleep(time_to_sleep);
//     // thread::sleep(time_to_sleep);
// }
