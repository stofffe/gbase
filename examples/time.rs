use gbase::{
    input::{self, KeyCode},
    time, Callbacks, Context, ContextBuilder, LogLevel,
};

struct App {}

impl Callbacks for App {
    fn update(&mut self, ctx: &mut Context) -> bool {
        // log::info!("FT {}", time::frame_time(ctx));

        false
    }
}

#[pollster::main]
pub async fn main() {
    let (ctx, ev) = ContextBuilder::new()
        .log_level(LogLevel::Info)
        .build()
        .await;
    let app = App {};
    gbase::run(app, ctx, ev).await;
}

