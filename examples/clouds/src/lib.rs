mod cloud_renderer;
mod noise;

use gbase::{
    filesystem,
    glam::{uvec2, vec3, Quat, UVec2, Vec3, Vec4, Vec4Swizzles},
    input,
    render::{self, UniformBufferBuilder},
    time, tracing, wgpu,
    winit::{self, dpi::PhysicalSize, window::Window},
    Callbacks, Context,
};
use gbase_utils::{
    gaussian_filter, Alignment, Direction, SizeKind, Transform3D, Widget, BLUE, GRAY, GREEN, RED,
};
use std::{f32::consts::PI, fs, io::Write, sync::mpsc};

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub async fn run() {
    gbase::run::<App>().await;
}

const FONT_SIZE: f32 = 40.0;
const FONT_COLOR: Vec4 = gbase_utils::WHITE;
const BTN_SIZE: f32 = 50.0;
const CLOUD_BASE_RESOLUTION: UVec2 = uvec2(3840, 2160);

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

    blue_noise_zoom: f32,
    blue_noise_step_mult: f32,

    sun_density_contribution_limit: f32,
}

impl Default for CloudParameters {
    fn default() -> Self {
        Self {
            light_pos: vec3(108.0, 167.0, 328.0),
            bounds_min: vec3(-250.0, -25.0, -250.0),
            bounds_max: vec3(250.0, 25.0, 250.0),

            alpha_cutoff: 0.0,
            density_cutoff: 0.27,
            henyey_forw: 0.57,
            henyey_back: 0.51,
            henyey_dist: 0.4,

            density_absorption: 0.65,
            sun_absorption: 0.1,

            transmittance_cutoff: 0.001,
            sun_light_mult: 2.88,
            cloud_sample_mult: 200.0,

            blue_noise_zoom: 5.0,
            blue_noise_step_mult: 1.0,

            sun_density_contribution_limit: 0.016,
        }
    }
}

pub struct App {
    cloud_framebuffer: render::FrameBuffer,
    framebuffer_blitter: gbase_utils::TextureRenderer,
    depth_buffer: render::DepthBuffer,

    camera: gbase_utils::Camera,
    camera_buffer: render::UniformBuffer<gbase_utils::CameraUniform>,

    ui_renderer: gbase_utils::GUIRenderer,
    gizmo_renderer: gbase_utils::GizmoRenderer,
    cloud_renderer: cloud_renderer::CloudRenderer,
    gaussian_blur: gaussian_filter::GaussianFilter,

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

    cloud_resolution: UVec2,

    frames: u32,
}

impl gbase::Callbacks for App {
    #[no_mangle]
    fn init_ctx() -> gbase::ContextBuilder {
        gbase::ContextBuilder::new()
            .log_level(gbase::tracing::Level::WARN)
            .window_attributes(Window::default_attributes().with_maximized(true))
        // .vsync(false)
    }

    #[no_mangle]
    fn new(ctx: &mut gbase::Context, cache: &mut gbase::asset::AssetCache) -> Self {
        let cloud_framebuffer = render::FrameBufferBuilder::new()
            .usage(
                wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::STORAGE_BINDING
                    | wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::COPY_SRC,
            )
            .screen_size(ctx)
            .build(ctx);
        let depth_buffer = render::DepthBufferBuilder::new()
            .screen_size(ctx)
            .build(ctx);
        let framebuffer_blitter = gbase_utils::TextureRenderer::new(ctx, cache);

        // TODO: check if it works
        let camera = gbase_utils::Camera::new_with_screen_size(
            ctx,
            gbase_utils::CameraProjection::perspective(PI / 2.0),
        )
        .pos(vec3(-68.0, -68.0, -67.0))
        .yaw(3.43)
        .pitch(0.35);
        let camera_buffer = render::UniformBufferBuilder::new().build(ctx);

        let ui_renderer = gbase_utils::GUIRenderer::new(
            ctx,
            1024,
            &filesystem::load_b!("fonts/font.ttf").unwrap(),
            gbase_utils::DEFAULT_SUPPORTED_CHARS, // TODO: make this an enum?
        );
        let cloud_parameters_buffer = UniformBufferBuilder::new().build(ctx);
        cloud_parameters_buffer.write(ctx, &CloudParameters::default());
        let gizmo_renderer = gbase_utils::GizmoRenderer::new(ctx);
        let cloud_renderer = cloud_renderer::CloudRenderer::new(ctx, cache)
            .expect("could not create cloud renderer");
        let gaussian_blur = gaussian_filter::GaussianFilter::new(ctx);

        Self {
            cloud_framebuffer,
            depth_buffer,
            framebuffer_blitter,
            ui_renderer,
            gizmo_renderer,
            cloud_renderer,
            gaussian_blur,

            camera,
            camera_buffer,

            cloud_params: CloudParameters::default(),
            cloud_parameters_buffer,

            debug_mode: false,
            enable_gizmos: false,
            debug_msg: String::from("Ok"),
            param_index: 1,
            load_param_index: true,
            write_param_index: false,
            params_changed: false,
            store_surface: false,

            cloud_resolution: CLOUD_BASE_RESOLUTION,

            frames: 0,
        }
    }

