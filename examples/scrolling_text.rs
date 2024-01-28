use gbase::{filesystem, input, render, time, Callbacks, Context, ContextBuilder};
use glam::{vec2, vec4, Vec2, Vec4};
use winit::keyboard::KeyCode;

#[pollster::main]
async fn main() {
    let (mut ctx, ev) = ContextBuilder::new().build().await;
    let app = App::new(&mut ctx).await;
    gbase::run(app, ctx, ev).await;
}

struct App {
    text_pos: Vec2,

    gui_renderer: render::GUIRenderer,
}

impl App {
    async fn new(ctx: &mut Context) -> Self {
        let gui_renderer = render::GUIRenderer::new(
            ctx,
            1000 * 4,
            1000 * 6,
            &filesystem::load_bytes(ctx, "font.ttf").await.unwrap(),
            // &filesystem::load_bytes(ctx, "font2.otf").await.unwrap(),
            render::DEFAULT_SUPPORTED_CHARS_SE,
        )
        .await;
        let text_pos = vec2(0.0, 0.0);
        Self {
            gui_renderer,
            text_pos,
        }
    }
}

impl Callbacks for App {
    fn update(&mut self, ctx: &mut Context) -> bool {
        let dt = time::delta_time(ctx);
        if input::key_pressed(ctx, KeyCode::Space) {
            self.text_pos.x -= dt * 2.0;
        }
        if input::key_just_pressed(ctx, KeyCode::KeyR) {
            self.text_pos.x = 1.0;
        }

        self.gui_renderer.draw_text(
            self.text_pos,
            vec2(10.0, 1.0),
            0.8,
            vec4(1.0, 1.0, 1.0, 1.0),
            "Whack på slack",
        );
        false
    }
    fn render(&mut self, ctx: &mut Context, screen_view: &wgpu::TextureView) -> bool {
        self.gui_renderer.render(ctx, screen_view);
        false
    }
}
