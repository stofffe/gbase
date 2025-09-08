use gbase::{filesystem, input, render, wgpu, winit::dpi::PhysicalSize, CallbackResult, Callbacks, Context};
use gbase_utils::{Alignment, Direction, SizeKind, Widget, BLUE, GRAY, GREEN, RED, WHITE};

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub async fn run() {
    gbase::run::<App>().await;
}

pub struct App {
    gui_renderer: gbase_utils::GUIRenderer,

    health: f32,
}

impl Callbacks for App {
    #[no_mangle]
    fn init_ctx() -> gbase::ContextBuilder {
        gbase::ContextBuilder::new().vsync(true).device_features(wgpu::Features::TIMESTAMP_QUERY)
    }
    #[no_mangle]
    fn new(ctx: &mut Context, _cache: &mut gbase::asset::AssetCache) -> Self {
        let gui_renderer = gbase_utils::GUIRenderer::new(
            ctx,
            1000,
            &filesystem::load_b!("fonts/times.ttf").unwrap(),
            gbase_utils::DEFAULT_SUPPORTED_CHARS,
        );

        Self {
            gui_renderer,
            health: 0.4,
        }
    }

    #[no_mangle]
    fn render(&mut self, ctx: &mut Context,cache: &mut gbase::asset::AssetCache, screen_view: &wgpu::TextureView) -> CallbackResult {
        if input::key_just_pressed(ctx, input::KeyCode::KeyR) {
            render::clear_cache(ctx);
            *self = Self::new(ctx, cache);
        }

        let renderer = &mut self.gui_renderer;

        let outer = Widget::new()
            .label("outer")
            .width(SizeKind::PercentOfParent(1.0))
            .height(SizeKind::PercentOfParent(1.0))
            .direction(Direction::Column)
            .main_axis_alignment(Alignment::Center)
            .cross_axis_alignment(Alignment::Center)
            .gap(20.0)
            .padding(20.0);
        outer.layout(renderer, |renderer| {
            let slider_row = Widget::new()
                .label("slider row")
                .height(SizeKind::Pixels(100.0))
                .width(SizeKind::ChildrenSum)
                .gap(20.0)
                .cross_axis_alignment(Alignment::Center)
                .direction(Direction::Row);
            slider_row.layout(renderer, |renderer| {
                Widget::new()
                    .text("health")
                    .text_color(WHITE)
                    .height(SizeKind::TextSize)
                    .width(SizeKind::TextSize)
                    .text_font_size(60.0)
                    .render(renderer);
                let slider = Widget::new()
                    .label("slider")
                    .color(GRAY)
                    .border_radius(10.0)
                    .height(SizeKind::Pixels(100.0))
                    .width(SizeKind::Pixels(500.0))
                    .direction(Direction::Row);
                slider.slider_layout(
                    ctx,
                    renderer,
                    0.0,
                    200.0,
                    &mut self.health,
                    |renderer, res| {
                        Widget::new()
                            .width(SizeKind::PercentOfParent(res.pos))
                            .render(renderer);
                        Widget::new()
                            .width(SizeKind::Pixels(10.0))
                            .height(SizeKind::Grow)
                            .color(BLUE)
                            .border_radius(5.0)
                            .render(renderer);
                    },
                );
                Widget::new()
                    .text(format!("({:.3})", self.health))
                    .text_color(WHITE)
                    .color(RED)
                    .width(SizeKind::TextSize)
                    .height(SizeKind::TextSize)
                    .text_font_size(60.0)
                    .render(renderer);
            });

            Widget::new()
                .label("playbtn")
                .color(BLUE)
                .height(SizeKind::ChildrenSum)
                .width(SizeKind::ChildrenSum)
                .border_radius(20.0)
                .padding(20.0)
                .button_layout(ctx, renderer, |renderer, _| {
                    Widget::new()
                        .width(SizeKind::TextSize)
                        .height(SizeKind::TextSize)
                        .text_font_size(50.0)
                        .text("Play")
                        .render(renderer);
                });

            let button_row = Widget::new()
                .height(SizeKind::Pixels(100.0))
                .width(SizeKind::ChildrenSum)
                .gap(20.0)
                .cross_axis_alignment(Alignment::Center)
                .direction(Direction::Row);
            button_row.layout(renderer, |renderer| {
                if Widget::new()
                    .label("btn")
                    .text("reset health")
                    .text_color(WHITE)
                    .width(SizeKind::TextSize)
                    .height(SizeKind::TextSize)
                    .text_font_size(60.0)
                    .button(ctx, renderer).clicked 
                {
                    self.health = 0.0;
                };
                if Widget::new()
                    .label("reset health")
                    .width(SizeKind::Pixels(200.0))
                    .border_radius(20.0)
                    .color(BLUE)
                    .button(ctx, renderer)
                    .clicked
                {
                    self.health = 0.0;
                }
            });

            let txt_row = Widget::new()
                .height(SizeKind::ChildrenSum)
                .width(SizeKind::PercentOfParent(1.0))
                .margin(40.0)
                .padding(20.0)
                .direction(Direction::Row);
            txt_row.layout(renderer, |renderer| {
                Widget::new()
                    .text(
                        "(Love is a song that never f(1+1) ends Life may be swift and fleeting Hope may die Yet love's beautiful music Comes each day like the dawn Love is a song that never ends One simple theme repeating Like the voice of a heavenly choir Love's sweet music flows on Like the voice of a heavenly choir Love's sweet music flows on Wake up. - What now? - Wake up, Friend Owl. What's going on around here? - Wake up. - lt's happened.)"
                    )
                    // .text("SAdASSDadasds")
                    .color(BLUE)
                    .text_color(WHITE)
                    .width(SizeKind::TextSize)
                    .height(SizeKind::TextSize)
                    .text_font_size(80.0)
                    // .margin(20.0)
                    .padding(20.0)
                    .text_wrap(true)
                    .label("textbtn")
                    .button(ctx, renderer);
            });

            Widget::new()
                .color(GREEN)
                .height(SizeKind::Grow)
                .width(SizeKind::PercentOfParent(1.0))
                .render(renderer);
        });

        // self.gui_renderer.display_debug_info(ctx);
        self.gui_renderer.render(ctx, screen_view, render::surface_format(ctx));
        CallbackResult::Continue
    }

    #[no_mangle]
    fn resize(&mut self, ctx: &mut Context,_cache: &mut gbase::asset::AssetCache, new_size: PhysicalSize<u32>)->CallbackResult {
        self.gui_renderer.resize(ctx, new_size);
        CallbackResult::Continue
    }
}

#[no_mangle]
fn hot_reload() {
    App::init_ctx().init_logging();
}
