use gbase::{
    filesystem,
    render::{self, GUIRenderer},
    time, Callbacks, Context, ContextBuilder,
};
use glam::{vec2, vec4};

#[pollster::main]
pub async fn main() {
    let (ctx, ev) = ContextBuilder::new()
        .log_level(gbase::LogLevel::Warn)
        .vsync(false)
        .build()
        .await;
    let app = App::new(&ctx).await;
    gbase::run(app, ctx, ev).await;
}

struct App {
    gui_renderer: GUIRenderer,
}

impl App {
    async fn new(ctx: &Context) -> Self {
        let quads = 1000;
        let gui_renderer = GUIRenderer::new(
            ctx,
            4 * quads,
            6 * quads,
            &filesystem::load_bytes(ctx, "font2.otf").await.unwrap(),
            render::DEFAULT_SUPPORTED_CHARS,
        )
        .await;

        Self { gui_renderer }
    }
}

impl Callbacks for App {
    #[rustfmt::skip]
    fn update(&mut self, ctx: &mut Context) -> bool {
        self.gui_renderer.draw_quad(vec2(0.0, 0.0), vec2(1.0, 1.0), vec4(1.0, 1.0, 1.0, 1.0));

        let fps_text = (1.0 / time::frame_time(ctx)).to_string();

        let text_color = vec4(0.0,0.0,0.0,1.0);
        self.gui_renderer.draw_text(vec2(0.0, 0.0), vec2(0.5,0.2), 1.0, text_color, &fps_text);
        self.gui_renderer.draw_text(vec2(0.0,0.3), vec2(0.5,0.5), 1.0, text_color, "hello this is some text that is going to wrap a few times lol lol");
        self.gui_renderer.draw_text(vec2(0.0,0.6), vec2(0.5,0.5), 2.0, text_color, "hello this is some text that is going to wrap a few times lol lol");
        false
    }
    fn render(&mut self, ctx: &mut Context, screen_view: &wgpu::TextureView) -> bool {
        self.gui_renderer.render(ctx, screen_view);
        false
    }
}
