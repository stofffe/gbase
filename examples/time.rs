use std::time::Duration;

use gbase::{time, Callbacks, Context};

pub fn main() {
    gbase::run_sync::<App>();
}

struct App {
    timer: time::Timer,
}

impl Callbacks for App {
    fn new(_ctx: &mut Context) -> Self {
        let timer = time::Timer::new(Duration::from_secs(1));
        Self { timer }
    }
    fn update(&mut self, ctx: &mut Context) -> bool {
        // log::info!("time since start {}", time::time_since_start(ctx));
        // log::info!("current time {:?}", time::current_time(ctx));
        // log::info!("delta time {}", time::delta_time(ctx));
        // log::info!("frame time {}", time::frame_time(ctx));
        // log::info!("fps {}", time::fps(ctx));

        if self.timer.just_ticked() {
            log::info!("timer just ticked");
            self.timer.reset();
        }
        if self.timer.ticked() {
            log::info!("timer has ticked");
        }

        false
    }
}
