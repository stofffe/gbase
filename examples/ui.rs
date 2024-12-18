use gbase::{
    collision::Quad,
    filesystem,
    render::{self, Button, BLACK, GRAY, GREEN, RED},
    time, Callbacks, Context,
};
use glam::vec2;

pub fn main() {
    gbase::ContextBuilder::new()
        .log_level(gbase::LogLevel::Info)
        .run_sync::<App>();
}

struct App {
    gui_renderer: render::GUIRenderer,

    toggle: bool,
}

impl Callbacks for App {
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

        let toggle = false;

        Self {
            gui_renderer,
            toggle,
        }
    }
    fn update(&mut self, ctx: &mut Context) -> bool {
        self.gui_renderer
            .quad(Quad::new(vec2(0.0, 0.0), vec2(1.0, 1.0)), render::WHITE);

        let fps_text = (1.0 / time::frame_time(ctx)).to_string();
        let text = "hello this is some text that is going to wrap a few times lol lol";

        let text_color = BLACK;
        // self.gui_renderer.quad(vec2(0.5,0.5), vec2(0.4,0.3), vec4(0.0,1.0,0.0,1.0));
        self.gui_renderer.text(
            &fps_text,
            Quad::new(vec2(0.005, 0.0), vec2(0.5, 0.5)),
            0.05,
            text_color,
            false,
        );
        self.gui_renderer.text(
            text,
            Quad::new(vec2(0.0, 0.3), vec2(0.5, 0.5)),
            0.05,
            text_color,
            true,
        );
        self.gui_renderer.text(
            text,
            Quad::new(vec2(0.0, 0.6), vec2(0.5, 0.5)),
            0.2,
            text_color,
            true,
        );

        // Idea: hash ui element and use as id?

        // self.gui_renderer.button(ctx, Quad::new( vec2(0.5, 0.5), vec2(0.1,0.1)), vec4(1.0,0.0,0.0,1.0));

        // button()
        // k

        // if btn("label1")
        //     .hover_color(RED)
        //     .size(vec2(0.1, 0.1))
        //     .orig(vec2(0.5, 0.5))
        //     .child(|bounds| {
        //         txt("label2")
        //             .text("hello")
        //             .fontsize(12)
        //             .build()
        //             .build()
        //             .build()
        //             .build()
        //             .build()
        //             .build()
        //             .build()
        //             .build()
        //     })
        //     .build()
        // {}
        Button::new()
            .dimension(vec2(0.1, 0.5))
            .origin(vec2(0.1, 0.1))
            .render(ctx, &mut self.gui_renderer);

        if self.gui_renderer.button(
            ctx,
            "test_btn",
            Quad::new(vec2(0.5, 0.3), vec2(0.1, 0.1)),
            GRAY,
        ) {
            log::warn!("testbtn pressed");
            self.toggle = !self.toggle;
        }
        if self.gui_renderer.button_text(
            ctx,
            "test_btn_txt",
            Quad::new(vec2(0.5, 0.1), vec2(0.1, 0.1)),
            GRAY,
            "test",
            0.05,
            false,
        ) {
            log::warn!("testbtntxt pressed");
            self.toggle = !self.toggle;
        }

        self.gui_renderer.quad(
            Quad::new(vec2(0.8, 0.5), vec2(0.1, 0.1)),
            if self.toggle { GREEN } else { RED },
        );

        false
    }
    fn render(&mut self, ctx: &mut Context, screen_view: &wgpu::TextureView) -> bool {
        self.gui_renderer.render(ctx, screen_view);
        false
    }
}
