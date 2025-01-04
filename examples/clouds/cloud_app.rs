use crate::cloud_renderer;
use encase::internal::BufferMut;
use gbase::render::{UniformBufferBuilder, Widget, BLUE, GRAY, GREEN, RED};
use gbase::Context;
use gbase::{filesystem, glam, input, render, time, wgpu, winit};
use glam::{vec3, Quat, Vec3, Vec4, Vec4Swizzles};
use std::fmt::format;
use std::fs::File;
use std::io::{self, Write};
use std::sync::mpsc;
use winit::dpi::PhysicalSize;
use winit::window::WindowBuilder;

const FONT_SIZE: f32 = 50.0;
const FONT_COLOR: Vec4 = render::WHITE;
const BTN_SIZE: f32 = 80.0;

#[derive(Debug, Clone, PartialEq, encase::ShaderType, serde::Serialize, serde::Deserialize)]
pub struct CloudParameters {
    light_pos: Vec3,
    bounds_min: Vec3,
    bounds_max: Vec3,

    alpha_cutoff: f32,
    density_cutoff: f32,
    henyey_forw: f32,
    henyey_back: f32,
    henyey_dist: f32,

    density_absorption: f32,
    sun_absorption: f32,
    transmittance_cutoff: f32,
    sun_light_mult: f32,
    cloud_sample_mult: f32,
}

impl Default for CloudParameters {
    fn default() -> Self {
        Self {
            light_pos: vec3(10.0, 0.0, 10.0),
            bounds_min: vec3(-5.0, -5.0, -5.0),
            bounds_max: vec3(5.0, 5.0, 5.0),

            alpha_cutoff: 0.0,
            density_cutoff: 0.0,
            henyey_forw: 0.7,
            henyey_back: 0.5,
            henyey_dist: 0.3,

            density_absorption: 6.0,
            sun_absorption: 3.0,

            transmittance_cutoff: 0.001,
            sun_light_mult: 15.0,
            cloud_sample_mult: 100.0,
        }
    }
}

pub struct App {
    framebuffer: render::FrameBuffer,
    framebuffer_renderer: render::TextureRenderer,
    depth_buffer: render::DepthBuffer,

    camera: render::PerspectiveCamera,
    camera_buffer: render::UniformBuffer<render::CameraUniform>,

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
    store_surface: bool,
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

        let cloud_parameters = CloudParameters::default();
        let cloud_parameters_buffer =
            UniformBufferBuilder::new(render::UniformBufferSource::Data(cloud_parameters.clone()))
                .build(ctx);
        let gizmo_renderer = render::GizmoRenderer::new(ctx, framebuffer.format(), &camera_buffer);
        let cloud_renderer = cloud_renderer::CloudRenderer::new(
            ctx,
            &framebuffer,
            &depth_buffer,
            &camera_buffer,
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

            debug_mode: false,
            enable_gizmos: false,
            debug_msg: String::from("Ok"),
            param_index: 1,
            load_param_index: true,
            write_param_index: false,
            params_changed: false,
            store_surface: false,
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
                &self.cloud_parameters_buffer,
            ) {
                println!("Ok");
                self.cloud_renderer = r;
                self.debug_msg = String::from("Ok")
            } else {
                println!("Fail");
                self.debug_msg = String::from("Fail");
            }
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

        // render to image
        if self.store_surface {
            self.store(ctx);
            self.store_surface = false;
        }

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

        self.framebuffer
            .resize(ctx, new_size.width, new_size.height);
        self.depth_buffer
            .resize(ctx, new_size.width, new_size.height);
        self.ui_renderer.resize(ctx, new_size);
    }
}

impl App {
    fn gizmos(&mut self, ctx: &Context) {
        let bb = render::Transform::from_scale(
            self.cloud_params.bounds_max - self.cloud_params.bounds_min,
        );
        self.gizmo_renderer.draw_cube(
            &render::Transform::new(bb.pos, Quat::IDENTITY, bb.scale),
            render::RED.xyz(),
        );
        self.gizmo_renderer.draw_cube(
            &render::Transform::from_pos(self.cloud_params.light_pos).with_scale(Vec3::ONE * 5.0),
            render::RED.xyz(),
        );
        self.gizmo_renderer.render(ctx, self.framebuffer.view_ref());
    }

