mod clouds;

use clouds::CloudRenderer;
use gbase::{
    collision::Quad,
    input,
    render::{self},
    time, Context, LogLevel,
};
use glam::{vec2, vec3};

#[pollster::main]
async fn main() {
    let (mut ctx, ev) = gbase::ContextBuilder::new()
        .log_level(LogLevel::Info)
        .vsync(false)
        .build()
        .await;
    let state = State::new(&mut ctx).await;
    gbase::run(state, ctx, ev);
}

struct State {
    framebuffer: render::FrameBuffer,
    framebuffer_renderer: render::TextureRenderer,
    depth_buffer: render::DepthBuffer,

    ui_renderer: render::GUIRenderer,

    cloud_renderer: CloudRenderer,

    camera: render::PerspectiveCamera,
    camera_buffer: render::UniformBuffer<render::CameraUniform>,

    show_fps: bool,
}

impl State {
    pub async fn new(ctx: &mut Context) -> Self {
        let framebuffer = render::FrameBufferBuilder::new()
            .screen_size(ctx)
            .build(ctx);
        let depth_buffer = render::DepthBufferBuilder::new()
            .screen_size(ctx)
            .build(ctx);
        let framebuffer_renderer =
            render::TextureRenderer::new(ctx, render::surface_config(ctx).format).await;

        let ui_renderer = render::GUIRenderer::new(
            ctx,
            framebuffer.format(),
            1024,
            1024,
            include_bytes!("../../assets/fonts/font.ttf"),
            render::DEFAULT_SUPPORTED_CHARS,
        )
        .await;

        let camera = render::PerspectiveCamera::new();
        let camera_buffer =
            render::UniformBufferBuilder::new(render::UniformBufferSource::Empty).build(ctx);
        let cloud_renderer =
            CloudRenderer::new(ctx, &framebuffer, &depth_buffer, &camera_buffer).await;

        Self {
            framebuffer,
            depth_buffer,
            framebuffer_renderer,
            ui_renderer,
            cloud_renderer,
            show_fps: false,

            camera,
            camera_buffer,
        }
    }
}

impl gbase::Callbacks for State {
    fn init(&mut self, _ctx: &mut gbase::Context) {
        self.camera.pos = vec3(0.0, 0.0, 5.0);
    }

    fn update(&mut self, ctx: &mut gbase::Context) -> bool {
        if input::key_just_pressed(ctx, input::KeyCode::KeyR) {
            self.cloud_renderer = pollster::block_on(CloudRenderer::new(
                ctx,
                &self.framebuffer,
                &self.depth_buffer,
                &self.camera_buffer,
            ));
        }

        if input::key_just_pressed(ctx, input::KeyCode::KeyF) {
            self.show_fps = !self.show_fps;
        }

        self.camera.flying_controls(ctx);

        false
    }

    fn render(&mut self, ctx: &mut gbase::Context, screen_view: &wgpu::TextureView) -> bool {
        // write buffers
        self.camera_buffer.write(ctx, &self.camera.uniform(ctx));

        // clear buffers
        self.framebuffer.clear(ctx, wgpu::Color::BLACK);
        self.depth_buffer.clear(ctx);

        // render
        self.cloud_renderer
            .render(ctx, self.framebuffer.view_ref(), &self.depth_buffer);

        if self.show_fps {
            let fps_txt = time::fps(ctx).to_string();
            self.ui_renderer.text(
                &fps_txt,
                Quad::new(vec2(0.0, 0.0), vec2(0.5, 0.1)),
                0.05,
                render::WHITE,
                false,
            );
        }

        self.ui_renderer.render(ctx, self.framebuffer.view_ref());
        self.framebuffer_renderer
            .render(ctx, self.framebuffer.view(), screen_view);
        false
    }

    fn resize(&mut self, ctx: &mut gbase::Context) {
        self.framebuffer.resize_screen(ctx);
        self.depth_buffer.resize_screen(ctx);
    }
}
