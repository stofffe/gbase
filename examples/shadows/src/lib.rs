use gbase::{
    asset::{self, AssetBuilder, AssetCache, AssetHandle},
    egui::{self, load::SizedTexture},
    egui_ui,
    glam::{vec3, Quat, Vec3},
    input::{self, mouse_button_pressed},
    load_b, profile, render, time,
    tracing::{self, span},
    wgpu, winit, CallbackResult, Callbacks, Context, ContextBuilder,
};
use gbase_utils::{
    Camera, Material, MeshLodLoader, PbrLightUniforms, PbrRenderer, Transform3D, ViewPort,
};
use gbase_utils::{MeshLod, ShadowPass};
use std::f32::consts::PI;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub async fn run() {
    gbase::run::<App>().await;
}

const PLANE_SIZE: f32 = 1000.0;
const PLANE_COLOR: [f32; 4] = [0.3, 1.0, 0.2, 1.0];

struct App {
    hdr_framebuffer_1: render::FrameBuffer,
    ldr_framebuffer: render::FrameBuffer,
    framebuffer_renderer: gbase_utils::TextureRenderer,

    depth_buffer: render::DepthBuffer,
    pbr_renderer: gbase_utils::PbrRenderer,
    gizmo_renderer: gbase_utils::GizmoRenderer,
    ui_renderer: gbase_utils::GUIRenderer,

    camera: Camera,
    camera_buffer: render::UniformBuffer<gbase_utils::CameraUniform>,
    lights: PbrLightUniforms,
    lights_buffer: render::UniformBuffer<PbrLightUniforms>,

    ak47_mesh: AssetHandle<MeshLod>,
    helmet_mesh: AssetHandle<MeshLod>,
    plane_mesh: AssetHandle<MeshLod>,

    shadow_pass: ShadowPass,
    shadow_map_rgb: render::ArcTexture,

    toggle_debug_ui: bool,
}

fn mesh_to_lod_mesh(
    cache: &mut AssetCache,
    mesh: AssetHandle<render::Mesh>,
    material: AssetHandle<Material>,
) -> AssetHandle<MeshLod> {
    cache.insert(MeshLod {
        meshes: vec![(mesh, 0.0)],
        material,
    })
}

#[derive(Debug, Clone)]
struct DrawCall {
    mesh: AssetHandle<MeshLod>,
    transform: Transform3D,
}

impl Callbacks for App {
    #[no_mangle]
    fn init_ctx() -> gbase::ContextBuilder {
        gbase::ContextBuilder::new()
            .log_level(tracing::Level::INFO)
            // .vsync(false)
            .device_features(
                wgpu::Features::POLYGON_MODE_LINE | wgpu::Features::RG11B10UFLOAT_RENDERABLE,
            )
            .window_attributes(winit::window::Window::default_attributes().with_maximized(true))
            .gpu_profiler_enabled(true)
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
        let hdr_framebuffer = hdr_framebuffer_builder.clone().label("hdr 1").build(ctx);

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
            cache,
            "assets/models/helmet_lod.glb",
            MeshLodLoader::new().with_node_name("mesh_damaged_helmet"),
        )
        .watch(cache)
        .build(cache);

        let ak47_mesh = AssetBuilder::load(cache, "assets/models/ak47.glb", MeshLodLoader::new())
            .watch(cache)
            .build(cache);

        let camera = gbase_utils::Camera::new_with_screen_size(
            ctx,
            gbase_utils::CameraProjection::Perspective { fov: PI / 2.0 },
        )
        .pos(vec3(0.0, 0.0, 8.0));
        let camera_buffer = render::UniformBufferBuilder::new().build(ctx);

        let ui_renderer = gbase_utils::GUIRenderer::new(
            ctx,
            1024,
            &load_b!("fonts/font.ttf").unwrap(),
            gbase_utils::DEFAULT_SUPPORTED_CHARS,
        );
        let gizmo_renderer = gbase_utils::GizmoRenderer::new(ctx);

        let lights = PbrLightUniforms {
            main_light_dir: vec3(1.0, -1.0, 1.0).normalize(),
            main_light_insensity: 1.0,
        };
        let lights_buffer = render::UniformBufferBuilder::new().build(ctx);

        let plane_mesh_handle = asset::AssetBuilder::insert(
            render::MeshBuilder::quad()
                .build()
                .with_extracted_attributes(pbr_renderer.required_attributes().clone()),
        )
        .build(cache);
        let plane_material = gbase_utils::Material::default(cache).with_color_factor(PLANE_COLOR);
        let plane_material = cache.insert(plane_material);
        let plane_mesh = mesh_to_lod_mesh(cache, plane_mesh_handle, plane_material);

        let shadow_pass = ShadowPass::new(ctx, cache);

