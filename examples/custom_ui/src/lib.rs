mod ui_layout;
mod ui_renderer;

use gbase::{asset, glam::vec2, render, wgpu, CallbackResult, Callbacks, Context};

use crate::{
    ui_layout::{UIElement, UILayouter},
    ui_renderer::{UIElementInstace, UIRenderer},
};

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub async fn run() {
    gbase::run::<App>().await;
}

struct App {
    renderer: UIRenderer,
    layouter: UILayouter,
}

impl Callbacks for App {
    #[no_mangle]
    fn init_ctx() -> gbase::ContextBuilder {
        gbase::ContextBuilder::new().vsync(true)
    }
    #[no_mangle]
    fn new(ctx: &mut Context, cache: &mut asset::AssetCache) -> Self {
        let renderer = UIRenderer::new(ctx, cache, 1024);
        let layouter = UILayouter::new();
        Self { renderer, layouter }
    }

    #[no_mangle]
    fn render(
        &mut self,
        ctx: &mut Context,
        cache: &mut asset::AssetCache,
        screen_view: &wgpu::TextureView,
    ) -> CallbackResult {
        let elements = vec![
            UIElement::new()
                .pos(vec2(0.0, 0.0))
                .dimensions(vec2(200.0, 100.0)),
            UIElement::new()
                .pos(vec2(0.0, 200.0))
                .dimensions(vec2(30.0, 120.0)), // UIElement {
                                                //     position: [0.0, 0.0],
                                                //     size: [200.0, 100.0],
                                                //     color: [1.0, 0.0, 0.0, 1.0],
                                                // },
                                                // UIElement {
                                                //     position: [0.0, 200.0],
                                                //     size: [30.0, 120.0],
                                                //     color: [0.0, 0.0, 1.0, 1.0],
                                                // },
                                                // UIElement {
                                                //     position: [200.0, 0.0],
                                                //     size: [130.0, 120.0],
                                                //     color: [0.0, 0.0, 1.0, 1.0],
                                                // },
                                                // UIElement {
                                                //     position: [200.0, 350.0],
                                                //     size: [500.0, 400.0],
                                                //     color: [0.0, 0.0, 1.0, 1.0],
                                                // },
        ];

        let screen_size = render::surface_size(ctx);
        let elements = self.layouter.layout_elements(
            vec2(screen_size.width as f32, screen_size.height as f32),
            elements,
        );

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
