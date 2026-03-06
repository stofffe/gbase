mod ui_font;
mod ui_layout;
mod ui_renderer;

use crate::{
    ui_font::UIFont,
    ui_layout::{LayoutDirection, Padding, Sizing, TextInfo, UIElement, UILayouter},
    ui_renderer::UIRenderer,
};
use gbase::{asset, filesystem, glam::vec4, render, wgpu, CallbackResult, Callbacks, Context};

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub async fn run() {
    gbase::run::<App>().await;
}

struct App {
    renderer: UIRenderer,
    layouter: UILayouter,
    font: UIFont,
}

impl Callbacks for App {
    #[no_mangle]
    fn init_ctx() -> gbase::ContextBuilder {
        gbase::ContextBuilder::new().vsync(true)
    }
    #[no_mangle]
    fn new(ctx: &mut Context, cache: &mut asset::AssetCache) -> Self {
        let renderer = UIRenderer::new(ctx, cache, 1024);
        let font = UIFont::new(include_bytes!("../assets/fonts/font.ttf")); // TODO: temp
        let layouter = UILayouter::new();
        Self {
            renderer,
            layouter,
            font,
        }
    }

    #[no_mangle]
    fn render(
        &mut self,
        ctx: &mut Context,
        cache: &mut asset::AssetCache,
        screen_view: &wgpu::TextureView,
    ) -> CallbackResult {
        UIElement::new()
            .sizing_x(Sizing::Grow)
            .sizing_y(Sizing::Grow)
            .layout_direction(LayoutDirection::TopToBottom)
            .draw_with_children(&mut self.layouter, |layouter| {
                UIElement::new()
                    .sizing_x(Sizing::Grow)
                    .sizing_y(Sizing::Fit)
                    .padding(Padding::new(20.0, 20.0, 20.0, 20.0))
                    .child_gap(20.0)
                    .background_color(vec4(0.2, 0.2, 0.2, 1.0))
                    .draw_with_children(layouter, |layouter| {
                        UIElement::new()
                            .background_color(vec4(1.0, 0.0, 1.0, 1.0))
                            .text("Hello brooo")
                            .font_size(32)
                            .draw(layouter);
                        UIElement::new()
                            .sizing_x(Sizing::Fixed(100.0))
                            .sizing_y(Sizing::Fixed(100.0))
                            .background_color(vec4(1.0, 1.0, 1.0, 1.0))
                            .draw(layouter);
                        UIElement::new().sizing_x(Sizing::Grow).draw(layouter);
                        UIElement::new()
                            .sizing_x(Sizing::Fixed(100.0))
                            .sizing_y(Sizing::Fixed(100.0))
                            .background_color(vec4(1.0, 1.0, 1.0, 1.0))
                            .draw(layouter);
                        UIElement::new()
                            .sizing_x(Sizing::Fixed(100.0))
                            .sizing_y(Sizing::Fixed(100.0))
                            .background_color(vec4(1.0, 1.0, 1.0, 1.0))
                            .draw(layouter);
                    });
                UIElement::new()
                    .sizing_x(Sizing::Grow)
                    .sizing_y(Sizing::Grow)
                    .background_color(vec4(0.1, 0.1, 0.1, 1.0))
                    .draw_with_children(layouter, |layouter| {
                        UIElement::new()
                            .sizing_x(Sizing::Percent(0.2))
                            .sizing_y(Sizing::Grow)
                            .background_color(vec4(0.6, 0.6, 0.6, 1.0))
                            .draw_with_children(layouter, |layouter| {});
                        UIElement::new()
                            .sizing_x(Sizing::Grow)
                            .sizing_y(Sizing::Grow)
                            .draw_with_children(layouter, |layouter| {});
                    });
                UIElement::new()
                    .sizing_x(Sizing::Grow)
                    .sizing_y(Sizing::Fit)
                    .background_color(vec4(0.2, 0.2, 0.2, 1.0))
                    .draw_with_children(layouter, |layouter| {
                        UIElement::new()
                            .sizing_x(Sizing::Fixed(100.0))
                            .sizing_y(Sizing::Fixed(100.0))
                            .background_color(vec4(0.2, 0.2, 0.2, 1.0))
                            .draw(layouter);
                    });
            });

        let ui_elements = self
            .layouter
            .layout_elements_fullscreen(ctx, &mut self.font);

        self.renderer.render(
            ctx,
            cache,
            screen_view,
            render::surface_format(ctx),
            ui_elements,
        );

        self.layouter.reset();

        CallbackResult::Continue
    }
}
