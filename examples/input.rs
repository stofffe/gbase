use gbase::{
    input::{self, KeyCode},
    Callbacks, Context, ContextBuilder, LogLevel,
};

struct App {}

impl Callbacks for App {
    fn update(&mut self, ctx: &mut Context) -> bool {
        if input::key_just_pressed(ctx, KeyCode::KeyA) {
            log::info!("A pressed");
        }
        if input::key_released(ctx, KeyCode::KeyA) {
            log::info!("A released");
        }
        if input::key_pressed(ctx, KeyCode::Space) {
            log::info!("mouse pos: {:?}", input::mouse_pos(ctx));
            // log::info!("mouse delta: {:?}", input::mouse_delta(ctx));
            // log::info!("scroll delta: {:?}", input::scroll_delta(ctx));
        }
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
    gbase::run(app, ctx, ev);
}
