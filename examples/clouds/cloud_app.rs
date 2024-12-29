use crate::cloud_renderer;
use gbase::render::Widget;
use gbase::Context;
use gbase::{
    collision::{self, Box3D},
    filesystem, glam, input, render, time, wgpu, winit,
};
use glam::{vec3, Quat, Vec4Swizzles};
use winit::dpi::PhysicalSize;
use winit::window::WindowBuilder;

pub struct App {
    framebuffer: render::FrameBuffer,
    framebuffer_renderer: render::TextureRenderer,
    depth_buffer: render::DepthBuffer,

    camera: render::PerspectiveCamera,
    camera_buffer: render::UniformBuffer<render::CameraUniform>,
    cloud_bb: collision::Box3D,
    cloud_bb_buffer: render::UniformBuffer<Box3D>,

    ui_renderer: render::GUIRenderer,
    gizmo_renderer: render::GizmoRenderer,
    cloud_renderer: cloud_renderer::CloudRenderer,

    show_fps: bool,
    debug_msg: String,
}

impl gbase::Callbacks for App {
    #[no_mangle]
    fn init_ctx() -> gbase::ContextBuilder {
        gbase::ContextBuilder::new()
            .log_level(gbase::LogLevel::Info)
            .window_builder(WindowBuilder::new().with_maximized(true))
            .vsync(false)
    }

    #[no_mangle]
    fn new(ctx: &mut gbase::Context) -> Self {
        let framebuffer = render::FrameBufferBuilder::new()
            .screen_size(ctx)
            .build(ctx);
        let depth_buffer = render::DepthBufferBuilder::new()
            .screen_size(ctx)
            .build(ctx);
        let framebuffer_renderer =
            render::TextureRenderer::new(ctx, render::surface_config(ctx).format);

        let mut camera = render::PerspectiveCamera::new();
        camera.pos = vec3(0.0, 0.0, 5.0);
        let camera_buffer =
            render::UniformBufferBuilder::new(render::UniformBufferSource::Empty).build(ctx);
        let ui_renderer = render::GUIRenderer::new(
            ctx,
            framebuffer.format(),
            1024,
            &filesystem::load_b!("fonts/font.ttf").unwrap(),
            render::DEFAULT_SUPPORTED_CHARS,
        );
        let cloud_bb = collision::Box3D::new(vec3(0.0, 0.0, 0.0), vec3(1.0, 1.0, 1.0));
        let cloud_bb_buffer =
            render::UniformBufferBuilder::new(render::UniformBufferSource::Empty).build(ctx);
        let gizmo_renderer = render::GizmoRenderer::new(ctx, framebuffer.format(), &camera_buffer);
        let cloud_renderer = cloud_renderer::CloudRenderer::new(
            ctx,
            &framebuffer,
            &depth_buffer,
            &camera_buffer,
            &cloud_bb_buffer,
        )
        .expect("could not create cloud renderer");

        Self {
            framebuffer,
            depth_buffer,
            framebuffer_renderer,
            ui_renderer,
            gizmo_renderer,
            cloud_renderer,

            camera,
            camera_buffer,

            cloud_bb,
            cloud_bb_buffer,

            show_fps: false,
            debug_msg: String::from("test"),
        }
    }

    #[no_mangle]
    fn update(&mut self, ctx: &mut gbase::Context) -> bool {
        #[cfg(debug_assertions)]
        if input::key_just_pressed(ctx, input::KeyCode::KeyR) {
            if let Ok(r) = cloud_renderer::CloudRenderer::new(
                ctx,
                &self.framebuffer,
                &self.depth_buffer,
                &self.camera_buffer,
                &self.cloud_bb_buffer,
            ) {
                println!("Reloaded cloud renderer");
                self.cloud_renderer = r;
                self.debug_msg = String::from("Ok")
            } else {
                self.debug_msg = String::from("Fail");
            }
        }

        if input::key_just_pressed(ctx, input::KeyCode::KeyF) {
            self.show_fps = !self.show_fps;
        }

        self.ui(ctx);

        self.camera.flying_controls(ctx);

        false
    }

    #[no_mangle]
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

    #[no_mangle]
    fn resize(&mut self, ctx: &mut gbase::Context, new_size: PhysicalSize<u32>) {
        self.gizmo_renderer
            .resize(ctx, new_size.width, new_size.height);
        self.framebuffer.resize_screen(ctx);
        self.depth_buffer.resize_screen(ctx);
        self.ui_renderer.resize(ctx, new_size);
    }
}

impl App {
    fn ui(&mut self, ctx: &Context) {
        if !self.show_fps {
            return;
        }

        let renderer = &mut self.ui_renderer;
        let mut outer = Widget::new()
            .direction(render::Direction::Column)
            .width(render::SizeKind::PercentOfParent(1.0))
            .height(render::SizeKind::PercentOfParent(1.0));

        outer.layout(renderer, |renderer| {
            Widget::new()
                .text(format!("Shader: {}", self.debug_msg))
                .text_color(render::WHITE)
                .width(render::SizeKind::TextSize)
                .height(render::SizeKind::TextSize)
                .render(renderer);
            Widget::new()
                .text(format!("fps: {:.2}", time::fps(ctx)))
                .text_color(render::WHITE)
                .width(render::SizeKind::TextSize)
                .height(render::SizeKind::TextSize)
                .render(renderer);
        });
    }
}
