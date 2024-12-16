use gbase::{time, Callbacks, Context, ContextBuilder, LogLevel};

pub fn main() {
    gbase::run_app_with_builder::<App>(
        ContextBuilder::new().log_level(LogLevel::Info).vsync(false),
    );
}

struct App {}

impl Callbacks for App {
    fn new(_ctx: &mut Context) -> Self {
        Self {}
    }
    fn update(&mut self, ctx: &mut Context) -> bool {
        log::info!("time since start {}", time::time_since_start(ctx));
        log::info!("delta time {}", time::delta_time(ctx));
        log::info!("frame time {}", time::frame_time(ctx));
        log::info!("fps {}", time::fps(ctx));
        false
    }
}
