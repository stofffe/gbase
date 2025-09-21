mod bloom;

use gbase::{
    asset::{AssetBuilder, AssetHandle},
    glam::{vec3, vec4, Vec3},
    input::{self, mouse_button_pressed},
    load_b, profile,
    render::{self},
    time, tracing, wgpu, winit, CallbackResult, Callbacks, Context,
};
use gbase_utils::{
    Alignment, Direction, Gltf, GltfLoader, MeshLod, MeshLodLoader, PbrLightUniforms, PbrRenderer,
    SizeKind, Transform3D, Widget, BLACK, GRAY, WHITE,
};
use std::f32::consts::PI;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub async fn run() {
    gbase::run::<App>().await;
}

struct App {
    hdr_framebuffer_1: render::FrameBuffer,
    hdr_framebuffer_2: render::FrameBuffer,
    ldr_framebuffer: render::FrameBuffer,
    framebuffer_renderer: gbase_utils::TextureRenderer,

    depth_buffer: render::DepthBuffer,
    pbr_renderer: gbase_utils::PbrRenderer,
    gizmo_renderer: gbase_utils::GizmoRenderer,
    ui_renderer: gbase_utils::GUIRenderer,

    camera: gbase_utils::Camera,
    camera_buffer: render::UniformBuffer<gbase_utils::CameraUniform>,
    lights_buffer: render::UniformBuffer<PbrLightUniforms>,
    lights: PbrLightUniforms,

    helmet_mesh: AssetHandle<MeshLod>,
    sponza_gltf: AssetHandle<Gltf>,

    tonemap: bloom::Tonemap,
    bloom: bloom::Bloom,

    shadow_pass: gbase_utils::ShadowPass,
}

