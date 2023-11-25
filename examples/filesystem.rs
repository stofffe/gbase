use std::path::Path;

use gbase::{filesystem, Callbacks, Context, ContextBuilder, LogLevel};
use log::info;

struct App {}

impl Callbacks for App {
    fn update(&mut self, ctx: &mut Context) -> bool {
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
    let txt = filesystem::load_bytes(&ctx, Path::new("test.txt"))
        .await
        .unwrap();
    info!("txt content {:?}", String::from_utf8(txt));
    gbase::run(app, ctx, ev).await;
}
