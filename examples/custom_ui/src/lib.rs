mod ui_font;
mod ui_layout;
mod ui_renderer;
use crate::{
    ui_font::{FontTextMeasurer, UIFonts},
    ui_layout::{
        layouter::UILayouter,
        Padding,
        Sizing::{self, Fixed, Percent},
        UIElement,
    },
    ui_renderer::UIRenderer,
};
use gbase::{
    asset::{self, AssetCache},
    glam::{vec4, Vec4},
    render::{self, SamplerBuilder},
    wgpu, CallbackResult, Callbacks, Context,
};

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub fn run() {
    gbase::run::<App>();
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
    fonts: UIFonts,
}

impl Callbacks for App {
    #[no_mangle]
    fn init_ctx() -> gbase::ContextBuilder {
        gbase::ContextBuilder::new().vsync(true).assets_path(".")
    }
    #[no_mangle]
    fn new(ctx: &mut Context, cache: &mut asset::AssetCache) -> Self {
        let renderer = UIRenderer::new(ctx, cache, "assets/fonts/font.ttf", 256, 4 * 4096);
        let layouter = UILayouter::new();
        let fonts = UIFonts::new(
            renderer.font_handle.clone(),
            renderer.font_atlas_handle.clone(),
        );
        Self {
            renderer,
            layouter,
            fonts,
        }
    }

    #[no_mangle]
    fn render_egui(
        &mut self,
        ctx: &mut Context,
        _cache: &mut asset::AssetCache,
        egui_ctx: &mut gbase::egui_ui::EguiContext,
    ) -> CallbackResult {
        // tracing::info!("typeid for font loader {:?}", TypeId::of::<FontLoader>());

        let Ok(font_atlas) = _cache.get_asset(&self.renderer.font_atlas_handle).cloned() else {
            return CallbackResult::Continue;
        };

        let image_view = render::TextureViewBuilder::new(font_atlas.texture);
        let texture_id =
            egui_ctx.register_wgpu_texture_cached(ctx, image_view, SamplerBuilder::new());

        // egui::Window::new("font atlas").show(egui_ctx.ctx(), |ui| {
        //     ui.image(SizedTexture::new(texture_id, [512.0, 512.0]));
        // });

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
            .with_sizing_x(Sizing::Fixed(1200.0))
            .with_sizing_y(Sizing::Fit)
            .with_background_color(GREY)
            .with_padding(Padding::all(10.0))
            .with_child_gap(10.0)
            .draw_with_children(&mut self.layouter, |layouter| {
                UIElement::new()
                    .with_text(
                        "Hello     my name is not bobbyyy Hello my name is not bobbyyy Hello my name is not bobbyyy",
                    )
                    .with_font_size(64)
                    .with_padding(Padding::all(50.0))
                    .with_background_color(BLUE)
                    .draw(layouter);
                UIElement::new()
                    .with_sizing_x(Sizing::Fixed(400.0))
                    .with_sizing_y(Sizing::Grow)
                    .with_background_color(GREEN)
                    .draw(layouter);
                UIElement::new()
                    // .text("Hello my name is bobbyyy")
                    .with_text("ab cd 123")
                    .with_font_size(128)
                    .with_padding(Padding::all(20.0))
                    .with_background_color(BLUE)
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
        //                     // .font(times_new_roman_font_id)
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

        let mut text_measurer = FontTextMeasurer {
            fonts: &self.fonts,
            cache,
        };

        self.layouter
            .layout_elements_fullscreen(ctx, &mut text_measurer);

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

#[cfg(not(target_arch = "wasm32"))]
impl App {
    #[no_mangle]
    fn hot_reload(&mut self, _ctx: &mut Context, _cache: &mut AssetCache) {
        Self::init_ctx().init_logging();
    }
}
