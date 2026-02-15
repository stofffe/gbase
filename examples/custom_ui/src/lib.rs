mod ui_layout;
mod ui_renderer;

use gbase::{asset, render, wgpu, CallbackResult, Callbacks, Context};

use crate::ui_renderer::{UIElementInstace, UIRenderer};

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub async fn run() {
    gbase::run::<App>().await;
}

struct App {
    renderer: UIRenderer,
}

impl Callbacks for App {
    #[no_mangle]
    fn init_ctx() -> gbase::ContextBuilder {
        gbase::ContextBuilder::new().vsync(true)
    }
    #[no_mangle]
    fn new(ctx: &mut Context, cache: &mut asset::AssetCache) -> Self {
        let renderer = UIRenderer::new(ctx, cache, 1024);
        Self { renderer }
    }

    #[no_mangle]
    fn render(
        &mut self,
        ctx: &mut Context,
        cache: &mut asset::AssetCache,
        screen_view: &wgpu::TextureView,
    ) -> CallbackResult {
        let elements = vec![
            UIElementInstace {
                position: [0.0, 0.0],
                size: [200.0, 100.0],
                color: [1.0, 0.0, 0.0, 1.0],
            },
            UIElementInstace {
                position: [0.0, 0.5],
                size: [0.1, 0.2],
                color: [0.0, 0.0, 1.0, 1.0],
            },
            UIElementInstace {
                position: [0.5, 0.0],
                size: [0.2, 0.1],
                color: [0.0, 0.0, 1.0, 1.0],
            },
            UIElementInstace {
                position: [0.5, 0.5],
                size: [0.2, 0.2],
                color: [0.0, 0.0, 1.0, 1.0],
            },
        ];
        self.renderer.render(
            ctx,
            cache,
            screen_view,
            render::surface_format(ctx),
            elements,
        );
        CallbackResult::Continue
    }
}
