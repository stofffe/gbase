mod bloom;

use gbase::{
    glam::{vec3, vec4, Vec3},
    input::{self, mouse_button_pressed},
    load_b,
    render::{self, GpuImage, GpuMesh, Image, Mesh, ShaderBuilder},
    time, tracing, wgpu, winit, Callbacks, Context,
};
use gbase_utils::{
    Alignment, AssetCache, AssetHandle, Direction, GpuMaterial, PbrLightUniforms, PbrRenderer,
    PixelCache, SizeKind, Transform3D, Widget, BLACK, GRAY, WHITE,
};
use std::{f32::consts::PI, sync::Arc, time::Instant};

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub async fn run() {
    gbase::run::<App>().await;
}

struct App {
    hdr_framebuffer_1: render::FrameBuffer,
    hdr_framebuffer_2: render::FrameBuffer,
    ldr_framebuffer: render::FrameBuffer,
    framebuffer_renderer: gbase_utils::TextureRenderer,

    image_cache: AssetCache<Image, GpuImage>,
    mesh_cache: AssetCache<Mesh, GpuMesh>,
    shader_cache: AssetCache<ShaderBuilder, wgpu::ShaderModule>,
    pixel_cache: PixelCache,

    depth_buffer: render::DepthBuffer,
    pbr_renderer: gbase_utils::PbrRenderer,
    gizmo_renderer: gbase_utils::GizmoRenderer,
    ui_renderer: gbase_utils::GUIRenderer,

    camera: gbase_utils::Camera,
    camera_buffer: render::UniformBuffer<gbase_utils::CameraUniform>,
    lights_buffer: render::UniformBuffer<PbrLightUniforms>,
    lights: PbrLightUniforms,

    ak47_mesh_handle: AssetHandle<Mesh>,
    ak47_material: Arc<GpuMaterial>,
    helmet_mesh_handle: AssetHandle<Mesh>,
    helmet_material: Arc<GpuMaterial>,
    cube_mesh_handle: gbase_utils::AssetHandle<Mesh>,
    cube_material: Arc<GpuMaterial>,

    tonemap: bloom::Tonemap,
    bloom: bloom::Bloom,
}

fn load_simple_mesh(
    bytes: &[u8],
    mesh_cache: &mut AssetCache<Mesh, GpuMesh>,
    image_cache: &mut AssetCache<Image, GpuImage>,
    pixel_cache: &mut PixelCache,
    pbr_renderer: &PbrRenderer,
) -> (AssetHandle<Mesh>, Arc<GpuMaterial>) {
    let prim = gbase_utils::parse_glb(bytes)[0].clone();
    let mesh = prim
        .mesh
        .extract_attributes(pbr_renderer.required_attributes().clone());
    let mesh_handle = mesh_cache.allocate(mesh);
    let material = prim.material.clone().to_material(image_cache, pixel_cache);
    (mesh_handle, Arc::new(material))
}

impl Callbacks for App {
    #[no_mangle]
    fn init_ctx() -> gbase::ContextBuilder {
        gbase::ContextBuilder::new()
            .log_level(tracing::Level::INFO)
            .vsync(false)
            .device_features(
                wgpu::Features::POLYGON_MODE_LINE
                    | wgpu::Features::TIMESTAMP_QUERY
                    | wgpu::Features::RG11B10UFLOAT_RENDERABLE,
            )
            .window_attributes(winit::window::Window::default_attributes().with_maximized(true))
            .gpu_profiler_enabled(false)
    }

