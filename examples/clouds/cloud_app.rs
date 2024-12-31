use crate::cloud_renderer;
use gbase::render::{Transform, UniformBufferBuilder, Widget, BLUE, GRAY};
use gbase::Context;
use gbase::{
    collision::{self, Box3D},
    filesystem, glam, input, render, time, wgpu, winit,
};
use glam::{vec3, Quat, Vec3, Vec4Swizzles};
use winit::dpi::PhysicalSize;
use winit::window::WindowBuilder;

#[derive(Debug, Clone, encase::ShaderType)]
pub struct CloudParameters {
    pub light_pos: Vec3,
}

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

    cloud_parameters_buffer: render::UniformBuffer<CloudParameters>,
    cloud_parameters: CloudParameters,

    show_fps: bool,
    enable_gizmos: bool,
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
        camera.pos = vec3(0.0, 0.0, 15.0);
        let camera_buffer =
            render::UniformBufferBuilder::new(render::UniformBufferSource::Empty).build(ctx);
        let ui_renderer = render::GUIRenderer::new(
            ctx,
            framebuffer.format(),
            1024,
            &filesystem::load_b!("fonts/font.ttf").unwrap(),
            render::DEFAULT_SUPPORTED_CHARS,
        );

        let cloud_parameters = CloudParameters {
            light_pos: vec3(10.0, 0.0, 10.0),
        };
        let cloud_parameters_buffer =
            UniformBufferBuilder::new(render::UniformBufferSource::Data(cloud_parameters.clone()))
                .build(ctx);
        let cloud_bb = collision::Box3D::new(vec3(0.0, 0.0, 0.0), vec3(10.0, 5.0, 10.0));
        let cloud_bb_buffer =
            render::UniformBufferBuilder::new(render::UniformBufferSource::Empty).build(ctx);
        let gizmo_renderer = render::GizmoRenderer::new(ctx, framebuffer.format(), &camera_buffer);
        let cloud_renderer = cloud_renderer::CloudRenderer::new(
            ctx,
            &framebuffer,
            &depth_buffer,
            &camera_buffer,
            &cloud_bb_buffer,
            &cloud_parameters_buffer,
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

            cloud_parameters,
            cloud_parameters_buffer,
            cloud_bb,
            cloud_bb_buffer,

            show_fps: false,
            enable_gizmos: false,
            debug_msg: String::from("Ok"),
        }
    }

    #[no_mangle]
    fn update(&mut self, ctx: &mut gbase::Context) -> bool {
        #[cfg(debug_assertions)]
        if input::key_just_pressed(ctx, input::KeyCode::KeyR) {
            println!("Reload cloud renderer");
            if let Ok(r) = cloud_renderer::CloudRenderer::new(
                ctx,
                &self.framebuffer,
                &self.depth_buffer,
                &self.camera_buffer,
                &self.cloud_bb_buffer,
                &self.cloud_parameters_buffer,
            ) {
                println!("Ok");
                self.cloud_renderer = r;
                self.debug_msg = String::from("Ok")
            } else {
                println!("Fail");
                self.debug_msg = String::from("Fail");
            }
            self.cloud_bb = Box3D::new(Vec3::ZERO, vec3(5.0, 5.0, 5.0));
        }

        #[cfg(feature = "hot_reload")]
        if input::key_just_pressed(ctx, input::KeyCode::KeyR)
            && input::modifier_pressed(ctx, input::KeyModifier::LCtrl)
        {
            println!("Hot Restart");
            gbase::hot_reload::hot_restart(ctx);
        }

        if input::key_just_pressed(ctx, input::KeyCode::KeyF) {
            self.show_fps = !self.show_fps;
        }

        if !self.show_fps {
            self.camera.flying_controls(ctx);
        }

        if input::key_just_pressed(ctx, input::KeyCode::KeyG) {
            self.enable_gizmos = !self.enable_gizmos;
        }

        let mut dir = Vec3::ZERO;
        if input::key_pressed(ctx, input::KeyCode::ArrowUp) {
            dir.z -= 1.0;
        }
        if input::key_pressed(ctx, input::KeyCode::ArrowDown) {
            dir.z += 1.0;
        }
        if input::key_pressed(ctx, input::KeyCode::ArrowRight) {
            dir.x += 1.0;
        }
        if input::key_pressed(ctx, input::KeyCode::ArrowLeft) {
            dir.x -= 1.0;
        }
        if dir != Vec3::ZERO {
            self.cloud_parameters.light_pos += dir * time::delta_time(ctx) * 10.0;
        }

        self.ui(ctx);

        false
    }

    #[no_mangle]
    fn render(&mut self, ctx: &mut gbase::Context, screen_view: &wgpu::TextureView) -> bool {
        // write buffers
        self.camera_buffer.write(ctx, &self.camera.uniform(ctx));
        self.cloud_bb_buffer.write(ctx, &self.cloud_bb);
        self.cloud_parameters_buffer
            .write(ctx, &self.cloud_parameters);

        // clear buffers
        self.framebuffer.clear(
            ctx,
            wgpu::Color {
                r: 0.35,
                g: 0.85,
                b: 0.96,
                a: 1.0,
            },
        );
        self.depth_buffer.clear(ctx);

        // render
        self.cloud_renderer
            .render(ctx, self.framebuffer.view_ref(), &self.depth_buffer);

        if self.enable_gizmos {
            self.gizmos(ctx);
        }
        self.ui_renderer.render(ctx, self.framebuffer.view_ref());
        self.framebuffer_renderer
            .render(ctx, self.framebuffer.view(), screen_view);

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
    fn gizmos(&mut self, ctx: &Context) {
        let bb = self.cloud_bb.to_transform();
        self.gizmo_renderer.draw_cube(
            &render::Transform::new(bb.pos, Quat::IDENTITY, bb.scale),
            render::RED.xyz(),
        );
        self.gizmo_renderer.draw_cube(
            &render::Transform::from_pos(self.cloud_parameters.light_pos)
                .with_scale(Vec3::ONE * 1.0),
            render::RED.xyz(),
        );
        self.gizmo_renderer.render(ctx, self.framebuffer.view_ref());
    }

    fn ui(&mut self, ctx: &Context) {
        if !self.show_fps {
            return;
        }

        let renderer = &mut self.ui_renderer;
        let mut outer = Widget::new()
            .direction(render::Direction::Column)
            .width(render::SizeKind::PercentOfParent(1.0))
            .height(render::SizeKind::PercentOfParent(1.0))
            .gap(20.0)
            .padding(20.0);

        outer.layout(renderer, |renderer| {
            Widget::new()
                .text(format!("Shader: {}", self.debug_msg))
                .text_color(render::BLUE)
                .width(render::SizeKind::TextSize)
                .height(render::SizeKind::TextSize)
                .text_font_size(75.0)
                .render(renderer);
            Widget::new()
                .text(format!("fps: {:.2}", time::fps(ctx)))
                .text_color(render::BLUE)
                .width(render::SizeKind::TextSize)
                .height(render::SizeKind::TextSize)
                .text_font_size(75.0)
                .render(renderer);
            Widget::new()
                .text("Enable gizmos")
                .width(render::SizeKind::TextSize)
                .height(render::SizeKind::TextSize)
                .text_font_size(50.0)
                .render(renderer);
            let gizmos_btn = Widget::new()
                .label("gizmos")
                .width(render::SizeKind::Pixels(100.0))
                .height(render::SizeKind::Pixels(100.0))
                .color(if self.enable_gizmos {
                    render::GREEN
                } else {
                    render::GRAY
                })
                .button(ctx, renderer);
            if gizmos_btn.clicked {
                self.enable_gizmos = !self.enable_gizmos;
            }

            fn f32_slider(
                ctx: &Context,
                renderer: &mut render::GUIRenderer,
                value: &mut f32,
                label: &str,
            ) {
                Widget::new()
                    .text(label)
                    .width(render::SizeKind::TextSize)
                    .height(render::SizeKind::TextSize)
                    .text_font_size(50.0)
                    .render(renderer);
                Widget::new()
                    .label(label)
                    .width(render::SizeKind::Pixels(500.0))
                    .height(render::SizeKind::Pixels(100.0))
                    .direction(render::Direction::Row)
                    .color(GRAY)
                    .slider_layout(ctx, renderer, -100.0, 100.0, value, |renderer, res| {
                        Widget::new()
                            .width(render::SizeKind::PercentOfParent(res.pos))
                            .render(renderer);
                        Widget::new()
                            .width(render::SizeKind::Pixels(20.0))
                            .height(render::SizeKind::PercentOfParent(1.0))
                            .color(BLUE)
                            .render(renderer);
                    });
            }

            f32_slider(
                ctx,
                renderer,
                &mut self.cloud_parameters.light_pos.x,
                "light pos x",
            );
            f32_slider(
                ctx,
                renderer,
                &mut self.cloud_parameters.light_pos.y,
                "light pos y",
            );
            f32_slider(
                ctx,
                renderer,
                &mut self.cloud_parameters.light_pos.z,
                "light pos z",
            );
        });
    }
}
