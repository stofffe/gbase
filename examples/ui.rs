use gbase::glam;
use gbase::render::{Widget, BLUE, GRAY, GREEN, RED, WHITE};
use gbase::wgpu;
use gbase::{
    filesystem,
    render::{self},
    Callbacks, Context,
};
use glam::{vec2, vec4};

pub fn main() {
    gbase::run_sync::<App>();
}

pub struct App {
    gui_renderer: render::GUIRenderer,
}

impl Callbacks for App {
    #[no_mangle]
    fn new(ctx: &mut Context) -> Self {
        let quads = 1000;
        let gui_renderer = render::GUIRenderer::new(
            ctx,
            wgpu::TextureFormat::Bgra8UnormSrgb,
            4 * quads,
            6 * quads,
            &filesystem::load_b!("fonts/font.ttf").unwrap(),
            render::DEFAULT_SUPPORTED_CHARS,
        );

        Self { gui_renderer }
    }

    #[no_mangle]
    fn render(&mut self, ctx: &mut Context, screen_view: &wgpu::TextureView) -> bool {
        let gr = &mut self.gui_renderer;
        gr.quad(vec2(0.0, 0.0), vec2(1.0, 1.0), vec4(0.05, 0.05, 0.07, 1.0));

        let outer = Widget::new()
            .label("outer")
            .size_y(render::SizeKind::Pixels(0.9))
            .size_x(render::SizeKind::Pixels(0.7))
            // .size(vec2(1.0, 1.0))
            // .padding(vec2(0.02, 0.02))
            .color(GRAY)
            .render(ctx, gr);

        // println!("outer {outer:?}");

        let header = Widget::new()
            .label("header")
            .parent(outer)
            .size_y(render::SizeKind::PercentOfParent(0.3))
            .size_x(render::SizeKind::Grow)
            .color(RED)
            .clickable()
            .render(ctx, gr);

        let body = Widget::new()
            .label("body")
            .parent(outer)
            .size_y(render::SizeKind::Grow)
            .color(GREEN)
            .clickable()
            .render(ctx, gr);

        let footer = Widget::new()
            .label("footer")
            .parent(outer)
            .size_y(render::SizeKind::Pixels(0.1))
            .color(BLUE)
            .clickable()
            .render(ctx, gr);

        // println!("header {header:?}");
        if header.clicked {
            println!("CLICK HEADER");
        }
        if body.clicked {
            println!("CLICK BODY");
        }

        // println!("{w}");

        self.gui_renderer.render(ctx, screen_view);
        false
    }
}
