use gbase::{filesystem, Callbacks, Context, ContextBuilder, LogLevel};
use log::info;
use std::path::Path;

#[pollster::main]
pub async fn main() {
    let (mut ctx, ev) = ContextBuilder::new()
        .log_level(LogLevel::Info)
        .build()
        .await;
    let app = App::new(&mut ctx).await;
    gbase::run(app, ctx, ev).await;
}

struct App {}

impl App {
    async fn new(ctx: &mut Context) -> Self {
        let txt = filesystem::load_bytes(ctx, Path::new("test.txt"))
            .await
            .unwrap();
        info!("txt content {:?}", String::from_utf8(txt));
        Self {}
    }
}

impl Callbacks for App {
    fn update(&mut self, _ctx: &mut Context) -> bool {
        false
    }
}