    #[no_mangle]
    fn new(ctx: &mut Context) -> Self {
        let mut image_cache = AssetCache::new();
        let mut mesh_cache = AssetCache::new();
        let mut shader_cache = AssetCache::new();
        let mut pixel_cache = PixelCache::new();

        let hdr_format = if render::device(ctx)
            .features()
            .contains(wgpu::Features::RG11B10UFLOAT_RENDERABLE)
        {
            wgpu::TextureFormat::Rg11b10Ufloat
        } else {
            wgpu::TextureFormat::Rgba16Float
        };
        let hdr_framebuffer_builder = render::FrameBufferBuilder::new()
            .screen_size(ctx)
            .format(hdr_format)
            .usage(wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING);
        let hdr_framebuffer_1 = hdr_framebuffer_builder.clone().label("hdr 1").build(ctx);
        let hdr_framebuffer_2 = hdr_framebuffer_builder.label("hdr 2").build(ctx);

        let ldr_framebuffer = render::FrameBufferBuilder::new()
            .screen_size(ctx)
            .format(wgpu::TextureFormat::Rgba8Unorm)
            .usage(
                wgpu::TextureUsages::STORAGE_BINDING
                    | wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::RENDER_ATTACHMENT,
            )
            .label("ldr")
            .build(ctx);
        let framebuffer_renderer = gbase_utils::TextureRenderer::new(ctx);

        let depth_buffer = render::DepthBufferBuilder::new()
            .screen_size(ctx)
            .build(ctx);

        let pbr_renderer = PbrRenderer::new(ctx);

        let (ak47_mesh_handle, ak47_material) = load_simple_mesh(
            &load_b!("models/ak47.glb").unwrap(),
            &mut mesh_cache,
            &mut image_cache,
            &mut pixel_cache,
            &pbr_renderer,
        );

        let (helmet_mesh_handle, helmet_material) = load_simple_mesh(
            &load_b!("models/helmet.glb").unwrap(),
            &mut mesh_cache,
            &mut image_cache,
            &mut pixel_cache,
            &pbr_renderer,
        );

        let (cube_mesh_handle, cube_material) = load_simple_mesh(
            &load_b!("models/cube.glb").unwrap(),
            &mut mesh_cache,
            &mut image_cache,
            &mut pixel_cache,
            &pbr_renderer,
        );

        let camera =
            gbase_utils::Camera::new(gbase_utils::CameraProjection::Perspective { fov: PI / 2.0 })
                .pos(vec3(0.0, 0.0, 8.0));
        let camera_buffer = render::UniformBufferBuilder::new(render::UniformBufferSource::Data(
            camera.uniform(ctx),
        ))
        .build(ctx);

        let ui_renderer = gbase_utils::GUIRenderer::new(
            ctx,
            1024,
            &load_b!("fonts/font.ttf").unwrap(),
            gbase_utils::DEFAULT_SUPPORTED_CHARS,
        );
        let gizmo_renderer = gbase_utils::GizmoRenderer::new(ctx);

        let lights = PbrLightUniforms {
            main_light_dir: vec3(0.0, -1.0, -1.0).normalize(),
            main_light_insensity: 1.0,
        };
        let lights_buffer =
            render::UniformBufferBuilder::new(render::UniformBufferSource::Empty).build(ctx);

        let tonemap = bloom::Tonemap::new(ctx, &mut shader_cache);
        let bloom = bloom::Bloom::new(ctx, &mut shader_cache, hdr_format);

        Self {
            hdr_framebuffer_1,
            hdr_framebuffer_2,
            ldr_framebuffer,
            depth_buffer,
            framebuffer_renderer,
            tonemap,
            bloom,

            pbr_renderer,
            ui_renderer,
            gizmo_renderer,
            lights,
            lights_buffer,

            camera,
            camera_buffer,

            image_cache,
            mesh_cache,
            shader_cache,
            pixel_cache,

            ak47_mesh_handle,
            ak47_material,

            helmet_mesh_handle,
            helmet_material,

            cube_mesh_handle,
            cube_material,
        }
    }

    #[no_mangle]
    fn update(&mut self, ctx: &mut Context) -> bool {
        if mouse_button_pressed(ctx, input::MouseButton::Left) {
            self.camera.flying_controls(ctx);
        }

        if gbase::input::key_just_pressed(ctx, gbase::input::KeyCode::KeyR) {
            tracing::warn!("RESTART");
            *self = Self::new(ctx);
        }

        false
    }

