use std::fs;

use gbase::{
    filesystem,
    input::{self, KeyCode},
    Callbacks, Context, ContextBuilder, LogLevel,
};
use log::info;

#[pollster::main]
pub async fn main() {
    let (mut ctx, ev) = ContextBuilder::new()
        .log_level(LogLevel::Info)
        .build()
        .await;
    let app = App::new(&mut ctx).await;
    gbase::run(app, ctx, ev);
}

struct App {}

impl App {
    async fn new(_ctx: &mut Context) -> Self {
        let s = filesystem::load_s!("other/test.txt");
        log::warn!("s {:?}", s);

        Self {}
    }
}

impl Callbacks for App {
    fn update(&mut self, ctx: &mut Context) -> bool {
        if input::key_just_pressed(ctx, KeyCode::KeyR) {
            let s = filesystem::load_s!("other/test.txt");
            log::warn!("s {:?}", s);
        }
        false
    }
}
