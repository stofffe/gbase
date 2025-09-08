use gbase::{
    input::{self, KeyCode},
    CallbackResult, Callbacks, Context,
};

pub fn main() {
    gbase::run_sync::<App>();
}

pub struct App {}

impl Callbacks for App {
    #[no_mangle]
    fn new(_ctx: &mut Context, _cache: &mut gbase::asset::AssetCache) -> Self {
        Self {}
    }

    #[no_mangle]
    fn render(
        &mut self,
        ctx: &mut Context,
        _cache: &mut gbase::asset::AssetCache,
        _screen_view: &wgpu::TextureView,
    ) -> CallbackResult {
        if input::key_just_pressed(ctx, KeyCode::Digit1) {
            println!("print");
        }
        if input::key_just_pressed(ctx, KeyCode::Digit2) {
            tracing::error!("log error");
            tracing::info!("log info");
        }
        CallbackResult::Continue
    }
}

impl App {
    #[no_mangle]
    fn hot_reload(&mut self, _ctx: &mut Context) {
        Self::init_ctx().init_logging();
    }
}
