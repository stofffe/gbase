use gbase::glam;
use gbase::render::BLUE;
use gbase::wgpu;
use gbase::{
    collision::Quad,
    filesystem,
    render::{self, Button, BLACK, GRAY, GREEN, RED},
    time, Callbacks, Context,
};
use glam::{vec2, vec4};

pub fn main() {
    gbase::run_sync::<App>();
}

pub struct App {
    gui_renderer: render::GUIRenderer,

    toggle: bool,
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

        let toggle = false;

        Self {
            gui_renderer,
            toggle,
        }
    }

    #[no_mangle]
    fn update(&mut self, ctx: &mut Context) -> bool {
        self.gui_renderer
            .quad(Quad::new(vec2(0.0, 0.0), vec2(1.0, 1.0)), render::WHITE);

        let fps_text = (1.0 / time::frame_time(ctx)).to_string();
        let text = "hello this is some text that is going to wrap a few times lol lol";

        let text_color = BLACK;

        let gr = &mut self.gui_renderer;
        // self.gui_renderer.quad(vec2(0.5,0.5), vec2(0.4,0.3), vec4(0.0,1.0,0.0,1.0));
        gr.text(
            &fps_text,
            Quad::new(vec2(0.005, 0.0), vec2(0.5, 0.5)),
            0.05,
            text_color,
            false,
        );
        gr.text(
            text,
            Quad::new(vec2(0.0, 0.3), vec2(0.5, 0.5)),
            0.05,
            text_color,
            true,
        );
        gr.text(
            text,
            Quad::new(vec2(0.0, 0.6), vec2(0.5, 0.5)),
            0.2,
            text_color,
            true,
        );

        let blue_btn = Button::new()
            .label("blue")
            .dimension(vec2(0.2, 0.2))
            .origin(vec2(0.7, 0.1))
            .color(BLUE)
            .render(ctx, gr);

        if blue_btn.clicked {
            println!("blue clicked");
        }

        if gr.check_last_hot(gr.get_id("blue")) || gr.check_last_hot(gr.get_id("red")) {
            let red_btn = Button::new_with_parent(blue_btn)
                .label("red")
                .dimension(vec2(0.1, 0.1))
                .origin(vec2(0.0, 0.2))
                .color(RED)
                .render(ctx, gr);
            if red_btn.clicked {
                println!("red clicked");
            }
        }

        let green_btn = Button::new_with_parent(blue_btn)
            .label("green")
            .dimension(vec2(0.05, 0.05))
            .color(GREEN)
            .render(ctx, gr);
        if green_btn.clicked {
            println!("green clicked");
        }

        // 1.
        // Button::new()
        //     .label("blue")
        //     .dimension(vec2(0.1, 0.1))
        //     .origin(vec2(0.8, 0.1))
        //     .color(BLUE)
        //     .render_with_children(ctx, gr, |gr, b, c| {
        //         if c {
        //             println!("blue clicked");
        //         }
        //         Button::new_with_parent(b)
        //             .label("red")
        //             .dimension(vec2(0.05, 0.05))
        //             .color(RED)
        //             .render_with_children(ctx, gr, |gr, b, c| {
        //                 if c {
        //                     println!("red clicked");
        //                 }
        //                 Button::new_with_parent(b)
        //                     .label("green")
        //                     .dimension(vec2(0.025, 0.025))
        //                     .color(GREEN)
        //                     .render_with_children(ctx, gr, |_, _, c| {
        //                         if c {
        //                             println!("green clicked");
        //                         }
        //                     });
        //             });
        //     });

        // 2.
        // let blue_btn = Button::new()
        //     .dimension(vec2(0.2, 0.2))
        //     .origin(vec2(0.7, 0.1))
        //     .color(BLUE);
        // let red_btn = Button::new().dimension(vec2(0.1, 0.1)).color(RED);
        // let green_btn = Button::new().dimension(vec2(0.05, 0.05)).color(GREEN);
        //
        // blue_btn.render_with_children(ctx, gr, |gr, b, c| {
        //     red_btn
        //         .origin(b.origin)
        //         .render_with_children(ctx, gr, |gr, b, c| {
        //             green_btn.origin(b.origin).render(ctx, gr);
        //         });
        // });

        if gr.button(
            ctx,
            "topbtn",
            Quad::new(vec2(0.5, 0.25), vec2(0.1, 0.1)),
            GRAY,
        ) {
            println!("top");
            gbase::log::warn!("testbtn pressed");
            self.toggle = !self.toggle;
        }
        if gr.button_text(
            ctx,
            "botbtn",
            Quad::new(vec2(0.5, 0.3), vec2(0.1, 0.1)),
            vec4(0.3, 0.3, 0.3, 1.0),
            "test",
            0.05,
            false,
        ) {
            println!("bot");
            gbase::log::warn!("testbtntxt pressed");
            self.toggle = !self.toggle;
        }

        gr.quad(
            Quad::new(vec2(0.8, 0.5), vec2(0.1, 0.1)),
            if self.toggle { GREEN } else { RED },
        );

        false
    }

    #[no_mangle]
    fn render(&mut self, ctx: &mut Context, screen_view: &wgpu::TextureView) -> bool {
        self.gui_renderer.render(ctx, screen_view);
        false
    }
}
