#![allow(dead_code)]

use gbase::{
    glam::{vec3, Quat, Vec3},
    input::{self, KeyCode},
    render::{self},
    time, wgpu,
    winit::dpi::PhysicalSize,
    Callbacks, Context,
};
use gbase_utils::Transform3D;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub async fn run() {
    gbase::run::<App>().await;
}

struct App {
    camera: gbase_utils::PerspectiveCamera,
    camera_buffer: render::UniformBuffer<gbase_utils::CameraUniform>,
    gizmo_renderer: gbase_utils::GizmoRenderer,
}

const RED: Vec3 = vec3(1.0, 0.0, 0.0);
const GREEN: Vec3 = vec3(0.0, 1.0, 0.0);
const BLUE: Vec3 = vec3(0.0, 0.0, 1.0);
const CYAN: Vec3 = vec3(0.0, 1.0, 1.0);
const MAGENTA: Vec3 = vec3(1.0, 0.0, 1.0);
const YELLOW: Vec3 = vec3(1.0, 1.0, 0.0);
const WHITE: Vec3 = vec3(1.0, 1.0, 1.0);

impl Callbacks for App {
    fn new(ctx: &mut Context) -> Self {
        let mut camera = gbase_utils::PerspectiveCamera::new();
        camera.pos = vec3(0.0, 0.0, 1.0);

        let camera_buffer =
            render::UniformBufferBuilder::new(render::UniformBufferSource::Empty).build(ctx);
        let gizmo_renderer = gbase_utils::GizmoRenderer::new(
            ctx,
            wgpu::TextureFormat::Bgra8UnormSrgb,
            &camera_buffer,
        );

        Self {
            camera,
            camera_buffer,
            gizmo_renderer,
        }
    }

    fn render(&mut self, ctx: &mut Context, screen_view: &wgpu::TextureView) -> bool {
        let t = time::time_since_start(ctx);
        self.camera_buffer.write(ctx, &self.camera.uniform(ctx));

        self.gizmo_renderer
            .draw_sphere(0.01, &Transform3D::default(), WHITE);
        self.gizmo_renderer.draw_sphere(
            0.5,
            &Transform3D::new(vec3(t.sin(), 0.0, 0.0), Quat::from_rotation_x(t), Vec3::ONE),
            BLUE,
        );
        self.gizmo_renderer
            .draw_cube(&Transform3D::from_scale(vec3(0.5, 1.0, 0.5)), GREEN);

        self.gizmo_renderer.draw_quad(
            &Transform3D::new(Vec3::ZERO, Quat::from_rotation_y(t), vec3(2.0, 1.0, 0.0)),
            WHITE,
        );
        self.gizmo_renderer.draw_circle(
            1.0,
            &Transform3D::new(Vec3::ZERO, Quat::default(), Vec3::ONE),
            WHITE,
        );

        self.gizmo_renderer.render(ctx, screen_view);
        false
    }

    fn update(&mut self, ctx: &mut Context) -> bool {
        let dt = gbase::time::delta_time(ctx);

        if input::key_just_pressed(ctx, KeyCode::KeyR) {
            self.camera.yaw = 0.0;
            self.camera.pitch = 0.0;
        }

        // Camera rotation
        if input::mouse_button_pressed(ctx, input::MouseButton::Left) {
            let (mouse_dx, mouse_dy) = input::mouse_delta(ctx);
            self.camera.yaw -= 1.0 * dt * mouse_dx;
            self.camera.pitch -= 1.0 * dt * mouse_dy;
        }

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
        if camera_movement_dir != Vec3::ZERO {
            self.camera.pos += camera_movement_dir.normalize() * dt;
        }

        // Camera zoom
        let (_, scroll_y) = input::scroll_delta(ctx);
        self.camera.fov += scroll_y * dt;
        false
    }
    fn resize(&mut self, ctx: &mut Context, new_size: PhysicalSize<u32>) {
        self.gizmo_renderer
            .resize(ctx, new_size.width, new_size.height);
    }
}