    #[no_mangle]
    fn render(&mut self, ctx: &mut Context, screen_view: &gbase::wgpu::TextureView) -> bool {
        let _guard = tracing::span!(tracing::Level::TRACE, "render").entered();
        // let timer = time::CpuProfileTimer::new("render");

        self.mesh_cache.check_watched_files(ctx);
        self.image_cache.check_watched_files(ctx);
        self.shader_cache.check_watched_files(ctx);

        self.hdr_framebuffer_1.clear(ctx, wgpu::Color::BLACK);
        self.depth_buffer.clear(ctx);

        self.camera_buffer.write(ctx, &self.camera.uniform(ctx));
        self.lights_buffer.write(ctx, &self.lights);

        // let t = time::time_since_start(ctx);
        self.pbr_renderer.add_mesh(
            self.helmet_mesh_handle.clone(),
            self.helmet_material.clone(),
            Transform3D::default()
                .with_pos(vec3(0.0, 0.0, 0.0))
                .with_scale(Vec3::ONE * 5.0), // .with_rot(Quat::from_rotation_y(t * PI / 10.0)),
        );

        {
            let _timer = time::ProfileTimer::new(ctx, "render");
            self.pbr_renderer.render(
                ctx,
                &mut self.mesh_cache,
                &mut self.image_cache,
                self.hdr_framebuffer_1.view_ref(),
                self.hdr_framebuffer_1.format(),
                &self.camera,
                &self.camera_buffer,
                &self.lights_buffer,
                &self.depth_buffer,
            );
        }

        time::ProfileTimer::scoped(ctx, "gizmo", |ctx| {
            self.gizmo_renderer.render(
                ctx,
                self.hdr_framebuffer_1.view_ref(),
                self.hdr_framebuffer_1.format(),
                &self.camera_buffer,
            );
        });

        let start = Instant::now();
        self.bloom.render(
            ctx,
            &mut self.shader_cache,
            &self.hdr_framebuffer_1,
            &self.hdr_framebuffer_2,
        );
        if input::key_pressed(ctx, input::KeyCode::KeyB) {
            time::profiler(ctx).add_cpu_sample("bloom", start.elapsed().as_secs_f32());
        }

        self.tonemap.tonemap(
            ctx,
            &mut self.shader_cache,
            &self.hdr_framebuffer_2,
            &self.ldr_framebuffer,
        );

        {
            let _guard = tracing::span!(tracing::Level::TRACE, "ui update").entered();
            let outer = Widget::new()
                .label("outer")
                .width(SizeKind::PercentOfParent(1.0))
                .height(SizeKind::PercentOfParent(1.0))
                .direction(Direction::Column)
                .gap(20.0)
                .padding(20.0);

            outer.layout(&mut self.ui_renderer, |renderer| {
                let slider_row = Widget::new()
                    .height(SizeKind::Pixels(100.0))
                    .width(SizeKind::ChildrenSum)
                    .gap(20.0)
                    .cross_axis_alignment(Alignment::Center)
                    .direction(Direction::Row);
                slider_row.layout(renderer, |renderer| {
                    Widget::new()
                        .text("main light intensity")
                        .text_color(WHITE)
                        .height(SizeKind::TextSize)
                        .width(SizeKind::TextSize)
                        .text_font_size(60.0)
                        .render(renderer);
                    let slider = Widget::new()
                        .label("slider")
                        .color(GRAY)
                        .border_radius(10.0)
                        .height(SizeKind::Pixels(100.0))
                        .width(SizeKind::Pixels(500.0))
                        .direction(Direction::Row);
                    slider.slider_layout(
                        ctx,
                        renderer,
                        0.0,
                        20.0,
                        &mut self.lights.main_light_insensity,
                        |renderer, res| {
                            Widget::new()
                                .width(SizeKind::PercentOfParent(res.pos))
                                .render(renderer);
                            Widget::new()
                                .width(SizeKind::Pixels(10.0))
                                .height(SizeKind::Grow)
                                .color(BLACK)
                                .border_radius(5.0)
                                .render(renderer);
                        },
                    );

                    Widget::new()
                        .text(format!("({:.3})", self.lights.main_light_insensity))
                        .text_color(WHITE)
                        .width(SizeKind::TextSize)
                        .height(SizeKind::TextSize)
                        .text_font_size(60.0)
                        .render(renderer);
                });
                Widget::new()
                    .width(SizeKind::TextSize)
                    .height(SizeKind::TextSize)
                    .text(format!("{:.5} fps", time::fps(ctx)))
                    .text_color(vec4(1.0, 1.0, 1.0, 1.0))
                    .render(renderer);

                Widget::new()
                    .width(SizeKind::TextSize)
                    .height(SizeKind::TextSize)
                    .text(format!("{:.5} ms", time::frame_time(ctx) * 1000.0))
                    .text_color(vec4(1.0, 1.0, 1.0, 1.0))
                    .render(renderer);

                for (label, time) in time::profiler(ctx).get_cpu_samples() {
                    Widget::new()
                        .width(SizeKind::TextSize)
                        .height(SizeKind::TextSize)
                        .text(format!("CPU: {:.5} {}", time * 1000.0, label))
                        .text_color(vec4(1.0, 1.0, 1.0, 1.0))
                        .render(renderer);
                }

                for (label, time) in time::profiler(ctx).get_gpu_samples() {
                    Widget::new()
                        .width(SizeKind::TextSize)
                        .height(SizeKind::TextSize)
                        .text(format!("GPU: {:.5} {}", time * 1000.0, label))
                        .text_color(vec4(1.0, 1.0, 1.0, 1.0))
                        .render(renderer);
                }
            });

            let _ui = tracing::span!(tracing::Level::TRACE, "ui update").entered();
            self.ui_renderer.render(
                ctx,
                self.ldr_framebuffer.view_ref(),
                self.ldr_framebuffer.format(),
            );
        }

        self.framebuffer_renderer.render(
            ctx,
            self.ldr_framebuffer.view(),
            screen_view,
            render::surface_format(ctx),
        );

        false
    }