    // save and load param configs
    fn save_load(&mut self, ctx: &Context) {
        //
        // Keyboard shortcuts
        //

        // if input::key_just_pressed(ctx, input::KeyCode::KeyS) {
        //     self.write_param_index = true;
        // }
        // if input::key_just_pressed(ctx, input::KeyCode::KeyD) {
        //     self.params_changed = false;
        //     self.load_param_index = true;
        // }

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
                println!("could not open file {file_name} aborting params load");
                return;
            };
            let content = io::read_to_string(file).expect("could not read params file");

            let params = match serde_json::from_str(&content) {
                Ok(params) => params,
                Err(err) => {
                    println!("could not deserialize params: {err}");
                    return;
                }
            };

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
                .text_color(FONT_COLOR)
                .width(render::SizeKind::TextSize)
                .height(render::SizeKind::TextSize)
                .text_font_size(FONT_SIZE)
                .render(renderer);
            Widget::new()
                .text(format!("fps: {:.2}", time::fps(ctx)))
                .text_color(FONT_COLOR)
                .width(render::SizeKind::TextSize)
                .height(render::SizeKind::TextSize)
                .text_font_size(FONT_SIZE)
                .render(renderer);
            Widget::new()
                .text(format!("Params {}", self.param_index))
                .text_color(FONT_COLOR)
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
                        .text_color(FONT_COLOR)
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
                        .text_color(FONT_COLOR)
                        .width(render::SizeKind::TextSize)
                        .height(render::SizeKind::TextSize)
                        .text_font_size(FONT_SIZE)
                        .color(if self.params_changed { RED } else { GRAY })
                        .padding(10.0)
                        .button(ctx, renderer);
                    if discard_btn.clicked {
                        self.params_changed = false;
                        self.load_param_index = true;
                    }
                });
            let store = Widget::new()
                .label("store")
                .text("Store")
                .text_color(FONT_COLOR)
                .width(render::SizeKind::TextSize)
                .height(render::SizeKind::TextSize)
                .text_font_size(FONT_SIZE)
                .color(render::GRAY)
                .button(ctx, renderer);
            if store.clicked {
                self.store_surface = true;
            }

            let gizmos_btn = Widget::new()
                .label("gizmos")
                .text("Gizmos")
                .text_color(FONT_COLOR)
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
                            .text_color(FONT_COLOR)
                            .width(render::SizeKind::Pixels(250.0))
                            .height(render::SizeKind::TextSize)
                            .text_font_size(FONT_SIZE)
                            .render(renderer);
                        Widget::new()
                            .label(label)
                            .width(render::SizeKind::Pixels(300.0))
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
                        Widget::new()
                            .text(format!("{value:.2}"))
                            .text_color(FONT_COLOR)
                            .width(render::SizeKind::TextSize)
                            .height(render::SizeKind::TextSize)
                            .text_font_size(FONT_SIZE)
                            .render(renderer);
                    });
            }

            let mut bounds_size = self.cloud_params.bounds_max - self.cloud_params.bounds_min;

            let p = &mut self.cloud_params;
            let sliders = [
                ("bounds x", 0.0, 500.0, &mut bounds_size.x),
                ("bounds y", 0.0, 100.0, &mut bounds_size.y),
                ("bounds z", 0.0, 500.0, &mut bounds_size.z),
                ("light x", -500.0, 500.0, &mut p.light_pos.x),
                ("light y", -500.0, 500.0, &mut p.light_pos.y),
                ("light z", -500.0, 500.0, &mut p.light_pos.z),
                ("henyey forw", 0.0, 1.0, &mut p.henyey_forw),
                ("henyey back", 0.0, 1.0, &mut p.henyey_back),
                ("henyey dist", 0.0, 1.0, &mut p.henyey_dist),
                ("sun light mult", 0.0, 30.0, &mut p.sun_light_mult),
                ("d absorption", 0.0, 3.0, &mut p.density_absorption),
                ("s absorption", 0.0, 10.0, &mut p.sun_absorption),
                ("noise zoom", 0.0, 300.0, &mut p.cloud_sample_mult),
                ("alpha cut", 0.0, 1.0, &mut p.alpha_cutoff),
                ("density cut", 0.0, 1.0, &mut p.density_cutoff),
            ];
            for (label, min, max, value) in sliders {
                f32_slider(ctx, renderer, min, max, value, label);
            }

            self.cloud_params.bounds_min = -bounds_size * 0.5;
            self.cloud_params.bounds_max = bounds_size * 0.5;
        });

        let params_changed = self.cloud_params != params_old;
        if params_changed {
            self.params_changed = true;
        }
    }

    /// store the image and the metadata
    fn store(&self, ctx: &mut Context) {
        let width = render::surface_size(ctx).width;
        let height = render::surface_size(ctx).height;

        // info
        let ms = time::frame_time(ctx);
        let mut metadata_file = File::create(format!("saved/image_{}_{}.info", width, height))
            .expect("could not create metadata file");
        metadata_file
            .write_all(ms.to_string().as_bytes())
            .expect("could not write to metadata file");

        // image
        let image_bytes = texture_to_buffer_gamma(ctx, self.framebuffer.view().clone());
        let image_buffer = gbase::image::ImageBuffer::<gbase::image::Rgba<u8>, _>::from_raw(
            width,
            height,
            image_bytes,
        )
        .expect("could not create image buffer");
        image_buffer
            .save(format!("saved/image_{}_{}.png", width, height))
            .expect("could not write to image file");

        println!("STORE")
    }
}