    #[no_mangle]
    fn render(
        &mut self,
        ctx: &mut gbase::Context,
        cache: &mut gbase::asset::AssetCache,
        screen_view: &wgpu::TextureView,
    ) -> bool {
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

        // write buffers
        self.camera_buffer.write(ctx, &self.camera.uniform());
        self.cloud_parameters_buffer.write(ctx, &self.cloud_params);

        // clear buffers
        self.cloud_framebuffer.clear(
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
        self.cloud_renderer.render(
            ctx,
            cache,
            self.cloud_framebuffer.view_ref(),
            &self.depth_buffer,
            &self.cloud_framebuffer,
            &self.camera_buffer,
            &self.cloud_parameters_buffer,
        );

        if input::key_pressed(ctx, input::KeyCode::KeyH) {
            self.gaussian_blur.apply_filter(
                ctx,
                &self.cloud_framebuffer,
                &gaussian_filter::GaussianFilterParams::new(2, 1.5),
            );
        }
        if input::key_pressed(ctx, input::KeyCode::KeyJ) {
            self.gaussian_blur.apply_filter(
                ctx,
                &self.cloud_framebuffer,
                &gaussian_filter::GaussianFilterParams::new(1, 1.0),
            );
        }

        // render to image
        // ignores gizmos and ui
        if self.store_surface {
            self.store(ctx, cache);
            self.store_surface = false;
        }

        self.framebuffer_blitter.render(
            ctx,
            cache,
            self.cloud_framebuffer.view(),
            screen_view,
            render::surface_format(ctx),
        );

        if self.enable_gizmos {
            self.gizmos(ctx);
            self.gizmo_renderer.render(
                ctx,
                screen_view,
                render::surface_format(ctx),
                &self.camera_buffer,
            );
        }

        self.ui_renderer
            .render(ctx, screen_view, render::surface_format(ctx));

        false
    }

    #[no_mangle]
    fn resize(
        &mut self,
        ctx: &mut gbase::Context,
        _cache: &mut gbase::asset::AssetCache,
        new_size: PhysicalSize<u32>,
    ) {
        self.gizmo_renderer.resize(ctx, new_size);
        self.ui_renderer.resize(ctx, new_size);
        self.camera.resize(new_size);
    }
}

impl App {
    #[no_mangle]
    fn hot_reload(&mut self, ctx: &mut Context) {
        Self::init_ctx().init_logging();
        self.cloud_renderer.reload_noise(ctx);
    }
}

impl App {
    fn gizmos(&mut self, _ctx: &Context) {
        let bb =
            Transform3D::from_scale(self.cloud_params.bounds_max - self.cloud_params.bounds_min);
        self.gizmo_renderer.draw_cube(
            &Transform3D::new(bb.pos, Quat::IDENTITY, bb.scale),
            gbase_utils::RED.xyz(),
        );
        self.gizmo_renderer.draw_cube(
            &Transform3D::from_pos(self.cloud_params.light_pos).with_scale(Vec3::ONE * 5.0),
            gbase_utils::RED.xyz(),
        );
    }

