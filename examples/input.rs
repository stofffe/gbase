use gbase::{
    input::{self, KeyCode},
    Callbacks, Context,
};

pub fn main() {
    gbase::run_sync::<App>();
}

struct App {}

impl Callbacks for App {
    fn new(_ctx: &mut Context, _cache: &mut gbase::asset::AssetCache) -> Self {
        Self {}
    }
    fn update(&mut self, ctx: &mut Context, _cache: &mut gbase::asset::AssetCache) -> bool {
        if input::key_just_pressed(ctx, KeyCode::KeyA) {
            tracing::info!("A pressed");
        }
        if input::key_released(ctx, KeyCode::KeyA) {
            tracing::info!("A released");
        }
        if input::key_pressed(ctx, KeyCode::Space) {
            tracing::info!("mouse pos: {:?}", input::mouse_pos(ctx));
            // tracing::info!("mouse delta: {:?}", input::mouse_delta(ctx));
            // tracing::info!("scroll delta: {:?}", input::scroll_delta(ctx));
        }
        false
    }
}
