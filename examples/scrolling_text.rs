use gbase::{filesystem, input, render, time, Callbacks, Context, ContextBuilder};
use glam::{vec2, vec4, Vec2, Vec4};
use winit::{keyboard::KeyCode, platform::macos::WindowBuilderExtMacOS, window::WindowBuilder};

#[pollster::main]
async fn main() {
    let (mut ctx, ev) = ContextBuilder::new()
        .window_builder(
            WindowBuilder::new()
                .with_maximized(true)
                .with_titlebar_hidden(true),
        )
        .build()
        .await;
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
        let text_pos = vec2(0.0, 0.1);

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

        self.gui_renderer
            .draw_quad(Vec2::ZERO, Vec2::ONE, Vec4::ONE);
        self.gui_renderer.draw_text(
            "BOOMBAAACLAT",
            self.text_pos,
            0.7,
            vec4(0.0, 0.0, 0.0, 1.0),
            None,
        );
        false
    }
    fn render(&mut self, ctx: &mut Context, screen_view: &wgpu::TextureView) -> bool {
        self.gui_renderer.render(ctx, screen_view);
        false
    }
}
//
// struct GifRecorder {
//     frames: Vec<Vec<u8>>,
//     width: u16,
//     height: u16,
//
//     output_buffer: wgpu::Buffer,
// }
//
// impl GifRecorder {
//     fn new(ctx: &Context) -> Self {
//         let size = render::window(ctx).inner_size();
//
//         let device = render::device(ctx);
//         let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
//             label: None,
//             size: (size.width * size.height * 4 * std::mem::size_of::<u8>() as u32) as u64,
//             usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
//             mapped_at_creation: false,
//         });
//
//         Self {
//             frames: Vec::new(),
//             width: size.width as u16,
//             height: size.height as u16,
//             output_buffer,
//         }
//     }
//
//     fn clear(&mut self) {
//         self.frames.clear();
//     }
//
//     fn record_frame(&mut self, frame: Vec<u8>) {
//         self.frames.push(frame);
//     }
//
//     fn export_gif(&mut self) -> Vec<u8> {
//         let mut encoder =
//             gif::Encoder::new(BufWriter::new(Vec::new()), self.width, self.height, &[]).unwrap();
//
//         for frame in self.frames.iter_mut() {
//             let mut rgba = image::RgbaImage::new(self.width as u32, self.height as u32);
//             rgba.copy_from_slice(frame.as_slice());
//
//             let mut gif_frame = gif::Frame::from_rgba(self.width, self.height, frame);
//             gif_frame.delay = 1;
//             encoder.write_frame(&gif_frame).unwrap();
//         }
//
//         self.clear();
//         encoder
//             .into_inner()
//             .expect("could not get bufwriter")
//             .into_inner()
//             .expect("could not get bytes") // extract inner vec
//     }
// }
