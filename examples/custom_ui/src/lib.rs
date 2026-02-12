mod ui_layout;
mod ui_renderer;

use gbase::{asset, render, wgpu, CallbackResult, Callbacks, Context};

use crate::ui_renderer::UiRenderer;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub async fn run() {
    gbase::run::<App>().await;
}

struct App {
    renderer: UiRenderer,
}

impl Callbacks for App {
    #[no_mangle]
    fn init_ctx() -> gbase::ContextBuilder {
        gbase::ContextBuilder::new().vsync(true)
    }
    #[no_mangle]
    fn new(ctx: &mut Context, cache: &mut asset::AssetCache) -> Self {
        let renderer = UiRenderer::new(ctx, cache);
        Self { renderer }
    }

    #[no_mangle]
    fn render(
        &mut self,
        ctx: &mut Context,
        cache: &mut asset::AssetCache,
        screen_view: &wgpu::TextureView,
    ) -> CallbackResult {
        self.renderer
            .render(ctx, cache, screen_view, render::surface_format(ctx));
        CallbackResult::Continue
    }
}
