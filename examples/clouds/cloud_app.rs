use crate::cloud_renderer;
use gbase::render::{window, UniformBufferBuilder, Widget, BLUE, DARK_GREY, GRAY, GREEN, RED};
use gbase::Context;
use gbase::{
    collision::{self, Box3D},
    filesystem, glam, input, render, time, wgpu, winit,
};
use glam::{vec3, Quat, Vec3, Vec4Swizzles};
use std::fs::File;
use std::io::{self, Write};
use winit::dpi::PhysicalSize;
use winit::window::WindowBuilder;

const FONT_SIZE: f32 = 40.0;
const BTN_SIZE: f32 = 80.0;

#[derive(Debug, Clone, PartialEq, encase::ShaderType, serde::Serialize, serde::Deserialize)]
pub struct CloudParameters {
    // lights
    light_pos: Vec3,

    alpha_cutoff: f32,
    henyey_forw: f32,
    henyey_back: f32,
    henyey_dist: f32,
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
    cloud_params: CloudParameters,

    debug_mode: bool,
    enable_gizmos: bool,
    debug_msg: String,

    param_index: usize,
    load_param_index: bool,
    write_param_index: bool,
    params_changed: bool,
}

impl gbase::Callbacks for App {
    #[no_mangle]
    fn init_ctx() -> gbase::ContextBuilder {
        gbase::ContextBuilder::new()
            .log_level(gbase::LogLevel::Info)
            .window_builder(WindowBuilder::new().with_maximized(true))
            .vsync(true)
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
            alpha_cutoff: 0.7,
            henyey_forw: 0.7,
            henyey_back: 0.5,
            henyey_dist: 0.3,
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

            cloud_params: cloud_parameters,
            cloud_parameters_buffer,
            cloud_bb,
            cloud_bb_buffer,

            debug_mode: false,
            enable_gizmos: false,
            debug_msg: String::from("Ok"),
            param_index: 1,
            load_param_index: true,
            write_param_index: false,
            params_changed: false,
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
            self.debug_mode = !self.debug_mode;
        }

        if self.debug_mode || input::modifier_pressed(ctx, input::KeyModifier::LSuper) {
            self.ui(ctx);

            if input::key_just_pressed(ctx, input::KeyCode::KeyG) {
                self.enable_gizmos = !self.enable_gizmos;
            }

            self.save_load(ctx);
        } else {
            self.camera.flying_controls(ctx);
        }

