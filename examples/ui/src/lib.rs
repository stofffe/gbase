use gbase::{filesystem, render, wgpu, winit::dpi::PhysicalSize, Callbacks, Context};
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
        gbase::ContextBuilder::new().vsync(true)
    }
    #[no_mangle]
    fn new(ctx: &mut Context) -> Self {
        let gui_renderer = gbase_utils::GUIRenderer::new(
            ctx,
            render::surface_format(ctx),
            1000,
            &filesystem::load_b!("fonts/font.ttf").unwrap(),
            gbase_utils::DEFAULT_SUPPORTED_CHARS,
        );

        Self {
            gui_renderer,
            health: 0.4,
        }
    }

    #[no_mangle]
    fn render(&mut self, ctx: &mut Context, screen_view: &wgpu::TextureView) -> bool {
        let renderer = &mut self.gui_renderer;

        let mut outer = Widget::new()
            .label("outer")
            .width(SizeKind::PercentOfParent(1.0))
            .height(SizeKind::PercentOfParent(1.0))
            .direction(Direction::Column)
            .main_axis_alignment(Alignment::Center)
            .cross_axis_alignment(Alignment::Center)
            .gap(20.0)
            .padding(20.0);
        outer.layout(renderer, |renderer| {
            let mut slider_row = Widget::new()
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
                let mut slider = Widget::new()
                    .label("slider")
                    .color(GRAY)
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
                            .render(renderer);
                    },
                );
                Widget::new()
                    .text(format!("{:.3}", self.health))
                    .text_color(WHITE)
                    .width(SizeKind::Pixels(200.0))
                    .color(RED)
                    .height(SizeKind::TextSize)
                    .text_font_size(60.0)
                    .render(renderer);
            });

            let mut button_row = Widget::new()
                .height(SizeKind::Pixels(100.0))
                .width(SizeKind::ChildrenSum)
                .gap(20.0)
                .cross_axis_alignment(Alignment::Center)
                .direction(Direction::Row);
            button_row.layout(renderer, |renderer| {
                Widget::new()
                    .text("reset health")
                    // .color(RED)
                    .text_color(WHITE)
                    .width(SizeKind::TextSize)
                    .height(SizeKind::TextSize)
                    .text_font_size(60.0)
                    .render(renderer);
                if Widget::new()
                    .label("btn")
                    .color(BLUE)
                    .margin(10.0)
                    .padding(10.0)
                    .button(ctx, renderer)
                    .clicked
                {
                    self.health = 0.0;
                }
            });

            let mut txt_row = Widget::new()
                .height(SizeKind::ChildrenSum)
                .width(SizeKind::PercentOfParent(1.0))
                .direction(Direction::Row);
            txt_row.layout(renderer, |renderer| {
                Widget::new()
                    .text(
                        "Love is a song that never ends Life may be swift and fleeting Hope may die Yet love's beautiful music Comes each day like the dawn Love is a song that never ends One simple theme repeating Like the voice of a heavenly choir Love's sweet music flows on Like the voice of a heavenly choir Love's sweet music flows on Wake up. - What now? - Wake up, Friend Owl. What's going on around here? - Wake up. - lt's happened."
                    )
                    .color(RED)
                    .text_color(WHITE)
                    .width(SizeKind::TextSize)
                    .height(SizeKind::TextSize)
                    .text_font_size(80.0)
                    .text_wrap(true)
                    .render(renderer);
            });

            Widget::new()
                .color(GREEN)
                .height(SizeKind::Grow)
                .width(SizeKind::PercentOfParent(1.0))
                .render(renderer);
        });

        // let outer = Widget::new()
        //     .width(render::SizeKind::PercentOfParent(1.0))
        //     .height(render::SizeKind::PercentOfParent(1.0))
        //     .direction(render::Direction::Column)
        //     .main_axis_alignment(render::Alignment::Center)
        //     .cross_axis_alignment(render::Alignment::Center)
        //     .gap(20.0)
        //     .padding(20.0)
        //     .color(BLACK);
        // let outer = outer.label("outer").layout(renderer);
        // {
        //     let slider_row = Widget::new()
        //         .parent(outer)
        //         .height(render::SizeKind::Pixels(100.0))
        //         .width(render::SizeKind::ChildrenSum)
        //         .cross_axis_alignment(render::Alignment::Center)
        //         .direction(render::Direction::Row)
        //         .layout(renderer);
        //     {
        //         Widget::new()
        //             .text("health")
        //             .parent(slider_row)
        //             .text_color(WHITE)
        //             .width(render::SizeKind::Pixels(200.0))
        //             .text_font_size(60.0)
        //             .layout(renderer);
        //         let slider = Widget::new()
        //             .label("slider")
        //             .parent(slider_row)
        //             .color(GRAY)
        //             .height(render::SizeKind::Pixels(100.0))
        //             .width(render::SizeKind::Pixels(500.0))
        //             .direction(render::Direction::Row)
        //             .slider(ctx, renderer, 0.0, 200.0, &mut self.health);
        //         {
        //             Widget::new()
        //                 .parent(slider)
        //                 .width(render::SizeKind::PercentOfParent(slider.pos))
        //                 .layout(renderer);
        //             Widget::new()
        //                 .parent(slider)
        //                 .width(render::SizeKind::Pixels(50.0))
        //                 .height(render::SizeKind::Grow)
        //                 .color(BLUE)
        //                 .layout(renderer);
        //         }
        //         Widget::new()
        //             .parent(slider_row)
        //             .text(format!("{:.3}", self.health))
        //             .text_color(WHITE)
        //             .width(render::SizeKind::Pixels(200.0))
        //             .text_font_size(60.0)
        //             .layout(renderer);
        //     }
        //     let button_row = Widget::new()
        //         .parent(outer)
        //         .height(render::SizeKind::Pixels(100.0))
        //         .width(render::SizeKind::ChildrenSum)
        //         .cross_axis_alignment(render::Alignment::Center)
        //         .direction(render::Direction::Row)
        //         .layout(renderer);
        //     {
        //         Widget::new()
        //             .text("reset health")
        //             .parent(button_row)
        //             .text_color(WHITE)
        //             .width(render::SizeKind::Pixels(400.0))
        //             .text_font_size(60.0)
        //             .layout(renderer);
        //         if Widget::new()
        //             .parent(button_row)
        //             .label("btn")
        //             .color(BLUE)
        //             .button(ctx, renderer)
        //             .clicked
        //         {
        //             self.health = 0.0;
        //         }
        //     }
        // }

        // let slider = Widget::new()
        //     .label("slider")
        //     .parent(outer)
        //     .color(GRAY)
        //     .height(render::SizeKind::Pixels(100.0))
        //     .width(render::SizeKind::Pixels(500.0))
        //     .slider(self.health, 0.0, 200.0)
        //     .direction(render::Direction::Row)
        //     .render(ctx, renderer);
        // if let Some(s) = slider.slider_value {
        //     self.health = s;
        // }
        // {
        // let handle_size = 50.0;
        // let slider_left = Widget::new()
        //     .parent(slider)
        //     .width(render::SizeKind::PercentOfParent(slider.slider_value))
        //     .render(ctx, renderer);
        // let slider_handle = Widget::new()
        //     .label("slider handle")
        //     .parent(slider)
        //     .color(BLUE)
        //     .width(render::SizeKind::Pixels(handle_size))
        //     .height(render::SizeKind::Grow)
        //     .render(ctx, renderer);
        // }

        // let outer = Widget::new()
        //     .label("outer")
        //     .width(render::SizeKind::PercentOfParent(1.0))
        //     .height(render::SizeKind::PercentOfParent(1.0))
        //     .direction(render::Direction::Column)
        //     .gap(20.0)
        //     .padding(20.0)
        //     .color(BLACK)
        //     .render(ctx, renderer);
        // {
        //     let header = Widget::new()
        //         .label("header")
        //         .parent(outer)
        //         .width(render::SizeKind::PercentOfParent(1.0))
        //         .height(render::SizeKind::ChildrenSum)
        //         .direction(render::Direction::Row)
        //         .main_axis_alignment(render::Alignment::Center)
        //         .cross_axis_alignment(render::Alignment::Center)
        //         .gap(20.0)
        //         .padding(20.0)
        //         .color(RED)
        //         .render(ctx, renderer);
        //     {
        //         let header1 = Widget::new()
        //             .label("header1")
        //             .parent(header)
        //             .width(render::SizeKind::Pixels(200.0))
        //             .height(render::SizeKind::ChildrenSum)
        //             .text_font_size(80.0)
        //             .padding(10.0)
        //             .gap(10.0)
        //             .color(BLUE)
        //             .render(ctx, renderer);
        //         {
        //             for _ in 0..5 {
        //                 Widget::new()
        //                     .parent(header1)
        //                     .width(render::SizeKind::Grow)
        //                     .height(render::SizeKind::Pixels(40.0))
        //                     .color(GRAY)
        //                     .render(ctx, renderer);
        //             }
        //         }
        //         Widget::new()
        //             .parent(header)
        //             .width(render::SizeKind::Grow)
        //             .render(ctx, renderer);
        //         let header2 = Widget::new()
        //             .label("header2")
        //             .parent(header)
        //             .width(render::SizeKind::Pixels(200.0))
        //             .height(render::SizeKind::Pixels(200.0))
        //             .text_font_size(100.0)
        //             .color(BLUE)
        //             .clickable()
        //             .render(ctx, renderer);
        //         let header3 = Widget::new()
        //             .label("header3")
        //             .parent(header)
        //             .width(render::SizeKind::Pixels(200.0))
        //             .height(render::SizeKind::Pixels(200.0))
        //             .text_font_size(100.0)
        //             .color(BLUE)
        //             .clickable()
        //             .render(ctx, renderer);
        //     }
        //
        //     let body = Widget::new()
        //         .label("body")
        //         .parent(outer)
        //         .width(render::SizeKind::Grow)
        //         .height(render::SizeKind::Grow)
        //         .direction(render::Direction::Row)
        //         .gap(20.0)
        //         .render(ctx, renderer);
        //
        //     {
        //         let sidebar = Widget::new()
        //             .label("sidebar")
        //             .parent(body)
        //             .width(render::SizeKind::Pixels(200.0))
        //             .height(render::SizeKind::Grow)
        //             .color(GRAY)
        //             .render(ctx, renderer);
        //
        //         let content = Widget::new()
        //             .label("content")
        //             .parent(body)
        //             .width(render::SizeKind::Grow)
        //             .height(render::SizeKind::Grow)
        //             .color(GRAY)
        //             .render(ctx, renderer);
        //     }
        //
        //     let footer = Widget::new()
        //         .label("footer")
        //         .parent(outer)
        //         .width(render::SizeKind::Grow)
        //         .height(render::SizeKind::Pixels(200.0))
        //         .color(BLUE)
        //         .render(ctx, renderer);
        // }

        self.gui_renderer.render(ctx, screen_view);
        false
    }

    #[no_mangle]
    fn resize(&mut self, ctx: &mut Context, new_size: PhysicalSize<u32>) {
        self.gui_renderer.resize(ctx, new_size);
    }
}
