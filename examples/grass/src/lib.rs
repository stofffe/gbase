mod grass_renderer;

use gbase::{
    asset::{self, AssetHandle},
    filesystem,
    glam::{vec2, vec3, vec4, Quat, Vec3},
    input, profile, render, time, tracing, wgpu,
    winit::{dpi::PhysicalSize, keyboard::KeyCode, window::Window},
    CallbackResult, Callbacks, Context,
};
use gbase_utils::{
    CameraFrustum, Direction, MeshLod, PbrLightUniforms, PixelCache, SizeKind, Transform3D, Widget,
};
use grass_renderer::GrassRenderer;
use std::f32::consts::PI;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub async fn run() {
    gbase::run::<App>().await;
}

const PLANE_SIZE: f32 = 1000.0;
const PLANE_COLOR: [f32; 4] = [0.3, 1.0, 0.2, 1.0];

pub struct App {
    camera: gbase_utils::Camera,
    camera_buffer: render::UniformBuffer<gbase_utils::CameraUniform>,
    frustum_buffer: render::UniformBuffer<CameraFrustum>,
    light_buffer: render::UniformBuffer<PbrLightUniforms>,
    light: PbrLightUniforms,

    depth_buffer: render::DepthBuffer,
    grass_renderer: GrassRenderer,
    gui_renderer: gbase_utils::GUIRenderer,
    gizmo_renderer: gbase_utils::GizmoRenderer,
    pbr_renderer: gbase_utils::PbrRenderer,

    plane_mesh: AssetHandle<MeshLod>,

    paused: bool,

    framebuffer: render::FrameBuffer,
    framebuffer_renderer: gbase_utils::TextureRenderer,

    shadow_pass: gbase_utils::ShadowPass,
}

impl Callbacks for App {
    #[no_mangle]
    fn init_ctx() -> gbase::ContextBuilder {
        gbase::ContextBuilder::new()
            .log_level(tracing::Level::INFO)
            .gpu_profiler_enabled(false)
            .window_attributes(Window::default_attributes().with_maximized(true))
            .device_features(wgpu::Features::TIMESTAMP_QUERY)
            .vsync(false)
    }

    #[no_mangle]
    fn new(ctx: &mut Context, cache: &mut gbase::asset::AssetCache) -> Self {
        // Framebuffer
        let framebuffer = render::FrameBufferBuilder::new()
            .screen_size(ctx)
            .format(wgpu::TextureFormat::Rgba8Unorm)
            .usage(
                wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_SRC
                    | wgpu::TextureUsages::STORAGE_BINDING,
            )
            .build(ctx);
        let framebuffer_renderer = gbase_utils::TextureRenderer::new(ctx, cache);

        // Camera
        let camera = gbase_utils::Camera::new_with_screen_size(
            ctx,
            gbase_utils::CameraProjection::perspective(PI / 2.0),
        )
        .pos(vec3(-1.0, 8.0, -1.0))
        .yaw(PI / 4.0);

        let camera_buffer = render::UniformBufferBuilder::new()
            .label("camera buf")
            .usage(wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST)
            .build(ctx);
        let frustum_buffer = render::UniformBufferBuilder::new()
            .label("frustum")
            .usage(wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST)
            .build(ctx);

        let light = gbase_utils::PbrLightUniforms {
            main_light_dir: vec3(1.0, -1.0, 1.0).normalize(),
            main_light_insensity: 1.0,
        };
        let light_buffer = render::UniformBufferBuilder::new().build(ctx);

        // Renderers
        let depth_buffer = render::DepthBufferBuilder::new()
            .screen_size(ctx)
            .build(ctx);
        let grass_renderer = GrassRenderer::new(ctx, cache);
        let pbr_renderer = gbase_utils::PbrRenderer::new(ctx, cache);
        let gui_renderer = gbase_utils::GUIRenderer::new(
            ctx,
            1024,
            &filesystem::load_b!("fonts/meslo.ttf").unwrap(),
            gbase_utils::DEFAULT_SUPPORTED_CHARS,
        );
        let gizmo_renderer = gbase_utils::GizmoRenderer::new(ctx);

        let plane_mesh_handle = asset::AssetBuilder::insert(
            render::MeshBuilder::quad()
                .build()
                .with_extracted_attributes(pbr_renderer.required_attributes().clone()),
        )
        .build(cache);
        let plane_material = gbase_utils::Material::default(cache).with_color_factor(PLANE_COLOR);
        let plane_material = cache.insert(plane_material);
        let plane_mesh = cache.insert(MeshLod::from_single_lod(plane_mesh_handle, plane_material));

        let shadow_pass = gbase_utils::ShadowPass::new(ctx, cache);

        Self {
            camera,
            camera_buffer,
            frustum_buffer,
            gui_renderer,
            gizmo_renderer,
            pbr_renderer,
            plane_mesh,
            light_buffer,
            light,
            depth_buffer,
            grass_renderer,

            paused: false,

            framebuffer,
            framebuffer_renderer,

            shadow_pass,
        }
    }

