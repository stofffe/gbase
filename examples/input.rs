use gbase::{
    input::{self, KeyCode},
    Callbacks, Context, LogLevel,
};

pub fn main() {
    gbase::ContextBuilder::new()
        .log_level(LogLevel::Info)
        .run_sync::<App>();
}

struct App {}

impl Callbacks for App {
    fn new(_ctx: &mut Context) -> Self {
        Self {}
    }
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
