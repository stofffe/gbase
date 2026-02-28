mod ui_layout;
mod ui_renderer;

use gbase::{
    asset,
    glam::{vec2, vec4},
    render, wgpu, CallbackResult, Callbacks, Context,
};

use crate::{
    ui_layout::{LayoutDirection, Padding, Sizing, UIElement, UILayouter},
    ui_renderer::UIRenderer,
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
        let screen_size = render::surface_size(ctx);
        let layouter = UILayouter::new(vec2(screen_size.width as f32, screen_size.height as f32));
        Self { renderer, layouter }
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

        // UIElement::new()
        //     .sizing_x(Sizing::Fixed(1600.0))
        //     .sizing_y(Sizing::Fit)
        //     .background_color(vec4(0.0, 0.0, 1.0, 1.0))
        //     .draw_with_children(&mut self.layouter, |layouter| {
        //         UIElement::new()
        //             .sizing_x(Sizing::Fixed(300.0))
        //             .sizing_y(Sizing::Fixed(300.0))
        //             .background_color(vec4(1.0, 0.8, 0.8, 1.0))
        //             .draw(layouter);
        //         UIElement::new()
        //             .sizing_x(Sizing::Grow)
        //             .sizing_y(Sizing::Fixed(500.0))
        //             .background_color(vec4(0.0, 0.8, 0.8, 1.0))
        //             .draw(layouter);
        //         UIElement::new()
        //             .sizing_x(Sizing::Grow)
        //             .sizing_y(Sizing::Fixed(300.0))
        //             .background_color(vec4(0.0, 0.0, 0.8, 1.0))
        //             .draw(layouter);
        //         UIElement::new()
        //             .sizing_x(Sizing::Fixed(100.0))
        //             .sizing_y(Sizing::Grow)
        //             .background_color(vec4(1.0, 1.0, 1.0, 1.0))
        //             .draw(layouter);
        //         UIElement::new()
        //             .sizing_x(Sizing::Fixed(100.0))
        //             .sizing_y(Sizing::Fixed(600.0))
        //             .background_color(vec4(1.0, 0.0, 0.0, 1.0))
        //             .draw(layouter);
        //     });

        // UIElement::new()
        //     .sizing_x(Sizing::Fixed(600.0))
        //     .sizing_y(Sizing::Fixed(1600.0))
        //     .background_color(vec4(0.0, 0.0, 1.0, 1.0))
        //     .layout_direction(ui_layout::LayoutDirection::TopToBottom)
        //     .draw_with_children(&mut self.layouter, |layouter| {
        //         UIElement::new()
        //             .sizing_x(Sizing::Fixed(500.0))
        //             .sizing_y(Sizing::Fixed(300.0))
        //             .background_color(vec4(1.0, 0.8, 0.8, 1.0))
        //             .draw(layouter);
        //         UIElement::new()
        //             .sizing_x(Sizing::Grow)
        //             .sizing_y(Sizing::Fixed(200.0))
        //             .background_color(vec4(0.0, 0.8, 0.8, 1.0))
        //             .draw(layouter);
        //         UIElement::new()
        //             .sizing_x(Sizing::Grow)
        //             .sizing_y(Sizing::Fixed(300.0))
        //             .background_color(vec4(0.0, 0.0, 0.8, 1.0))
        //             .draw(layouter);
        //         UIElement::new()
        //             .sizing_x(Sizing::Fixed(100.0))
        //             .sizing_y(Sizing::Fixed(300.0))
        //             .background_color(vec4(1.0, 0.0, 0.0, 1.0))
        //             .draw(layouter);
        //     });

        // UIElement::new()
        //     .sizing_x(ui_layout::Sizing::Fit)
        //     .sizing_y(ui_layout::Sizing::Fixed(500.0))
        //     .background_color(vec4(1.0, 0.0, 0.0, 0.0))
        //     .padding(Padding::new(32.0, 32.0, 32.0, 32.0))
        //     .draw_with_children(&mut self.layouter, |layouter| {
        //         UIElement::new()
        //             .sizing_x(ui_layout::Sizing::Fixed(500.0))
        //             .sizing_y(ui_layout::Sizing::Fixed(200.0))
        //             .background_color(vec4(0.0, 1.0, 0.0, 0.0))
        //             .draw(layouter);
        //         UIElement::new()
        //             .sizing_x(ui_layout::Sizing::Fixed(100.0))
        //             .sizing_y(ui_layout::Sizing::Fixed(300.0))
        //             .background_color(vec4(0.0, 0.0, 1.0, 0.0))
        //             .draw_with_children(layouter, |layouter| {});
        //     });
        // self.layouter.add_element(
        //     UIElement::new()
        //         .sizing_x(ui_layout::Sizing::Fit)
        //         .sizing_y(ui_layout::Sizing::Fixed(500.0))
        //         .background_color(vec4(1.0, 0.0, 0.0, 0.0)),
        //     |layouter| {
        //         layouter.add_element(
        //             UIElement::new()
        //                 .sizing_x(ui_layout::Sizing::Fixed(500.0))
        //                 .sizing_y(ui_layout::Sizing::Fixed(200.0))
        //                 .background_color(vec4(0.0, 1.0, 0.0, 0.0)),
        //             |layouter| {},
        //         );
        //         layouter.add_element(
        //             UIElement::new()
        //                 .sizing_x(ui_layout::Sizing::Fixed(100.0))
        //                 .sizing_y(ui_layout::Sizing::Fixed(300.0))
        //                 .background_color(vec4(0.0, 0.0, 1.0, 0.0)),
        //             |layouter| {},
        //         );
        //     },
        // );

        let ui_elements = self.layouter.layout_elements_fullscreen(ctx);

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
