mod grass;
use gbase::{
    collision, filesystem, input,
    render::{self, CameraUniform, DeferredRenderer, MeshRenderer, Transform, TransformUniform},
    time, Callbacks, Context, ContextBuilder, LogLevel,
};
use glam::{vec2, vec3, vec4, Quat, Vec3, Vec4};
use grass::GrassRenderer;
use std::f32::consts::PI;
use winit::{
    keyboard::KeyCode,
    window::{CursorGrabMode, WindowBuilder},
};

#[pollster::main]
pub async fn main() {
    let (ctx, ev) = ContextBuilder::new()
        .window_builder(WindowBuilder::new().with_maximized(true))
        .log_level(LogLevel::Warn)
        .vsync(false)
        .build()
        .await;
    gbase::run::<App>(ctx, ev);
}

const CAMERA_MOVE_SPEED: f32 = 15.0;

const PLANE_SIZE: f32 = 500.0;
const PLANE_COLOR: [f32; 4] = [0.0, 0.4, 0.0, 1.0];

struct App {
    camera: render::PerspectiveCamera,
    camera_buffer: render::UniformBuffer<CameraUniform>,
    light: Vec3,
    light_buffer: render::UniformBuffer<Vec3>,
    deferred_buffers: render::DeferredBuffers,

    mesh_renderer: render::MeshRenderer,
    deferred_renderer: render::DeferredRenderer,
    grass_renderer: GrassRenderer,
    gui_renderer: render::GUIRenderer,
    gizmo_renderer: render::GizmoRenderer,

    paused: bool,

    plane: render::GpuDrawCall,
    plane_transform: render::Transform,
    plane_transform_buffer: render::UniformBuffer<TransformUniform>,

    framebuffer: render::FrameBuffer,
    framebuffer_renderer: render::TextureRenderer,
    sobel_filter: render::SobelFilter,
    gamma_correction: render::GammaCorrection,
}

impl Callbacks for App {
    fn new(ctx: &mut Context) -> Self {
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
        let framebuffer_renderer =
            render::TextureRenderer::new(ctx, render::surface_config(ctx).format);

        // Camera
        let mut camera = render::PerspectiveCamera::new();
        camera.pos = vec3(-1.0, 8.0, -1.0);
        camera.yaw = PI / 4.0;

        let camera_buffer = render::UniformBufferBuilder::new(render::UniformBufferSource::Empty)
            .label("camera buf".to_string())
            .usage(wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST)
            .build(ctx);
        let light = vec3(10.0, 10.0, -10.0);
        let light_buffer =
            render::UniformBufferBuilder::new(render::UniformBufferSource::Empty).build(ctx);

        // Renderers
        let deferred_buffers = render::DeferredBuffers::new(ctx);
        let mesh_renderer = render::MeshRenderer::new(ctx, &deferred_buffers);
        let deferred_renderer = render::DeferredRenderer::new(
            ctx,
            framebuffer.format(),
            &deferred_buffers,
            &camera_buffer,
            &light_buffer,
        );
        let grass_renderer = GrassRenderer::new(ctx, &deferred_buffers, &camera_buffer);
        let gui_renderer = render::GUIRenderer::new(
            ctx,
            framebuffer.format(),
            1000 * 4,
            1000 * 6,
            &filesystem::load_b!("fonts/meslo.ttf").unwrap(),
            render::DEFAULT_SUPPORTED_CHARS,
        );
        let gizmo_renderer = render::GizmoRenderer::new(ctx, framebuffer.format(), &camera_buffer);

        // Plane mesh
        let plane_transform = render::Transform::new(
            // vec3(-10.0, 8.0, -10.0),
            vec3(0.0, 0.0, 0.0),
            Quat::from_rotation_x(-PI / 2.0),
            vec3(PLANE_SIZE, PLANE_SIZE, 1.0),
        );
        let plane_transform_buffer =
            render::UniformBufferBuilder::new(render::UniformBufferSource::Empty).build(ctx);
        let gpu_mesh = render::GpuMesh::from_mesh(
            ctx,
            render::Mesh::new(
                CENTERED_QUAD_VERTICES.to_vec(),
                CENTERED_QUAD_INDICES.to_vec(),
            ),
        );
        let gpu_material = render::GpuMaterial::from_material(
            ctx,
            render::Material {
                color_factor: PLANE_COLOR,
                roughness_factor: 0.0,
                metalness_factor: 0.0,
                occlusion_strength: 1.0,
                albedo: None,
                normal: None,
                roughness: None,
            },
        );
        let plane = render::GpuDrawCall::new(
            ctx,
            gpu_mesh,
            gpu_material,
            &plane_transform_buffer,
            &camera_buffer,
            &mesh_renderer,
        );

        let sobel_filter = render::SobelFilter::new(ctx);
        let gamma_correction = render::GammaCorrection::new(ctx);

        Self {
            camera,
            camera_buffer,
            light,
            light_buffer,
            deferred_buffers,
            mesh_renderer,
            deferred_renderer,
            grass_renderer,
            gui_renderer,
            gizmo_renderer,

            paused: false,

            plane,
            plane_transform,
            plane_transform_buffer,

            framebuffer,
            framebuffer_renderer,
            sobel_filter,
            gamma_correction,
        }
    }

