use gbase::{Callbacks, Context, ContextBuilder};

struct App {}

impl Callbacks for App {
    fn update(&mut self, _ctx: &mut Context) -> bool {
        log::info!("info");
        log::warn!("warn");
        log::error!("error");
        false
    }
}

#[pollster::main]
pub async fn main() {
    let (ctx, ev) = ContextBuilder::new()
        .log_level(gbase::LogLevel::Info)
        .build()
        .await;
    let app = App {};
    gbase::run(app, ctx, ev).await;
}
