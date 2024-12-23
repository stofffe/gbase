use gbase::render::{Widget, BLUE, GRAY, GREEN, RED};
use gbase::wgpu;
use gbase::{
    filesystem,
    render::{self},
    Callbacks, Context,
};

pub fn main() {
    gbase::run_sync::<App>();
}

pub struct App {
    gui_renderer: render::GUIRenderer,
}

impl Callbacks for App {
    #[no_mangle]
    fn init_ctx() -> gbase::ContextBuilder {
        gbase::ContextBuilder::new()
    }
    #[no_mangle]
    fn new(ctx: &mut Context) -> Self {
        let max_quads = 10;
        let gui_renderer = render::GUIRenderer::new(
            ctx,
            wgpu::TextureFormat::Bgra8UnormSrgb,
            max_quads,
            &filesystem::load_b!("fonts/font.ttf").unwrap(),
            render::DEFAULT_SUPPORTED_CHARS,
        );

        Self { gui_renderer }
    }

    #[no_mangle]
    fn render(&mut self, ctx: &mut Context, screen_view: &wgpu::TextureView) -> bool {
        let gr = &mut self.gui_renderer;
        // gr.quad(vec2(0.0, 0.0), vec2(1.0, 1.0), vec4(0.05, 0.05, 0.07, 1.0));

        let outer = Widget::new()
            .label("outer")
            .size_main(render::SizeKind::PercentOfParent(1.0))
            .size_cross(render::SizeKind::PercentOfParent(1.0))
            .direction(render::Direction::Column)
            .color(GRAY)
            .padding(0.02)
            .render(ctx, gr);
        {
            let header = Widget::new()
                .label("header")
                .parent(outer)
                .size_main(render::SizeKind::PercentOfParent(0.2))
                .size_cross(render::SizeKind::PercentOfParent(1.0))
                .direction(render::Direction::Row)
                .gap(0.01)
                .padding(0.01)
                .color(RED)
                .margin(0.01)
                .render(ctx, gr);

            {
                let header_btn_size = 0.1;
                let a = Widget::new()
                    .label("a")
                    .parent(header)
                    .text("a")
                    .size_main(render::SizeKind::PercentOfParent(header_btn_size))
                    .size_cross(render::SizeKind::PercentOfParent(1.0))
                    .color(BLUE)
                    .clickable()
                    .render(ctx, gr);

                let c = Widget::new()
                    .label("c")
                    .parent(header)
                    .text("c")
                    .size_main(render::SizeKind::PercentOfParent(header_btn_size))
                    .size_cross(render::SizeKind::PercentOfParent(1.0))
                    .color(BLUE)
                    .clickable()
                    .render(ctx, gr);

                let d = Widget::new()
                    .label("d")
                    .parent(header)
                    .text("d")
                    .size_main(render::SizeKind::PercentOfParent(header_btn_size))
                    .size_cross(render::SizeKind::PercentOfParent(1.0))
                    .color(BLUE)
                    .clickable()
                    .render(ctx, gr);

                Widget::new()
                    .parent(header)
                    .size_main(render::SizeKind::Grow)
                    .render(ctx, gr);

                let b = Widget::new()
                    .label("b")
                    .parent(header)
                    .text("b")
                    .size_main(render::SizeKind::PercentOfParent(header_btn_size))
                    .size_cross(render::SizeKind::PercentOfParent(1.0))
                    .color(BLUE)
                    .clickable()
                    .render(ctx, gr);
            }
            let body = Widget::new()
                .label("body")
                .parent(outer)
                .size_main(render::SizeKind::Grow)
                .size_cross(render::SizeKind::PercentOfParent(1.0))
                .color(GREEN)
                .margin(0.01)
                .render(ctx, gr);

            let footer = Widget::new()
                .label("footer")
                .parent(outer)
                .size_main(render::SizeKind::Pixels(0.1))
                .size_cross(render::SizeKind::PercentOfParent(1.0))
                .color(BLUE)
                .margin(0.01)
                .render(ctx, gr);
        }

        // let outer = Widget::new()
        //     .label("outer")
        //     .size_main(render::SizeKind::PercentOfParent(1.0))
        //     .size_cross(render::SizeKind::PercentOfParent(1.0))
        //     .direction(render::Direction::Column)
        //     .color(GRAY)
        //     .render(ctx, gr);
        // {
        //     let header = Widget::new()
        //         .label("header")
        //         .parent(outer)
        //         .size_main(render::SizeKind::PercentOfParent(0.2))
        //         .size_cross(render::SizeKind::PercentOfParent(1.0))
        //         .direction(render::Direction::Row)
        //         .color(RED)
        //         .render(ctx, gr);
        //
        //     {
        //         let home_btn = Widget::new()
        //             .parent(header)
        //             .label("home")
        //             .text("Home")
        //             .color(GRAY)
        //             .size_main(render::SizeKind::Pixels(0.2))
        //             .size_cross(render::SizeKind::Pixels(0.2))
        //             .margin(0.02)
        //             .clickable()
        //             .render(ctx, gr);
        //
        //         Widget::new()
        //             .parent(header)
        //             .size_main(render::SizeKind::Grow)
        //             .render(ctx, gr);
        //
        //         let about_btn = Widget::new()
        //             .parent(header)
        //             .label("about")
        //             .text("About")
        //             .color(GRAY)
        //             .size_main(render::SizeKind::Pixels(0.2))
        //             .size_cross(render::SizeKind::Pixels(0.2))
        //             .margin(0.02)
        //             .clickable()
        //             .render(ctx, gr);
        //
        //         if home_btn.clicked {
        //             println!("HOME");
        //         }
        //         if about_btn.clicked {
        //             println!("ABOUT");
        //         }
        //     }
        //
        //     let body = Widget::new()
        //         .label("body")
        //         .parent(outer)
        //         .size_main(render::SizeKind::Grow)
        //         .size_cross(render::SizeKind::PercentOfParent(1.0))
        //         .color(GREEN)
        //         .render(ctx, gr);
        //
        //     let footer = Widget::new()
        //         .label("footer")
        //         .parent(outer)
        //         .size_main(render::SizeKind::Pixels(0.1))
        //         .size_cross(render::SizeKind::PercentOfParent(1.0))
        //         .color(BLUE)
        //         .render(ctx, gr);
        //
        //     if header.clicked {
        //         println!("CLICK HEADER");
        //     }
        //     if body.clicked {
        //         println!("CLICK BODY");
        //     }
        // }

        self.gui_renderer.render(ctx, screen_view);
        false
    }
}