    fn render(&mut self, ctx: &mut Context, screen_view: &wgpu::TextureView) -> bool {
        // log::warn!("RENDER");
        self.deferred_buffers.clear(ctx);

        // update buffers
        self.camera_buffer.write(ctx, &self.camera.uniform(ctx));
        let t = time::time_since_start(ctx);
        self.light = vec3(t.cos(), 1.0, t.sin()) * 10.0;
        self.light_buffer.write(ctx, &self.light);
        self.plane_transform_buffer
            .write(ctx, &self.plane_transform.uniform());

        // Render
        // let pt = time::ProfileTimer::new("GRASS");
        self.grass_renderer
            .render(ctx, &self.camera, &self.deferred_buffers);
        // pt.log();

        //Mesh
        self.mesh_renderer
            .render(ctx, &self.deferred_buffers, &[&self.plane]);
        self.deferred_renderer
            .render(ctx, self.framebuffer.view_ref());
        self.gui_renderer.render(ctx, self.framebuffer.view_ref());
        self.gizmo_renderer.draw_sphere(
            1.0,
            &Transform::new(self.light, Quat::IDENTITY, Vec3::ONE),
            vec3(1.0, 0.0, 0.0),
        );
        self.gizmo_renderer.render(ctx, self.framebuffer.view_ref());

        if input::key_pressed(ctx, KeyCode::KeyP) {
            self.sobel_filter.apply_filter(
                ctx,
                &self.framebuffer,
                &render::SobelFilterParams::new(1),
            );
        }

        // not super efficient but its fine :))
        let surface_conf = render::surface_config(ctx).format;
        if !surface_conf.is_srgb() {
            self.gamma_correction.apply(ctx, &self.framebuffer);
        }

        self.framebuffer_renderer
            .render(ctx, self.framebuffer.view(), screen_view);

        false
    }

    fn resize(&mut self, ctx: &mut Context) {
        log::info!("resize");
        self.gizmo_renderer.resize_screen(ctx);
        self.deferred_buffers.resize_screen(ctx);
        self.framebuffer.resize_screen(ctx);
        self.deferred_renderer.rebuild_bindgroup(
            ctx,
            &self.deferred_buffers,
            &self.camera_buffer,
            &self.light_buffer,
        );
    }

    fn update(&mut self, ctx: &mut Context) -> bool {
        // hot reload
        #[cfg(debug_assertions)]
        if input::key_just_pressed(ctx, KeyCode::KeyR) {
            self.grass_renderer =
                GrassRenderer::new(ctx, &self.deferred_buffers, &self.camera_buffer);
            self.deferred_renderer = DeferredRenderer::new(
                ctx,
                self.framebuffer.format(),
                &self.deferred_buffers,
                &self.camera_buffer,
                &self.light_buffer,
            );
            self.mesh_renderer = MeshRenderer::new(ctx, &self.deferred_buffers);
            println!("reload");
        }

        // pausing
        if input::key_just_pressed(ctx, KeyCode::Escape) {
            self.paused = !self.paused;

            #[cfg(not(target_arch = "wasm32"))]
            {
                render::window(ctx)
                    .set_cursor_grab(if self.paused {
                        CursorGrabMode::None
                    } else {
                        CursorGrabMode::Locked
                    })
                    .expect("could not set grab mode");
                render::window(ctx).set_cursor_visible(self.paused);
            }
        }
        if self.paused {
            self.gui_renderer.text(
                "pause (esc)",
                collision::Quad::new(vec2(0.0, 0.0), vec2(0.5, 0.5)),
                0.05,
                vec4(1.0, 1.0, 1.0, 1.0),
                false,
            );
            return false;
        }

        self.plane_transform.pos.x = self.camera.pos.x;
        self.plane_transform.pos.z = self.camera.pos.z;

        self.camera_movement(ctx);

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
                self.gui_renderer.text(
                    text,
                    collision::Quad::new(vec2(0.0, DEBUG_HEIGH * i as f32), vec2(0.5, 0.5)),
                    DEBUG_HEIGH,
                    DEBUG_COLOR,
                    false,
                );
            }
        }

        false
    }
}

