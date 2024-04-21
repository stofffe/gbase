use gbase::{time, Callbacks, Context, ContextBuilder, LogLevel};

struct App {}

impl Callbacks for App {
    fn update(&mut self, ctx: &mut Context) -> bool {
        log::info!("time since start {}", time::time_since_start(ctx));
        log::info!("delta time {}", time::delta_time(ctx));
        log::info!("frame time {}", time::frame_time(ctx));
        log::info!("fps {}", time::fps(ctx));
        false
    }
}

#[pollster::main]
pub async fn main() {
    let (ctx, ev) = ContextBuilder::new()
        .log_level(LogLevel::Info)
        .vsync(false)
        .build()
        .await;
    let app = App {};
    gbase::run(app, ctx, ev);
}