        false
    }

    #[no_mangle]
    fn render(&mut self, ctx: &mut gbase::Context, screen_view: &wgpu::TextureView) -> bool {
        // write buffers
        self.camera_buffer.write(ctx, &self.camera.uniform(ctx));
        self.cloud_bb_buffer.write(ctx, &self.cloud_bb);
        self.cloud_parameters_buffer.write(ctx, &self.cloud_params);

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
            &render::Transform::from_pos(self.cloud_params.light_pos).with_scale(Vec3::ONE * 1.0),
            render::RED.xyz(),
        );
        self.gizmo_renderer.render(ctx, self.framebuffer.view_ref());
    }

    // save and load param configs
    fn save_load(&mut self, ctx: &Context) {
        //
        // Keyboard shortcuts
        //

        if input::key_just_pressed(ctx, input::KeyCode::KeyS) {
            self.write_param_index = true;
        }
        if input::key_just_pressed(ctx, input::KeyCode::KeyD) {
            self.params_changed = false;
        }

        if !self.params_changed {
            if input::key_just_pressed(ctx, input::KeyCode::Digit1) {
                self.param_index = 1;
                self.load_param_index = true;
            }
            if input::key_just_pressed(ctx, input::KeyCode::Digit2) {
                self.param_index = 2;
                self.load_param_index = true;
            }
            if input::key_just_pressed(ctx, input::KeyCode::Digit3) {
                self.param_index = 3;
                self.load_param_index = true;
            }
            if input::key_just_pressed(ctx, input::KeyCode::Digit4) {
                self.param_index = 4;
                self.load_param_index = true;
            }
            if input::key_just_pressed(ctx, input::KeyCode::Digit5) {
                self.param_index = 5;
                self.load_param_index = true;
            }

            if input::key_just_pressed(ctx, input::KeyCode::KeyL) {
                self.load_param_index = true;
            }
            if input::key_just_pressed(ctx, input::KeyCode::KeyS) {
                self.write_param_index = true;
            }
        }

        //
        // Saving and loading
        //

        let index = self.param_index;
        let file_name = format!("saved/cloud_params_{index}.txt");

        if self.write_param_index {
            let content =
                serde_json::to_string(&self.cloud_params).expect("could not serialze params");
            let mut file = File::create(&file_name).expect("could not open params file");
            file.write_all(content.as_bytes())
                .expect("could not write to params file");

            self.write_param_index = false;
            self.params_changed = false;
            println!("wrote params {}", self.param_index);
        }

        if self.load_param_index && !self.params_changed {
            let Ok(file) = File::open(&file_name) else {
                return;
            };
            let content = io::read_to_string(file).expect("could not read params file");
            let params = serde_json::from_str(&content).expect("could not deserialize params");
            self.cloud_params = params;

            self.load_param_index = false;
            self.params_changed = false;
            println!("loaded params {}", self.param_index);
        }
    }

    fn ui(&mut self, ctx: &Context) {
        let params_old = self.cloud_params.clone();

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
                .text_font_size(FONT_SIZE)
                .render(renderer);
            Widget::new()
                .text(format!("fps: {:.2}", time::fps(ctx)))
                .text_color(render::BLUE)
                .width(render::SizeKind::TextSize)
                .height(render::SizeKind::TextSize)
                .text_font_size(FONT_SIZE)
                .render(renderer);
            Widget::new()
                .text(format!("Params {}", self.param_index))
                .text_color(render::BLUE)
                .width(render::SizeKind::TextSize)
                .height(render::SizeKind::TextSize)
                .text_font_size(FONT_SIZE)
                .render(renderer);
            Widget::new()
                .width(render::SizeKind::ChildrenSum)
                .height(render::SizeKind::ChildrenSum)
                .direction(render::Direction::Row)
                .cross_axis_alignment(render::Alignment::Center)
                .gap(20.0)
                .layout(renderer, |renderer| {
                    for i in 1..=5 {
                        let param_index_btn = Widget::new()
                            .label(format!("param index {i}"))
                            .width(render::SizeKind::Pixels(BTN_SIZE))
                            .height(render::SizeKind::Pixels(BTN_SIZE))
                            .color(if i == self.param_index {
                                GREEN
                            } else if self.params_changed {
                                RED
                            } else {
                                GRAY
                            })
                            .button(ctx, renderer);
                        if !self.params_changed && param_index_btn.clicked {
                            self.param_index = i;
                            self.load_param_index = true;
                        }
                    }
                    let save_btn = Widget::new()
                        .label("params save")
                        .text("Save")
                        .width(render::SizeKind::TextSize)
                        .height(render::SizeKind::TextSize)
                        .text_font_size(FONT_SIZE)
                        .color(if self.params_changed { RED } else { GRAY })
                        .padding(10.0)
                        .button(ctx, renderer);
                    if save_btn.clicked {
                        self.write_param_index = true;
                    }
                    let discard_btn = Widget::new()
                        .label("params discard")
                        .text("Discard")
                        .width(render::SizeKind::TextSize)
                        .height(render::SizeKind::TextSize)
                        .text_font_size(FONT_SIZE)
                        .color(if self.params_changed { RED } else { GRAY })
                        .padding(10.0)
                        .button(ctx, renderer);
                    if discard_btn.clicked {
                        self.params_changed = false;
                    }
                });

            let gizmos_btn = Widget::new()
                .label("gizmos")
                .text("Gizmos")
                .width(render::SizeKind::TextSize)
                .height(render::SizeKind::TextSize)
                .text_font_size(FONT_SIZE)
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
                min: f32,
                max: f32,
                value: &mut f32,
                label: &str,
            ) {
                Widget::new()
                    .width(render::SizeKind::ChildrenSum)
                    .height(render::SizeKind::ChildrenSum)
                    .direction(render::Direction::Row)
                    .layout(renderer, |renderer| {
                        Widget::new()
                            .text(label)
                            .width(render::SizeKind::Pixels(250.0))
                            .height(render::SizeKind::TextSize)
                            .text_font_size(FONT_SIZE)
                            .render(renderer);
                        Widget::new()
                            .label(label)
                            .width(render::SizeKind::Pixels(500.0))
                            .height(render::SizeKind::Pixels(50.0))
                            .direction(render::Direction::Row)
                            .color(GRAY)
                            .slider_layout(ctx, renderer, min, max, value, |renderer, res| {
                                Widget::new()
                                    .width(render::SizeKind::PercentOfParent(res.pos))
                                    .render(renderer);
                                Widget::new()
                                    .width(render::SizeKind::Pixels(20.0))
                                    .height(render::SizeKind::PercentOfParent(1.0))
                                    .color(BLUE)
                                    .render(renderer);
                            });
                    });
            }

            f32_slider(
                ctx,
                renderer,
                -100.0,
                100.0,
                &mut self.cloud_params.light_pos.x,
                "light pos x",
            );
            f32_slider(
                ctx,
                renderer,
                -100.0,
                100.0,
                &mut self.cloud_params.light_pos.y,
                "light pos y",
            );
            f32_slider(
                ctx,
                renderer,
                -100.0,
                100.0,
                &mut self.cloud_params.light_pos.z,
                "light pos z",
            );
            f32_slider(
                ctx,
                renderer,
                0.0,
                1.0,
                &mut self.cloud_params.alpha_cutoff,
                "alpha cutoff",
            );
            f32_slider(
                ctx,
                renderer,
                0.0,
                1.0,
                &mut self.cloud_params.henyey_forw,
                "henyey forw",
            );
            f32_slider(
                ctx,
                renderer,
                0.0,
                1.0,
                &mut self.cloud_params.henyey_back,
                "henyey back",
            );
            f32_slider(
                ctx,
                renderer,
                0.0,
                1.0,
                &mut self.cloud_params.henyey_dist,
                "henyey dist",
            );
        });

        let params_changed = self.cloud_params != params_old;
        if params_changed {
            self.params_changed = true;
        }
    }
}
