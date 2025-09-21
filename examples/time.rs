use std::time::Duration;

use gbase::{time, CallbackResult, Callbacks, Context};

pub fn main() {
    gbase::run_sync::<App>();
}

struct App {
    timer: time::Timer,
}

impl Callbacks for App {
    fn new(_ctx: &mut Context, _cache: &mut gbase::asset::AssetCache) -> Self {
        let timer = time::Timer::new(Duration::from_secs(1));
        Self { timer }
    }

    fn render(
        &mut self,
        _ctx: &mut Context,
        _cache: &mut gbase::asset::AssetCache,
        _screen_view: &wgpu::TextureView,
    ) -> CallbackResult {
        if self.timer.just_ticked() {
            tracing::info!("timer just ticked");
            self.timer.reset();
        }
        if self.timer.ticked() {
            tracing::info!("timer has ticked");
        }

        CallbackResult::Continue
    }
}
