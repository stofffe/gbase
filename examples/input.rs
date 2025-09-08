use gbase::{
    input::{self, KeyCode},
    CallbackResult, Callbacks, Context,
};

pub fn main() {
    gbase::run_sync::<App>();
}

struct App {}

impl Callbacks for App {
    fn new(_ctx: &mut Context, _cache: &mut gbase::asset::AssetCache) -> Self {
        Self {}
    }
    fn render(
        &mut self,
        ctx: &mut Context,
        _cache: &mut gbase::asset::AssetCache,
        _screen_view: &wgpu::TextureView,
    ) -> CallbackResult {
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
        CallbackResult::Continue
    }
}