impl App {
    fn camera_movement(&mut self, ctx: &mut Context) {
        let dt = gbase::time::delta_time(ctx);

        // Camera rotation
        let (mouse_dx, mouse_dy) = input::mouse_delta(ctx);
        self.camera.yaw -= 1.0 * dt * mouse_dx;
        self.camera.pitch -= 1.0 * dt * mouse_dy;

        // Camera movement
        let mut camera_movement_dir = Vec3::ZERO;
        if input::key_pressed(ctx, KeyCode::KeyW) {
            camera_movement_dir += self.camera.forward();
        }
        if input::key_pressed(ctx, KeyCode::KeyS) {
            camera_movement_dir -= self.camera.forward();
        }
        if input::key_pressed(ctx, KeyCode::KeyA) {
            camera_movement_dir -= self.camera.right();
        }
        if input::key_pressed(ctx, KeyCode::KeyD) {
            camera_movement_dir += self.camera.right();
        }
        camera_movement_dir.y = 0.0;
        if input::key_pressed(ctx, KeyCode::Space) {
            camera_movement_dir += self.camera.world_up();
        }
        if input::key_pressed(ctx, KeyCode::ShiftLeft) {
            camera_movement_dir -= self.camera.world_up();
        }
        if camera_movement_dir != Vec3::ZERO {
            if input::key_pressed(ctx, KeyCode::KeyM) {
                self.camera.pos += camera_movement_dir.normalize() * dt * CAMERA_MOVE_SPEED / 10.0;
            } else {
                self.camera.pos += camera_movement_dir.normalize() * dt * CAMERA_MOVE_SPEED;
            }
        }
    }
}

#[rustfmt::skip]
const CENTERED_QUAD_VERTICES: &[render::VertexFull] = &[
    render::VertexFull { position: [-0.5, -0.5, 0.0], color: PLANE_COLOR, uv: [0.0, 1.0], normal: [0.0, 0.0, 1.0], tangent: [1.0, 0.0, 0.0, 1.0] }, // bottom left
    render::VertexFull { position: [ 0.5, -0.5, 0.0], color: PLANE_COLOR, uv: [1.0, 1.0], normal: [0.0, 0.0, 1.0], tangent: [1.0, 0.0, 0.0, 1.0] }, // bottom right
    render::VertexFull { position: [ 0.5,  0.5, 0.0], color: PLANE_COLOR, uv: [1.0, 0.0], normal: [0.0, 0.0, 1.0], tangent: [1.0, 0.0, 0.0, 1.0] }, // top right

    render::VertexFull { position: [-0.5, -0.5, 0.0], color: PLANE_COLOR, uv: [0.0, 1.0], normal: [0.0, 0.0, 1.0], tangent: [1.0, 0.0, 0.0, 1.0] }, // bottom left
    render::VertexFull { position: [ 0.5,  0.5, 0.0], color: PLANE_COLOR, uv: [1.0, 0.0], normal: [0.0, 0.0, 1.0], tangent: [1.0, 0.0, 0.0, 1.0] }, // top right
    render::VertexFull { position: [-0.5,  0.5, 0.0], color: PLANE_COLOR, uv: [0.0, 0.0], normal: [0.0, 0.0, 1.0], tangent: [1.0, 0.0, 0.0, 1.0] }, // top left

];

#[rustfmt::skip]
const CENTERED_QUAD_INDICES: &[u32] = &[
    0, 1, 2,
    3, 4, 5
];
