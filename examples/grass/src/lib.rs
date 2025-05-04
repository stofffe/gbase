mod grass_renderer;

use gbase::{
    filesystem, glam, input, log,
    render::{self, MeshBuilder},
    time, wgpu,
    winit::{dpi::PhysicalSize, keyboard::KeyCode, window::Window},
    Callbacks, Context,
};
use gbase_utils::{
    AssetCache, AssetHandle, CameraFrustum, GpuMaterial, PbrLightUniforms, PbrMaterial, PixelCache,
    Transform3D,
};
use glam::{vec2, vec3, vec4, Quat, Vec3, Vec4};
use grass_renderer::GrassRenderer;
use std::{f32::consts::PI, fmt::write, sync::Arc};

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub async fn run() {
    gbase::run::<App>().await;
}

const CAMERA_MOVE_SPEED: f32 = 15.0;
const PLANE_SIZE: f32 = 500.0;
const PLANE_COLOR: [f32; 4] = [0.0, 0.4, 0.0, 1.0];

pub struct App {
    camera: gbase_utils::Camera,
    camera_buffer: render::UniformBuffer<gbase_utils::CameraUniform>,
    frustum_buffer: render::UniformBuffer<CameraFrustum>,
    light_buffer: render::UniformBuffer<PbrLightUniforms>,
    light: PbrLightUniforms,

    // light: Vec3,
    depth_buffer: render::DepthBuffer,
    grass_renderer: GrassRenderer,
    gui_renderer: gbase_utils::GUIRenderer,
    gizmo_renderer: gbase_utils::GizmoRenderer,
    pbr_renderer: gbase_utils::PbrRenderer,

    shader_cache: AssetCache<render::ShaderBuilder, wgpu::ShaderModule>,
    mesh_cache: AssetCache<render::Mesh, render::GpuMesh>,
    image_cache: AssetCache<render::Image, render::GpuImage>,
    pixel_cache: PixelCache,

    plane_mesh: AssetHandle<render::Mesh>,
    plane_material: Arc<GpuMaterial>,

    paused: bool,

    framebuffer: render::FrameBuffer,
    framebuffer_renderer: gbase_utils::TextureRenderer,
}

impl Callbacks for App {
    #[no_mangle]
    fn init_ctx() -> gbase::ContextBuilder {
        gbase::ContextBuilder::new()
            .log_level(gbase::LogLevel::Info)
            .window_attributes(Window::default_attributes().with_maximized(true))
            // .device_features(wgpu::Features::POLYGON_MODE_LINE)
            .vsync(false)
    }
    #[no_mangle]
    fn new(ctx: &mut Context) -> Self {
        let mut shader_cache = AssetCache::new();
        let mut mesh_cache = AssetCache::new();
        let mut image_cache = AssetCache::new();
        let mut pixel_cache = PixelCache::new();

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
        let framebuffer_renderer = gbase_utils::TextureRenderer::new(ctx);

        // Camera
        let camera = gbase_utils::Camera::new(gbase_utils::CameraProjection::perspective(PI / 2.0))
            .pos(vec3(-1.0, 8.0, -1.0))
            .yaw(PI / 4.0);

        let camera_buffer = render::UniformBufferBuilder::new(render::UniformBufferSource::Empty)
            .label("camera buf")
            .usage(wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST)
            .build(ctx);
        let frustum_buffer = render::UniformBufferBuilder::new(render::UniformBufferSource::Empty)
            .label("frustum")
            .usage(wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST)
            .build(ctx);
        frustum_buffer.write(ctx, &camera.calculate_frustum(ctx)); // TODO: remove
                                                                   // .build(ctx);
        let light = gbase_utils::PbrLightUniforms {
            main_light_dir: vec3(1.0, -1.0, 1.0).normalize(),
            main_light_insensity: 1.0,
        };
        let light_buffer =
            render::UniformBufferBuilder::new(render::UniformBufferSource::Empty).build(ctx);

        // Renderers
        // let deferred_buffers = gbase_utils::DeferredBuffers::new(ctx);
        // let deferred_renderer = gbase_utils::DeferredRenderer::new(ctx, &mut shader_cache);
        let depth_buffer = render::DepthBufferBuilder::new()
            .screen_size(ctx)
            .build(ctx);
        let grass_renderer = GrassRenderer::new(ctx, &mut shader_cache);
        let pbr_renderer = gbase_utils::PbrRenderer::new(ctx);
        let gui_renderer = gbase_utils::GUIRenderer::new(
            ctx,
            1024,
            &filesystem::load_b!("fonts/meslo.ttf").unwrap(),
            gbase_utils::DEFAULT_SUPPORTED_CHARS,
        );
        let gizmo_renderer = gbase_utils::GizmoRenderer::new(ctx);

        let plane_mesh = mesh_cache.allocate(
            MeshBuilder::quad()
                .build()
                .extract_attributes(pbr_renderer.required_attributes().clone()),
        );
        let plane_material = Arc::new(
            PbrMaterial {
                base_color_texture: None,
                color_factor: [0.3, 1.0, 0.2, 1.0],
                metallic_roughness_texture: None,
                roughness_factor: 1.0,
                metallic_factor: 0.0,
                occlusion_texture: None,
                occlusion_strength: 0.0,
                normal_texture: None,
                normal_scale: 1.0,
                emissive_texture: None,
                emissive_factor: [0.0, 0.0, 0.0],
            }
            .to_material(&mut image_cache, &mut pixel_cache),
        );

        Self {
            shader_cache,
            mesh_cache,
            image_cache,
            pixel_cache,

            camera,
            camera_buffer,
            frustum_buffer,
            gui_renderer,
            gizmo_renderer,
            pbr_renderer,
            plane_mesh,
            light_buffer,
            light,
            plane_material,
            depth_buffer,
            grass_renderer,

            paused: false,

            framebuffer,
            framebuffer_renderer,
        }
    }