    #[no_mangle]
    fn render(
        &mut self,
        ctx: &mut Context,
        cache: &mut gbase::asset::AssetCache,
        screen_view: &wgpu::TextureView,
    ) -> CallbackResult {
        // pausing
        if input::key_just_pressed(ctx, KeyCode::Escape) {
            self.paused = !self.paused;
        }
        if self.paused {
            // self.gui_renderer.text(
            //     "pause (esc)",
            //     vec2(0.0, 0.0),
            //     vec2(22.5, 22.5),
            //     100.0,
            //     vec4(1.0, 1.0, 1.0, 1.0),
            //     false,
            // );
        } else {
            self.camera.flying_controls(ctx);
        }

        // TODO: temp

        // update buffers
        self.camera_buffer.write(ctx, &self.camera.uniform());
        let frustum = self.camera.calculate_frustum();

        self.frustum_buffer.write(ctx, &frustum);
        self.framebuffer.clear(ctx, wgpu::Color::BLACK);
        self.depth_buffer.clear(ctx);

        self.light_buffer.write(ctx, &self.light);

        // Render
        let meshes = vec![(
            self.plane_mesh.clone(),
            Transform3D::default()
                .with_pos(vec3(self.camera.pos.x, 0.0, self.camera.pos.z))
                .with_rot(Quat::from_rotation_x(-PI / 2.0))
                .with_scale(Vec3::ONE * PLANE_SIZE),
        )];

        self.shadow_pass
            .render(ctx, cache, &meshes, &self.camera, self.light.main_light_dir);

        self.pbr_renderer.render(
            ctx,
            cache,
            self.framebuffer.view_ref(),
            self.framebuffer.format(),
            &self.camera,
            &self.camera_buffer,
            &self.light_buffer,
            &self.depth_buffer,
            &self.camera.calculate_frustum(),
            meshes,
            &self.shadow_pass.shadow_map,
            &self.shadow_pass.light_matrices_buffer,
            &self.shadow_pass.light_matrices_distances,
        );

        self.grass_renderer.render(
            ctx,
            cache,
            &self.camera,
            &self.camera_buffer,
            &self.frustum_buffer,
            grass_renderer::RenderMode::Forward {
                view: self.framebuffer.view_ref(),
                view_format: self.framebuffer.format(),
                depth_buffer: &self.depth_buffer,
            },
        );

        self.gizmo_renderer.render(
            ctx,
            self.framebuffer.view_ref(),
            self.framebuffer.format(),
            &self.camera_buffer,
        );
        // self.gui_renderer.display_debug_info(ctx);
        let outer = Widget::new()
            .label("outer")
            .width(SizeKind::PercentOfParent(1.0))
            .height(SizeKind::PercentOfParent(1.0))
            .direction(Direction::Column)
            .gap(20.0)
            .padding(20.0);
        outer.layout(&mut self.gui_renderer, |renderer| {
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
        self.gui_renderer
            .render(ctx, self.framebuffer.view_ref(), self.framebuffer.format());
        self.framebuffer_renderer.render(
            ctx,
            cache,
            self.framebuffer.view(),
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
        new_size: PhysicalSize<u32>,
    ) -> CallbackResult {
        self.gizmo_renderer.resize(ctx, new_size);
        self.framebuffer.resize(ctx, new_size);
        self.depth_buffer.resize(ctx, new_size);
        self.gui_renderer.resize(ctx, new_size);
        self.camera.resize(new_size);

        CallbackResult::Continue
    }
}

impl App {
    #[no_mangle]
    fn hot_reload(&mut self, ctx: &mut Context) {
        Self::init_ctx().init_logging();
    }
}