impl Callbacks for App {
    #[no_mangle]
    fn init_ctx() -> gbase::ContextBuilder {
        gbase::ContextBuilder::new()
            .log_level(tracing::Level::ERROR)
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
    fn new(ctx: &mut Context, cache: &mut gbase::asset::AssetCache) -> Self {
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
        let framebuffer_renderer = gbase_utils::TextureRenderer::new(ctx, cache);

        let depth_buffer = render::DepthBufferBuilder::new()
            .screen_size(ctx)
            .build(ctx);

        let pbr_renderer = PbrRenderer::new(ctx, cache);

        let helmet_mesh = AssetBuilder::load(
            "assets/models/helmet_lod.glb",
            MeshLodLoader::new().with_node_name("mesh_damaged_helmet"),
        )
        .watch(cache)
        .build(cache);
        let sponza_gltf = AssetBuilder::load("assets/models/sponza-2.glb", GltfLoader {})
            .watch(cache)
            .build(cache);

        let camera = gbase_utils::Camera::new_with_screen_size(
            ctx,
            gbase_utils::CameraProjection::Perspective { fov: PI / 2.0 },
        )
        .pos(vec3(0.0, 0.0, 8.0));
        let camera_buffer = render::UniformBufferBuilder::new().build(ctx);
        camera_buffer.write(ctx, &camera.uniform());

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
        let lights_buffer = render::UniformBufferBuilder::new().build(ctx);

        let tonemap = bloom::Tonemap::new(ctx, cache);
        let bloom = bloom::Bloom::new(ctx, cache, hdr_format);

        let shadow_pass = gbase_utils::ShadowPass::new(ctx, cache);

        Self {
            hdr_framebuffer_1,
            hdr_framebuffer_2,
            ldr_framebuffer,
            tonemap,
            bloom,

            pbr_renderer,
            ui_renderer,
            gizmo_renderer,
            lights,
            lights_buffer,

            camera,
            camera_buffer,

            helmet_mesh,
            sponza_gltf,

            shadow_pass,

            framebuffer_renderer,
            depth_buffer,
        }
    }

    #[no_mangle]
    fn render(
        &mut self,
        ctx: &mut Context,
        cache: &mut gbase::asset::AssetCache,
        screen_view: &wgpu::TextureView,
    ) -> CallbackResult {
        if mouse_button_pressed(ctx, input::MouseButton::Left) {
            self.camera.flying_controls(ctx);
        }

        if cache.handle_just_loaded(self.helmet_mesh.clone()) {
            let mesh_lod = self.helmet_mesh.get_mut(cache).unwrap();
            for (mesh, _) in mesh_lod.meshes.clone() {
                mesh.get_mut(cache)
                    .unwrap()
                    .extract_attributes(self.pbr_renderer.required_attributes().clone());
            }
        }

        if self.sponza_gltf.loaded(cache) {
            let gltf = cache.get(self.sponza_gltf.clone()).unwrap().clone();

            for mesh in gltf.meshes.iter().clone() {
                let prim = &mesh.get(cache).unwrap().primitives[0].clone(); // Assume 1 mesh = 1 prim
                prim.mesh
                    .get_mut(cache)
                    .unwrap()
                    .extract_attributes(self.pbr_renderer.required_attributes().clone());

                let mesh = mesh.get(cache).unwrap();
                for primitive in mesh.primitives.iter().clone() {
                    tracing::info!("prim {:#?}", primitive);
                }
            }

            for node in gltf.nodes.iter() {
                let node = node.get(cache).unwrap();
                tracing::info!("node {:#?}", node);

                if let Some(mesh) = node.mesh.clone() {
                    let prim = &mesh.get(cache).unwrap().primitives[0].clone(); // Assume 1 mesh = 1 prim
                    prim.mesh
                        .get_mut(cache)
                        .unwrap()
                        .extract_attributes(self.pbr_renderer.required_attributes().clone());
                }
            }

            dbg!(&gltf.named_nodes);
        }

        let _guard = tracing::span!(tracing::Level::TRACE, "render").entered();

        self.hdr_framebuffer_1.clear(ctx, wgpu::Color::BLACK);
        self.depth_buffer.clear(ctx);

        // TODO: temp
        self.camera_buffer.write(ctx, &self.camera.uniform());
        self.lights_buffer.write(ctx, &self.lights);

        // Render
        let mut meshes = vec![(
            self.helmet_mesh.clone(),
            Transform3D::default()
                .with_pos(vec3(0.0, 5.0, 0.0))
                .with_scale(Vec3::ONE * 5.0),
        )];

        let i = 0;
        let j = 4;
        if self.sponza_gltf.loaded(cache) {
            for node in &self.sponza_gltf.get(cache).unwrap().clone().nodes {
                let node = node.get(cache).unwrap();
                let transform = node.transform.clone();
                if let Some(mesh) = node.mesh.clone() {
                    let mesh = mesh.get(cache).unwrap();
                    let prim = mesh.primitives[0].clone(); // Assume 1 mesh = 1 prim
                    let lod = MeshLod::from_single_lod(prim.mesh, prim.material);
                    meshes.push((cache.insert(lod), transform));
                }
            }

            // for submesh in &self.sponza_gltf.get(cache).unwrap().clone().meshes[i..i + j] {
            //     let prim = &submesh.get(cache).unwrap().primitives[0].clone(); // Assume 1 mesh = 1 prim
            //     let lod = MeshLod::from_single_lod(prim.mesh, prim.material);
            //     meshes.push((cache.insert(lod), Transform3D::default()));
            // }
        }

        self.shadow_pass.render(
            ctx,
            cache,
            &meshes,
            &self.camera,
            self.lights.main_light_dir,
        );

        {
            let _timer = profile::ProfileTimer::new(ctx, "render");
            self.pbr_renderer.render(
                ctx,
                cache,
                self.hdr_framebuffer_1.view_ref(),
                self.hdr_framebuffer_1.format(),
                &self.camera,
                &self.camera_buffer,
                &self.lights_buffer,
                &self.depth_buffer,
                &self.camera.calculate_frustum(),
                meshes,
                &self.shadow_pass.shadow_map,
                &self.shadow_pass.light_matrices_buffer,
                &self.shadow_pass.light_matrices_distances,
            );
        }

        profile::ProfileTimer::scoped(ctx, "gizmo", |ctx| {
            self.gizmo_renderer.render(
                ctx,
                self.hdr_framebuffer_1.view_ref(),
                self.hdr_framebuffer_1.format(),
                &self.camera_buffer,
            );
        });

        // let start = std::time::Instant::now();
        // // self.bloom
        // //     .render(ctx, cache, &self.hdr_framebuffer_1, &self.hdr_framebuffer_2);
        // if input::key_pressed(ctx, input::KeyCode::KeyB) {
        //     time::profiler(ctx).add_cpu_sample("bloom", start.elapsed().as_secs_f32());
        // }
        // self.tonemap
        //     .tonemap(ctx, cache, &self.hdr_framebuffer_2, &self.ldr_framebuffer);

        self.tonemap
            .tonemap(ctx, cache, &self.hdr_framebuffer_1, &self.ldr_framebuffer);

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

                for (label, time) in profile::profiler(ctx).get_cpu_samples() {
                    Widget::new()
                        .width(SizeKind::TextSize)
                        .height(SizeKind::TextSize)
                        .text(format!("CPU: {:.5} {}", time * 1000.0, label))
                        .text_color(vec4(1.0, 1.0, 1.0, 1.0))
                        .render(renderer);
                }

                for (label, time) in profile::profiler(ctx).get_gpu_samples() {
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
            cache,
            self.ldr_framebuffer.view(),
            screen_view,
            render::surface_format(ctx),
        );

        CallbackResult::Continue
    }

    #[no_mangle]
    fn resize(
        &mut self,
        ctx: &mut Context,
        _cache: &mut gbase::asset::AssetCache,
        new_size: gbase::winit::dpi::PhysicalSize<u32>,
    ) -> CallbackResult {
        self.depth_buffer.resize(ctx, new_size);
        self.ui_renderer.resize(ctx, new_size);
        self.gizmo_renderer.resize(ctx, new_size);
        self.hdr_framebuffer_1.resize(ctx, new_size);
        self.hdr_framebuffer_2.resize(ctx, new_size);
        self.ldr_framebuffer.resize(ctx, new_size);
        self.camera.resize(new_size);

        CallbackResult::Continue
    }
}

impl App {
    #[no_mangle]
    fn hot_reload(&mut self, _ctx: &mut Context) {
        Self::init_ctx().init_logging();
    }
}
