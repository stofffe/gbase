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
        let gui_renderer = render::GUIRenderer::new(
            ctx,
            wgpu::TextureFormat::Bgra8UnormSrgb,
            1000,
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
            .gap(20.0)
            .padding(20.0)
            .color(GRAY)
            .render(ctx, gr);
        {
            let header = Widget::new()
                .label("header")
                .parent(outer)
                .size_main(render::SizeKind::PercentOfParent(0.3))
                .size_cross(render::SizeKind::PercentOfParent(1.0))
                .direction(render::Direction::Row)
                .gap(20.0)
                .padding(20.0)
                .color(RED)
                .render(ctx, gr);
            {
                let header_btn_size = 150.0;
                let a = Widget::new()
                    .label("header1")
                    .parent(header)
                    .text("abc")
                    .size_main(render::SizeKind::Pixels(header_btn_size))
                    .size_cross(render::SizeKind::Pixels(header_btn_size))
                    .text_font_size(100.0)
                    .color(BLUE)
                    .clickable()
                    .render(ctx, gr);

                let b = Widget::new()
                    .label("header2")
                    .parent(header)
                    .text("b")
                    .size_main(render::SizeKind::Pixels(header_btn_size))
                    .size_cross(render::SizeKind::Pixels(header_btn_size))
                    .color(BLUE)
                    .clickable()
                    .render(ctx, gr);

                Widget::new()
                    .parent(header)
                    .size_main(render::SizeKind::Grow)
                    .render(ctx, gr);

                let c = Widget::new()
                    .label("header3")
                    .parent(header)
                    .text("c")
                    .size_main(render::SizeKind::Pixels(header_btn_size))
                    .size_cross(render::SizeKind::Pixels(header_btn_size))
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
                .render(ctx, gr);

            let footer = Widget::new()
                .label("footer")
                .parent(outer)
                .size_main(render::SizeKind::PercentOfParent(0.1))
                .size_cross(render::SizeKind::PercentOfParent(1.0))
                .color(BLUE)
                .render(ctx, gr);
        }

        self.gui_renderer.render(ctx, screen_view);
        false
    }

    #[no_mangle]
    fn resize(&mut self, ctx: &mut Context) {
        let new_size = render::surface_size(ctx);
        self.gui_renderer.resize(ctx, new_size);
    }
}
