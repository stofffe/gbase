mod clouds;
mod noise;

use clouds::CloudRenderer;
use gbase::{
    collision::{self, Box3D, Quad},
    input,
    render::{self},
    time, Context, LogLevel,
};
use glam::{vec2, vec3, Quat, Vec4Swizzles};

#[pollster::main]
async fn main() {
    let (mut ctx, ev) = gbase::ContextBuilder::new()
        .window_builder(winit::window::WindowBuilder::new().with_maximized(true))
        .log_level(LogLevel::Info)
        .build()
        .await;
    let state = State::new(&mut ctx).await;
    gbase::run(state, ctx, ev);
}

struct State {
    framebuffer: render::FrameBuffer,
    framebuffer_renderer: render::TextureRenderer,
    depth_buffer: render::DepthBuffer,

    camera: render::PerspectiveCamera,
    camera_buffer: render::UniformBuffer<render::CameraUniform>,
    cloud_bb: collision::Box3D,
    cloud_bb_buffer: render::UniformBuffer<Box3D>,

    ui_renderer: render::GUIRenderer,
    gizmo_renderer: render::GizmoRenderer,
    cloud_renderer: CloudRenderer,

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

        let camera = render::PerspectiveCamera::new();
        let camera_buffer =
            render::UniformBufferBuilder::new(render::UniformBufferSource::Empty).build(ctx);
        let ui_renderer = render::GUIRenderer::new(
            ctx,
            framebuffer.format(),
            1024,
            1024,
            include_bytes!("../../assets/fonts/font.ttf"),
            render::DEFAULT_SUPPORTED_CHARS,
        )
        .await;
        let cloud_bb = collision::Box3D::new(vec3(0.0, 0.0, 0.0), vec3(1.0, 1.0, 1.0));
        let cloud_bb_buffer =
            render::UniformBufferBuilder::new(render::UniformBufferSource::Empty).build(ctx);
        let gizmo_renderer =
            render::GizmoRenderer::new(ctx, framebuffer.format(), &camera_buffer).await;
        let cloud_renderer = CloudRenderer::new(
            ctx,
            &framebuffer,
            &depth_buffer,
            &camera_buffer,
            &cloud_bb_buffer,
        );

        Self {
            framebuffer,
            depth_buffer,
            framebuffer_renderer,
            ui_renderer,
            gizmo_renderer,
            cloud_renderer,
            show_fps: false,

            camera,
            camera_buffer,

            cloud_bb,
            cloud_bb_buffer,
        }
    }
}

impl gbase::Callbacks for State {
    fn init(&mut self, _ctx: &mut gbase::Context) {
        self.camera.pos = vec3(0.0, 0.0, 5.0);
    }

    fn update(&mut self, ctx: &mut gbase::Context) -> bool {
        #[cfg(debug_assertions)]
        if input::key_just_pressed(ctx, input::KeyCode::KeyR) {
            self.cloud_renderer = CloudRenderer::new(
                ctx,
                &self.framebuffer,
                &self.depth_buffer,
                &self.camera_buffer,
                &self.cloud_bb_buffer,
            );
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
        self.cloud_bb_buffer.write(ctx, &self.cloud_bb);

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

        self.gizmo_renderer.draw_cube(
            &render::Transform::new(
                self.cloud_bb.origin,
                Quat::IDENTITY,
                self.cloud_bb.dimension,
            ),
            render::RED.xyz(),
        );

        self.gizmo_renderer.render(ctx, self.framebuffer.view_ref());
        self.ui_renderer.render(ctx, self.framebuffer.view_ref());
        self.framebuffer_renderer
            .render(ctx, self.framebuffer.view(), screen_view);
        // self.framebuffer_renderer
        //     .render(ctx, self.depth_buffer.framebuffer().view(), screen_view);
        false
    }

    fn resize(&mut self, ctx: &mut gbase::Context) {
        self.framebuffer.resize_screen(ctx);
        self.depth_buffer.resize_screen(ctx);
    }
}