// render texture to Rgba8UnormSrgb and then load to PNG
fn texture_to_buffer_gamma(ctx: &mut Context, texture: render::ArcTextureView) -> Vec<u8> {
    let temp_framebuffer = render::FrameBufferBuilder::new()
        .screen_size(ctx)
        .format(wgpu::TextureFormat::Rgba8UnormSrgb)
        .build(ctx);
    render::TextureRenderer::new(ctx, wgpu::TextureFormat::Rgba8UnormSrgb).render(
        ctx,
        texture,
        temp_framebuffer.view_ref(),
    );
    texture_to_buffer_sync(ctx, temp_framebuffer.texture_ref())
}

fn texture_to_buffer_sync(ctx: &Context, texture: &wgpu::Texture) -> Vec<u8> {
    let pixel_size = std::mem::size_of::<u8>() as u32 * 4;
    let window_size = render::surface_size(ctx);
    let buffer_size = window_size.width * window_size.height * pixel_size;
    let read_back_buffer = render::RawBufferBuilder::new()
        .usage(wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST)
        .build(ctx, buffer_size);

    let mut encoder = render::EncoderBuilder::new().build(ctx);
    encoder.copy_texture_to_buffer(
        wgpu::ImageCopyTextureBase {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::ImageCopyBufferBase {
            buffer: &read_back_buffer.buffer(),
            layout: wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(window_size.width * pixel_size),
                rows_per_image: Some(window_size.height),
            },
        },
        wgpu::Extent3d {
            width: window_size.width,
            height: window_size.height,
            depth_or_array_layers: 1,
        },
    );
    let queue = render::queue(ctx);
    queue.submit(Some(encoder.finish()));

    read_buffer_sync(render::device(ctx), &read_back_buffer.buffer())
}

fn read_buffer_sync<T: bytemuck::AnyBitPattern>(
    device: &wgpu::Device,
    buffer: &wgpu::Buffer,
) -> Vec<T> {
    let buffer_slice = buffer.slice(..);
    let (sc, rc) = mpsc::channel();
    buffer_slice.map_async(wgpu::MapMode::Read, move |res| {
        sc.send(res).unwrap();
    });
    device.poll(wgpu::MaintainBase::Wait);
    let _ = rc.recv().unwrap();
    let data = buffer_slice.get_mapped_range();
    let result: Vec<T> = bytemuck::cast_slice(&data).to_vec();
    drop(data);
    buffer.unmap();
    result
}