    // save and load param configs
    fn save_load(&mut self, ctx: &Context) {
        //
        // Keyboard shortcuts
        //

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
        let file_name = format!("cloud_params_{index}.txt");

        if self.write_param_index {
            let content =
                serde_json::to_string(&self.cloud_params).expect("could not serialze params");
            match filesystem::store_str_tmp(ctx, &file_name, &content) {
                Ok(_) => tracing::info!("Sucessfully updated params {}", index),
                Err(err) => tracing::error!("could not write params: {}", err),
            }

            self.write_param_index = false;
            self.params_changed = false;
            println!("aaaaa wrote params {}", self.param_index);
        }

        if self.load_param_index && !self.params_changed {
            match filesystem::load_str_tmp(ctx, &file_name) {
                Ok(content) => {
                    let params = match serde_json::from_str(&content) {
                        Ok(params) => params,
                        Err(err) => {
                            println!("could not deserialize params: {err}");
                            return;
                        }
                    };
                    self.cloud_params = params;
                }
                Err(err) => tracing::error!("could not load params {}: {}", index, err),
            }

            self.load_param_index = false;
            self.params_changed = false;
            println!("loaded params {}", self.param_index);
        }
    }

    fn ui(&mut self, ctx: &Context) {
        let params_old = self.cloud_params.clone();

        let renderer = &mut self.ui_renderer;
        let outer = Widget::new()
            .direction(Direction::Column)
            .width(SizeKind::PercentOfParent(1.0))
            .height(SizeKind::PercentOfParent(1.0))
            .gap(20.0)
            .padding(20.0);
        outer.layout(renderer, |renderer| {
            Widget::new()
                .text(format!("Shader: {}", self.debug_msg))
                .text_color(FONT_COLOR)
                .width(SizeKind::TextSize)
                .height(SizeKind::TextSize)
                .text_font_size(FONT_SIZE)
                .render(renderer);
            Widget::new()
                .text(format!("fps: {:.2}", time::fps(ctx)))
                .text_color(FONT_COLOR)
                .width(SizeKind::TextSize)
                .height(SizeKind::TextSize)
                .text_font_size(FONT_SIZE)
                .render(renderer);
            Widget::new()
                .text("Params")
                .text_color(FONT_COLOR)
                .width(SizeKind::TextSize)
                .height(SizeKind::TextSize)
                .text_font_size(FONT_SIZE)
                .render(renderer);
            let params = Widget::new()
                .width(SizeKind::ChildrenSum)
                .height(SizeKind::ChildrenSum)
                .direction(Direction::Row)
                .cross_axis_alignment(Alignment::Center)
                .gap(20.0);
            params.layout(renderer, |renderer| {
                for i in 1..=5 {
                    let param_index_btn = Widget::new()
                        .label(format!("param index {i}"))
                        .width(SizeKind::Pixels(BTN_SIZE))
                        .height(SizeKind::Pixels(BTN_SIZE))
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
                    .width(SizeKind::TextSize)
                    .height(SizeKind::TextSize)
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
                    .width(SizeKind::TextSize)
                    .height(SizeKind::TextSize)
                    .text_font_size(FONT_SIZE)
                    .color(if self.params_changed { RED } else { GRAY })
                    .padding(10.0)
                    .button(ctx, renderer);
                if discard_btn.clicked {
                    self.params_changed = false;
                    self.load_param_index = true;
                }
            });
            Widget::new()
                .text("Resolution")
                .text_color(FONT_COLOR)
                .width(SizeKind::TextSize)
                .height(SizeKind::TextSize)
                .text_font_size(FONT_SIZE)
                .render(renderer);
            let resolution = Widget::new()
                .width(SizeKind::ChildrenSum)
                .height(SizeKind::ChildrenSum)
                .direction(Direction::Row)
                .cross_axis_alignment(Alignment::Center)
                .gap(20.0);
            resolution.layout(renderer, |renderer| {
                let resolutions = [
                    // same aspect ratio
                    CLOUD_BASE_RESOLUTION / 1,
                    CLOUD_BASE_RESOLUTION / 2,
                    CLOUD_BASE_RESOLUTION / 3,
                    CLOUD_BASE_RESOLUTION / 4,
                    CLOUD_BASE_RESOLUTION / 6,
                    CLOUD_BASE_RESOLUTION / 7,
                    CLOUD_BASE_RESOLUTION / 8,
                    CLOUD_BASE_RESOLUTION / 12,
                    CLOUD_BASE_RESOLUTION / 16,
                ];
                for res in resolutions {
                    let resolution_btn = Widget::new()
                        .label(format!("resolution {} {}", res.x, res.y))
                        .text(format!("{} {}", res.x, res.y))
                        .text_font_size(FONT_SIZE)
                        .text_color(FONT_COLOR)
                        .color(if res == self.cloud_resolution {
                            GREEN
                        } else {
                            GRAY
                        })
                        .width(SizeKind::TextSize)
                        .height(SizeKind::TextSize)
                        .button(ctx, renderer);
                    if resolution_btn.clicked {
                        println!("change cloud res to {} {}", res.x, res.y);
                        self.cloud_resolution = res;
                        let new_size = winit::dpi::PhysicalSize::new(
                            self.cloud_resolution.x,
                            self.cloud_resolution.y,
                        );
                        self.cloud_framebuffer.resize(ctx, new_size);
                        self.depth_buffer.resize(ctx, new_size);
                    }
                }
            });

            let store_layout = Widget::new()
                .width(SizeKind::ChildrenSum)
                .height(SizeKind::ChildrenSum)
                .direction(Direction::Row)
                .gap(20.0);
            store_layout.layout(renderer, |renderer| {
                let store = Widget::new()
                    .label("store")
                    .text("Store")
                    .text_color(FONT_COLOR)
                    .width(SizeKind::TextSize)
                    .height(SizeKind::TextSize)
                    .text_font_size(FONT_SIZE)
                    .color(GRAY)
                    .button(ctx, renderer);
                if store.clicked {
                    self.frames = 0;
                    self.store_surface = true;
                }
                Widget::new()
                    .width(SizeKind::TextSize)
                    .height(SizeKind::TextSize)
                    .text_font_size(FONT_SIZE)
                    .text(format!("{}", self.frames))
                    .render(renderer);
            });
            self.frames += 1;

            let gizmos_btn = Widget::new()
                .label("gizmos")
                .text("Gizmos")
                .text_color(FONT_COLOR)
                .width(SizeKind::TextSize)
                .height(SizeKind::TextSize)
                .text_font_size(FONT_SIZE)
                .color(if self.enable_gizmos { GREEN } else { GRAY })
                .button(ctx, renderer);
            if gizmos_btn.clicked {
                self.enable_gizmos = !self.enable_gizmos;
            }

            fn f32_slider(
                ctx: &Context,
                renderer: &mut gbase_utils::GUIRenderer,
                min: f32,
                max: f32,
                value: &mut f32,
                label: &str,
            ) {
                Widget::new()
                    .width(SizeKind::ChildrenSum)
                    .height(SizeKind::ChildrenSum)
                    .direction(Direction::Row)
                    .layout(renderer, |renderer| {
                        Widget::new()
                            .text(label)
                            .text_color(FONT_COLOR)
                            .width(SizeKind::Pixels(250.0))
                            .height(SizeKind::TextSize)
                            .text_font_size(FONT_SIZE)
                            .render(renderer);
                        Widget::new()
                            .label(label)
                            .width(SizeKind::Pixels(300.0))
                            .height(SizeKind::Pixels(50.0))
                            .direction(Direction::Row)
                            .color(GRAY)
                            .slider_layout(ctx, renderer, min, max, value, |renderer, res| {
                                Widget::new()
                                    .width(SizeKind::PercentOfParent(res.pos))
                                    .render(renderer);
                                Widget::new()
                                    .width(SizeKind::Pixels(20.0))
                                    .height(SizeKind::PercentOfParent(1.0))
                                    .color(BLUE)
                                    .render(renderer);
                            });
                        Widget::new()
                            .text(format!("{value:.2}"))
                            .text_color(FONT_COLOR)
                            .width(SizeKind::TextSize)
                            .height(SizeKind::TextSize)
                            .text_font_size(FONT_SIZE)
                            .render(renderer);
                    });
            }

            let mut bounds_size = self.cloud_params.bounds_max - self.cloud_params.bounds_min;

            let p = &mut self.cloud_params;
            let sliders = [
                ("bounds x", 0.0, 1000.0, &mut bounds_size.x),
                ("bounds y", 0.0, 100.0, &mut bounds_size.y),
                ("bounds z", 0.0, 1000.0, &mut bounds_size.z),
                // ("light x", -500.0, 500.0, &mut p.light_pos.x),
                // ("light y", -500.0, 500.0, &mut p.light_pos.y),
                // ("light z", -500.0, 500.0, &mut p.light_pos.z),
                ("henyey forw", 0.0, 1.0, &mut p.henyey_forw),
                ("henyey back", 0.0, 1.0, &mut p.henyey_back),
                ("henyey dist", 0.0, 1.0, &mut p.henyey_dist),
                ("sun light mult", 0.0, 30.0, &mut p.sun_light_mult),
                ("d absorption", 0.0, 3.0, &mut p.density_absorption),
                ("s absorption", 0.0, 1.0, &mut p.sun_absorption),
                ("noise zoom", 0.0, 300.0, &mut p.cloud_sample_mult),
                ("blue zoom", 0.0, 10.0, &mut p.blue_noise_zoom),
                ("blue step", 0.0, 1.0, &mut p.blue_noise_step_mult),
                ("alpha cut", 0.0, 1.0, &mut p.alpha_cutoff),
                ("density cut", 0.0, 1.0, &mut p.density_cutoff),
                (
                    "sun density limit",
                    0.0,
                    0.1,
                    &mut p.sun_density_contribution_limit,
                ),
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
    fn store(&self, ctx: &mut Context, cache: &mut gbase::asset::AssetCache) {
        // info
        let ms = time::frame_time(ctx);
        let mut metadata_file = fs::File::create(format!(
            "{}/image_{}_{}.info",
            filesystem::tmp_path(),
            self.cloud_resolution.x,
            self.cloud_resolution.y
        ))
        .expect("could not create metadata file");
        metadata_file
            .write_all(ms.to_string().as_bytes())
            .expect("could not write to metadata file");

        // image
        // NOTE: always render to original resolution
        let image_bytes = texture_to_buffer_gamma(
            ctx,
            cache,
            self.cloud_framebuffer.view(),
            CLOUD_BASE_RESOLUTION.x,
            CLOUD_BASE_RESOLUTION.y,
        );
        let image_buffer =
            gbase_utils::image::ImageBuffer::<gbase_utils::image::Rgba<u8>, _>::from_raw(
                CLOUD_BASE_RESOLUTION.x,
                CLOUD_BASE_RESOLUTION.y,
                image_bytes,
            )
            .expect("could not create image buffer");
        image_buffer
            .save(format!(
                "{}/image_{}_{}.png",
                filesystem::tmp_path(),
                self.cloud_resolution.x,
                self.cloud_resolution.y
            ))
            .expect("could not write to image file");

        println!("STORE")
    }
}

// render texture to Rgba8UnormSrgb and then load to PNG
fn texture_to_buffer_gamma(
    ctx: &mut Context,
    cache: &mut gbase::asset::AssetCache,
    texture: render::ArcTextureView,
    width: u32,
    height: u32,
) -> Vec<u8> {
    let temp_framebuffer = render::FrameBufferBuilder::new()
        .size(width, height)
        .format(wgpu::TextureFormat::Rgba8UnormSrgb)
        .build(ctx);
    gbase_utils::TextureRenderer::new(ctx, cache).render(
        ctx,
        cache,
        texture,
        temp_framebuffer.view_ref(),
        wgpu::TextureFormat::Rgba8UnormSrgb,
    );
    texture_to_buffer_sync(ctx, temp_framebuffer.texture_ref(), width, height)
}

fn texture_to_buffer_sync(
    ctx: &Context,
    texture: &wgpu::Texture,
    width: u32,
    height: u32,
) -> Vec<u8> {
    let pixel_size = std::mem::size_of::<u8>() as u32 * 4;
    let buffer_size = width * height * pixel_size;
    let read_back_buffer = render::RawBufferBuilder::<u8>::new(buffer_size as u64)
        .usage(wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST)
        .build(ctx);

    let mut encoder = render::EncoderBuilder::new().build(ctx);
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &read_back_buffer.buffer(),
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * pixel_size),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
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
