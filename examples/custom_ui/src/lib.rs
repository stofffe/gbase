mod ui_font;
mod ui_layout;
mod ui_renderer;

use crate::{
    ui_layout::{Sizing, UIElement, UILayouter},
    ui_renderer::UIRenderer,
};
use gbase::{
    asset,
    egui::{self, load::SizedTexture},
    glam::{vec4, Vec4},
    render::{self, SamplerBuilder},
    wgpu, CallbackResult, Callbacks, Context,
};

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub async fn run() {
    gbase::run::<App>().await;
}

const RED: Vec4 = vec4(1.0, 0.0, 0.0, 1.0);
const GREEN: Vec4 = vec4(0.0, 1.0, 0.0, 1.0);
const BLUE: Vec4 = vec4(0.0, 0.0, 1.0, 1.0);
const WHITE: Vec4 = vec4(1.0, 1.0, 1.0, 1.0);
const BLACK: Vec4 = vec4(0.0, 0.0, 0.0, 1.0);
const GREY: Vec4 = vec4(0.5, 0.5, 0.5, 1.0);

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
        let renderer = UIRenderer::new(
            ctx,
            cache,
            include_bytes!("../assets/fonts/font.ttf"),
            4 * 4096,
        );
        let layouter = UILayouter::new();
        Self { renderer, layouter }
    }

    #[no_mangle]
    fn render_egui(
        &mut self,
        ctx: &mut Context,
        _cache: &mut asset::AssetCache,
        egui_ctx: &mut gbase::egui_ui::EguiContext,
    ) -> CallbackResult {
        let image_view = render::TextureViewBuilder::new(self.renderer.font_atlas.clone());
        let texture_id =
            egui_ctx.register_wgpu_texture_cached(ctx, image_view, SamplerBuilder::new());

        egui::Window::new("font atlas").show(egui_ctx.ctx(), |ui| {
            ui.image(SizedTexture::new(texture_id, [512.0, 512.0]));
        });

        CallbackResult::Continue
    }

    #[no_mangle]
    fn render(
        &mut self,
        ctx: &mut Context,
        cache: &mut asset::AssetCache,
        screen_view: &wgpu::TextureView,
    ) -> CallbackResult {
        UIElement::new()
            .sizing_x(Sizing::Fixed(1200.0))
            .sizing_y(Sizing::Fit)
            .background_color(WHITE)
            .draw_with_children(&mut self.layouter, |layouter| {
                UIElement::new()
                    .text("Hello my name is not bobbyyy Hello my name is not bobbyyy Hello my name is not bobbyyy Hello my name is not bobbyyy Hello my name is not bobbyyy Hello my name is not bobbyyy")
                    .font_size(32)
                    .background_color(BLUE)
                    .draw(layouter);
                UIElement::new()
                    .sizing_x(Sizing::Fixed(400.0))
                    .sizing_y(Sizing::Grow)
                    .background_color(GREEN)
                    .draw(layouter);
                UIElement::new()
                    .text("Hello my name is bobbyyy")
                    .text("abc")
                    .font_size(128)
                    .background_color(BLUE)
                    .draw(layouter);
            });

        // UIElement::new()
        //     .sizing_x(Sizing::Grow)
        //     .sizing_y(Sizing::Grow)
        //     .layout_direction(LayoutDirection::TopToBottom)
        //     .draw_with_children(&mut self.layouter, |layouter| {
        //         UIElement::new()
        //             .sizing_x(Sizing::Grow)
        //             .sizing_y(Sizing::Fit)
        //             .padding(Padding::new(20.0, 20.0, 20.0, 20.0))
        //             .child_gap(20.0)
        //             .background_color(vec4(0.2, 0.2, 0.2, 1.0))
        //             .draw_with_children(layouter, |layouter| {
        //                 UIElement::new()
        //                     .background_color(vec4(1.0, 0.0, 1.0, 1.0))
        //                     .text("Hello brooo")
        //                     .font_size(32)
        //                     .draw(layouter);
        //                 UIElement::new()
        //                     .sizing_x(Sizing::Fixed(100.0))
        //                     .sizing_y(Sizing::Fixed(100.0))
        //                     .background_color(vec4(1.0, 1.0, 1.0, 1.0))
        //                     .draw(layouter);
        //                 UIElement::new().sizing_x(Sizing::Grow).draw(layouter);
        //                 UIElement::new()
        //                     .sizing_x(Sizing::Fixed(100.0))
        //                     .sizing_y(Sizing::Fixed(100.0))
        //                     .background_color(vec4(1.0, 1.0, 1.0, 1.0))
        //                     .draw(layouter);
        //                 UIElement::new()
        //                     .sizing_x(Sizing::Fixed(100.0))
        //                     .sizing_y(Sizing::Fixed(100.0))
        //                     .background_color(vec4(1.0, 1.0, 1.0, 1.0))
        //                     .draw(layouter);
        //             });
        //         UIElement::new()
        //             .sizing_x(Sizing::Grow)
        //             .sizing_y(Sizing::Grow)
        //             .background_color(vec4(0.1, 0.1, 0.1, 1.0))
        //             .draw_with_children(layouter, |layouter| {
        //                 UIElement::new()
        //                     .sizing_x(Sizing::Percent(0.2))
        //                     .sizing_y(Sizing::Grow)
        //                     .background_color(vec4(0.6, 0.6, 0.6, 1.0))
        //                     .draw_with_children(layouter, |layouter| {});
        //                 UIElement::new()
        //                     .sizing_x(Sizing::Grow)
        //                     .sizing_y(Sizing::Grow)
        //                     .draw_with_children(layouter, |layouter| {});
        //             });
        //         UIElement::new()
        //             .sizing_x(Sizing::Grow)
        //             .sizing_y(Sizing::Fit)
        //             .background_color(vec4(0.2, 0.2, 0.2, 1.0))
        //             .draw_with_children(layouter, |layouter| {
        //                 UIElement::new()
        //                     .sizing_x(Sizing::Fixed(100.0))
        //                     .sizing_y(Sizing::Fixed(100.0))
        //                     .background_color(vec4(0.2, 0.2, 0.2, 1.0))
        //                     .draw(layouter);
        //             });
        //     });

        let ui_elements = self
            .layouter
            .layout_elements_fullscreen(ctx, &mut self.renderer);

        self.renderer.render(
            ctx,
            cache,
            screen_view,
            render::surface_format(ctx),
            self.layouter.elements(),
        );

        self.layouter.reset();

        CallbackResult::Continue
    }
}