    #[no_mangle]
    fn resize(&mut self, ctx: &mut Context, new_size: gbase::winit::dpi::PhysicalSize<u32>) {
        self.depth_buffer.resize(ctx, new_size);
        self.ui_renderer.resize(ctx, new_size);
        self.gizmo_renderer.resize(ctx, new_size);
        self.hdr_framebuffer_1.resize(ctx, new_size);
        self.hdr_framebuffer_2.resize(ctx, new_size);
        self.ldr_framebuffer.resize(ctx, new_size);
    }
}

impl App {
    #[no_mangle]
    fn hot_reload(&mut self, _ctx: &mut Context) {
        Self::init_ctx().init_logging();
        // render::set_vsync(_ctx, false);
    }
}

// let elems = 10000u32;
// for x in 0..(elems.isqrt()) {
//     for z in 0..(elems.isqrt()) {
//         let transform = Transform3D::from_pos(vec3(5.0 * x as f32, 0.0, 10.0 * z as f32))
//             .with_rot(Quat::from_rotation_y(
//                 (time::time_since_start(ctx) + (x + z) as f32) * 1.0,
//             ));
//
//         // self.pbr_renderer.add_mesh(
//         //     self.cube_mesh_handle.clone(),
//         //     self.cube_material.clone(),
//         //     transform,
//         // );
//
//         if (x + z) % 2 == 0 {
//             self.pbr_renderer.add_mesh(
//                 self.ak47_mesh_handle.clone(),
//                 self.ak47_material.clone(),
//                 transform,
//             );
//         } else {
//             self.pbr_renderer.add_mesh(
//                 self.helmet_mesh_handle.clone(),
//                 self.helmet_material.clone(),
//                 transform,
//             );
//         }
//     }
// }