        let shadow_map_rgb = render::TextureBuilder::new(render::TextureSource::Empty(
            gbase_utils::SHADOW_MAP_RESOLUTION,
            gbase_utils::SHADOW_MAP_RESOLUTION,
        ))
        .label("shadow map rgb")
        .with_format(wgpu::TextureFormat::Rgba8Unorm)
        .usage(wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING)
        .depth_or_array_layers(gbase_utils::MAX_SHADOW_CASCADES)
        .build(ctx);

        Self {
            hdr_framebuffer_1: hdr_framebuffer,
            ldr_framebuffer,
            depth_buffer,
            framebuffer_renderer,

            pbr_renderer,
            ui_renderer,
            gizmo_renderer,
            lights,
            lights_buffer,

            camera,
            camera_buffer,

            helmet_mesh,
            ak47_mesh,
            plane_mesh,

            shadow_pass,
            shadow_map_rgb,

            toggle_debug_ui: true,
        }
    }

    #[no_mangle]
    fn render(
        &mut self,
        ctx: &mut Context,
        cache: &mut gbase::asset::AssetCache,
        screen_view: &gbase::wgpu::TextureView,
    ) -> CallbackResult {
        let _render_span = span!(tracing::Level::INFO, "render").entered();

        if input::key_just_pressed(ctx, input::KeyCode::F1) {
            profile::enable_gpu_profiling(ctx, true);
        }
        if input::key_just_pressed(ctx, input::KeyCode::F2) {
            profile::enable_gpu_profiling(ctx, false);
        }

        if cache.handle_just_loaded(self.helmet_mesh.clone()) {
            let mesh_lod = self.helmet_mesh.get_mut(cache).unwrap();
            for (mesh, _) in mesh_lod.meshes.clone() {
                mesh.get_mut(cache)
                    .unwrap()
                    .extract_attributes(self.pbr_renderer.required_attributes().clone());
            }
        }
        if cache.handle_just_loaded(self.ak47_mesh.clone()) {
            let mesh_lod = self.ak47_mesh.get_mut(cache).unwrap();
            for (mesh, _) in mesh_lod.meshes.clone() {
                mesh.get_mut(cache)
                    .unwrap()
                    .extract_attributes(self.pbr_renderer.required_attributes().clone());
            }
        }

        if mouse_button_pressed(ctx, input::MouseButton::Left) {
            self.camera.flying_controls(ctx);
        }

        if gbase::input::key_just_pressed(ctx, gbase::input::KeyCode::KeyR) {
            tracing::warn!("RESTART");
            *self = Self::new(ctx, cache);
        }

        // update buffers
        let _prepare_span = tracing::span!(tracing::Level::INFO, "prepare").entered();
        self.camera_buffer.write(ctx, &self.camera.uniform());
        self.lights_buffer.write(ctx, &self.lights);

        // clear textures
        // TODO: clear in some other pass
        self.hdr_framebuffer_1.clear(ctx, wgpu::Color::BLACK);
        self.depth_buffer.clear(ctx);

        let mut draw_calls: Vec<DrawCall> = vec![
            DrawCall {
                mesh: self.plane_mesh.clone(),
                transform: Transform3D::default()
                    .with_pos(vec3(0.0, -2.0, 0.0))
                    .with_rot(Quat::from_rotation_x(-PI / 2.0))
                    .with_scale(Vec3::ONE * PLANE_SIZE),
            },
            // add meshes
            DrawCall {
                mesh: self.helmet_mesh.clone(),
                transform: Transform3D::default()
                    .with_rot(Quat::from_rotation_y(time::time_since_start(ctx)))
                    .with_pos(vec3(0.0, 0.0, 0.0))
                    .with_scale(Vec3::ONE * 1.0),
            },
            DrawCall {
                mesh: self.helmet_mesh.clone(),
                transform: Transform3D::default()
                    .with_pos(vec3(-3.0, 10.0, 0.0))
                    .with_scale(Vec3::ONE * 1.0),
            },
            DrawCall {
                mesh: self.ak47_mesh.clone(),
                transform: Transform3D::default()
                    .with_pos(vec3(3.0, 0.0, -1.0))
                    .with_scale(Vec3::ONE * 1.0),
            },
        ];

        let amount = 20;
        let gap = 20;
        for x in -amount..amount {
            for z in -amount..amount {
                draw_calls.push(DrawCall {
                    mesh: self.helmet_mesh.clone(),
                    transform: Transform3D::default()
                        .with_pos(vec3(gap as f32 * x as f32, 10.0, gap as f32 * z as f32))
                        .with_scale(Vec3::ONE * 1.0),
                });
            }
        }

        // shadow pass
        // TODO: scuffed
        let meshes = draw_calls
            .clone()
            .into_iter()
            .map(|draw_call| (draw_call.mesh, draw_call.transform))
            .collect::<Vec<_>>();

        self.lights.main_light_dir = vec3(1.0, -1.0, 0.0);
        _prepare_span.exit();

        let _shadow_span = tracing::span!(tracing::Level::INFO, "shadow pass").entered();
        self.shadow_pass.render(
            ctx,
            cache,
            &meshes,
            &self.camera,
            // TODO: doesnt work for (0,-1,0)
            self.lights.main_light_dir.normalize(),
        );
        _shadow_span.exit();

        // self.pbr_renderer
        //     .render_bounding_boxes(ctx, cache, &mut self.gizmo_renderer, &self.camera);
        let _pbr_span = span!(tracing::Level::INFO, "pbr").entered();
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
        _pbr_span.exit();

        let _gizmo_span = span!(tracing::Level::INFO, "gizmo").entered();
        self.gizmo_renderer.render(
            ctx,
            self.hdr_framebuffer_1.view_ref(),
            self.hdr_framebuffer_1.format(),
            &self.camera_buffer,
        );
        _gizmo_span.exit();

        self.framebuffer_renderer.render(
            ctx,
            cache,
            self.hdr_framebuffer_1.view(),
            screen_view,
            render::surface_format(ctx),
        );

        _render_span.exit();

        CallbackResult::Continue
    }

    #[no_mangle]
    fn render_egui(
        &mut self,
        ctx: &mut Context,
        cache: &mut AssetCache,
        egui_ctx: &mut egui_ui::EguiContext,
    ) -> CallbackResult {
        if input::key_just_pressed(ctx, input::KeyCode::Escape) {
            self.toggle_debug_ui = !self.toggle_debug_ui;
        }
        if !self.toggle_debug_ui {
            return CallbackResult::Continue;
        }

        egui_ctx.ctx().style_mut(|style| {
            style.text_styles = [
                (
                    egui::TextStyle::Heading,
                    egui::FontId::new(20.0, egui::FontFamily::Proportional),
                ),
                (
                    egui::TextStyle::Body,
                    egui::FontId::new(18.0, egui::FontFamily::Proportional),
                ),
            ]
            .into();
        });

        egui::Window::new("Profiling").show(egui_ctx.ctx(), |ui| {
            ui.heading("Total:");
            ui.label(format!("{:.4} fps", time::fps(ctx)));
            ui.label(format!("{:.4} ms", time::frame_time(ctx) * 1000.0));
            ui.heading("CPU:");
            for (label, time) in profile::profiler(ctx).get_cpu_samples() {
                ui.label(format!("{:.4} {}", time * 1000.0, label));
            }

            ui.heading("GPU:");
            for (label, time) in profile::profiler(ctx).get_gpu_samples() {
                ui.label(format!("{:.4} {}", time * 1000.0, label));
            }
        });

        //
        // render depth
        //

        for i in 0..3 {
            let shadow_view = render::TextureViewBuilder::new(self.shadow_pass.shadow_map.clone())
                .base_array_layer(i)
                .dimension(wgpu::TextureViewDimension::D2)
                .build(ctx);
            let shadow_view_rgb = render::TextureViewBuilder::new(self.shadow_map_rgb.clone())
                .base_array_layer(i)
                .dimension(wgpu::TextureViewDimension::D2)
                .build(ctx);
            self.framebuffer_renderer.render_depth(
                ctx,
                cache,
                shadow_view,
                &shadow_view_rgb,
                wgpu::TextureFormat::Rgba8Unorm,
                &self.camera_buffer,
                None,
            );
        }

        let mut tex_ids = Vec::new();
        for i in 0..gbase_utils::MAX_SHADOW_CASCADES {
            let tex_id = egui_ctx.register_wgpu_texture_cached(
                ctx,
                render::TextureViewBuilder::new(self.shadow_map_rgb.clone())
                    .base_array_layer(i)
                    .dimension(wgpu::TextureViewDimension::D2),
                render::SamplerBuilder::new(),
            );
            tex_ids.push(tex_id);
        }

        egui::Window::new("Depth maps").show(egui_ctx.ctx(), |ui| {
            for tex_id in tex_ids {
                ui.image(SizedTexture::new(tex_id, [256.0, 256.0]));
            }
        });

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
        self.ldr_framebuffer.resize(ctx, new_size);
        self.camera.resize(new_size);

        CallbackResult::Continue
    }

    #[no_mangle]
    #[cfg(feature = "hot_reload")]
    fn hot_reload(&mut self, ctx: &mut Context, _cache: &mut AssetCache) {
        ContextBuilder::init_logging_with_profiler::<Self>(ctx);
    }
}
