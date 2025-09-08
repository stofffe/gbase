use std::time::Duration;

use gbase::{
    input, tracing,
    wgpu::{self, CompositeAlphaMode},
    CallbackResult, Callbacks, Context,
};

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub async fn run() {
    gbase::run::<App>().await;
}

#[derive(Debug)]
struct App {}
impl Callbacks for App {
    #[no_mangle]
    fn init_ctx() -> gbase::ContextBuilder {
        gbase::ContextBuilder::new()
            .log_level(tracing::Level::INFO)
            .vsync(false)
            .device_features(wgpu::Features::TIMESTAMP_QUERY)
    }
    #[no_mangle]
    fn new(ctx: &mut Context, _cache: &mut gbase::asset::AssetCache) -> Self {
        Self {}
    }

    fn fixed_update(
        &mut self,
        ctx: &mut Context,
        _cache: &mut gbase::asset::AssetCache,
    ) -> CallbackResult {
        tracing::info!("fixed update called");
        CallbackResult::Continue
    }

    #[no_mangle]
    fn render(
        &mut self,
        ctx: &mut Context,
        _cache: &mut gbase::asset::AssetCache,
        screen_view: &wgpu::TextureView,
    ) -> CallbackResult {
        if input::key_just_pressed(ctx, input::KeyCode::Space) {
            tracing::error!("LAG SPIKE");
            std::thread::sleep(Duration::from_millis(2000));
        }
        tracing::info!("render called");
        CallbackResult::Continue
    }
}

#[no_mangle]
fn hot_reload() {
    App::init_ctx().init_logging();
}
