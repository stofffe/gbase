use gbase::{filesystem, render, time, Callbacks, Context, ContextBuilder};
use glam::{vec2, vec4};

#[pollster::main]
pub async fn main() {
    let (mut ctx, ev) = ContextBuilder::new()
        .log_level(gbase::LogLevel::Warn)
        .vsync(false)
        .build()
        .await;
    let app = App::new(&mut ctx).await;
    gbase::run(app, ctx, ev);
}

struct App {
    gui_renderer: render::GUIRenderer,
}

impl App {
    async fn new(ctx: &mut Context) -> Self {
        let quads = 1000;
        let gui_renderer = render::GUIRenderer::new(
            ctx,
            wgpu::TextureFormat::Bgra8UnormSrgb,
            4 * quads,
            6 * quads,
            &filesystem::load_bytes(ctx, "fonts/font.ttf").await.unwrap(),
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
        let text = "hello this is some text that is going to wrap a few times lol lol";

        let text_color = vec4(0.0,0.0,0.0,1.0);
        self.gui_renderer.draw_quad(vec2(0.5,0.5), vec2(0.4,0.3), vec4(0.0,1.0,0.0,1.0));
        self.gui_renderer.draw_text(&fps_text,vec2(0.005, 0.0), 0.05, text_color,  None);
        self.gui_renderer.draw_text(text, vec2(0.0,0.3), 0.05, text_color, Some(0.5));
        self.gui_renderer.draw_text(text, vec2(0.0,0.6), 0.2, text_color, Some(0.5));
        false
    }
    fn render(&mut self, ctx: &mut Context, screen_view: &wgpu::TextureView) -> bool {
        self.gui_renderer.render(ctx, screen_view);
        false
    }
}