    #[no_mangle]
    fn render(&mut self, ctx: &mut Context, screen_view: &wgpu::TextureView) -> bool {
        self.shader_cache.check_watched_files(ctx);
        self.image_cache.check_watched_files(ctx);

        // update buffers
        self.camera_buffer.write(ctx, &self.camera.uniform(ctx));
        self.frustum_buffer
            .write(ctx, &self.camera.calculate_frustum(ctx));
        self.framebuffer.clear(ctx, wgpu::Color::BLACK);
        self.depth_buffer.clear(ctx);

        // self.light.main_light_insensity = 3.0;
        self.light_buffer.write(ctx, &self.light);

        // Render

        self.pbr_renderer.add_mesh(
            self.plane_mesh.clone(),
            self.plane_material.clone(),
            Transform3D::default()
                .with_pos(vec3(self.camera.pos.x, 0.0, self.camera.pos.z))
                .with_rot(Quat::from_rotation_x(-PI / 2.0))
                .with_scale(Vec3::ONE * 1000.0),
        );
        self.pbr_renderer.render(
            ctx,
            &mut self.mesh_cache,
            &mut self.image_cache,
            self.framebuffer.view_ref(),
            self.framebuffer.format(),
            &self.camera,
            &self.camera_buffer,
            &self.light_buffer,
            &self.depth_buffer,
        );

        self.grass_renderer.render(
            ctx,
            &mut self.shader_cache,
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
        self.gui_renderer.display_debug_info(ctx);
        self.gui_renderer
            .render(ctx, self.framebuffer.view_ref(), self.framebuffer.format());
        self.framebuffer_renderer.render(
            ctx,
            self.framebuffer.view(),
            screen_view,
            render::surface_format(ctx),
        );

        false
    }

    #[no_mangle]
    fn resize(&mut self, ctx: &mut Context, new_size: PhysicalSize<u32>) {
        self.gizmo_renderer.resize(ctx, new_size);
        self.framebuffer.resize(ctx, new_size);
        self.depth_buffer.resize(ctx, new_size);
        self.gui_renderer.resize(ctx, new_size);
    }

    #[no_mangle]
    fn update(&mut self, ctx: &mut Context) -> bool {
        // pausing
        if input::key_just_pressed(ctx, KeyCode::Escape) {
            self.paused = !self.paused;
        }
        if self.paused {
            self.gui_renderer.text(
                "pause (esc)",
                vec2(0.0, 0.0),
                vec2(0.5, 0.5),
                0.05,
                vec4(1.0, 1.0, 1.0, 1.0),
                false,
            );
            return false;
        }

        // self.plane_transform.pos.x = self.camera.pos.x;
        // self.plane_transform.pos.z = self.camera.pos.z;

        self.camera.flying_controls(ctx);

        // debug camera pos
        if input::key_pressed(ctx, KeyCode::KeyC) {
            log::info!("{}", self.camera.pos);
        }

        // debug text
        if input::key_pressed(ctx, KeyCode::KeyF) {
            let avg_ms = time::frame_time(ctx) * 1000.0;
            let avg_fps = 1.0 / time::frame_time(ctx);
            let strings = [format!("fps {avg_fps:.5}"), format!("ms  {avg_ms:.5}")];

            const DEBUG_HEIGH: f32 = 0.05;
            const DEBUG_COLOR: Vec4 = vec4(1.0, 1.0, 1.0, 1.0);
            for (i, text) in strings.iter().enumerate() {
                // self.gui_renderer.text(
                //     text,
                //     vec2(0.0, DEBUG_HEIGH * i as f32),
                //     vec2(0.5, 0.5),
                //     DEBUG_HEIGH,
                //     DEBUG_COLOR,
                //     false,
                // );
            }
        }

        false
    }
}
