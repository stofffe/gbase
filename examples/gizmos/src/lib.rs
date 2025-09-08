#![allow(dead_code)]

use std::f32::consts::PI;

use gbase::{
    glam::{vec3, Quat, Vec3},
    input::{self, KeyCode},
    render::{self},
    time, wgpu,
    winit::dpi::PhysicalSize,
    CallbackResult, Callbacks, Context,
};
use gbase_utils::Transform3D;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub async fn run() {
    gbase::run::<App>().await;
}

struct App {
    camera: gbase_utils::Camera,
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
    #[no_mangle]
    fn new(ctx: &mut Context, _cache: &mut gbase::asset::AssetCache) -> Self {
        let camera = gbase_utils::Camera::new_with_screen_size(
            ctx,
            gbase_utils::CameraProjection::perspective(PI / 2.0),
        )
        .pos(vec3(0.0, 0.0, 1.0));

        let camera_buffer = render::UniformBufferBuilder::new().build(ctx);
        let gizmo_renderer = gbase_utils::GizmoRenderer::new(ctx);

        Self {
            camera,
            camera_buffer,
            gizmo_renderer,
        }
    }

    #[no_mangle]
    fn render(
        &mut self,
        ctx: &mut Context,
        _cache: &mut gbase::asset::AssetCache,
        screen_view: &wgpu::TextureView,
    ) -> CallbackResult {
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
        self.camera.zoom(input::scroll_delta(ctx).1 * dt);

        let t = time::time_since_start(ctx);
        self.camera_buffer.write(ctx, &self.camera.uniform());

        // let t: f32 = 10.5;
        // self.gizmo_renderer
        //     .draw_sphere(&Transform3D::from_scale(Vec3::ONE * 0.01), WHITE);
        self.gizmo_renderer.draw_sphere(
            &Transform3D::new(
                vec3(t.sin(), 0.0, 0.0),
                Quat::from_rotation_x(t),
                Vec3::ONE * 0.5,
            ),
            BLUE,
        );
        self.gizmo_renderer
            .draw_cube(&Transform3D::from_scale(vec3(0.5, 1.0, 0.5)), GREEN);
        self.gizmo_renderer.draw_quad(
            &Transform3D::new(Vec3::ZERO, Quat::from_rotation_y(t), vec3(2.0, 1.0, 0.0)),
            WHITE,
        );
        // self.gizmo_renderer.draw_circle(
        //     &Transform3D::new(Vec3::ZERO, Quat::default(), Vec3::ONE),
        //     WHITE,
        // );
        self.gizmo_renderer.draw_sphere(
            &Transform3D::new(Vec3::ZERO, Quat::default(), Vec3::ONE),
            WHITE,
        );

        self.gizmo_renderer.render(
            ctx,
            screen_view,
            render::surface_format(ctx),
            &self.camera_buffer,
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
        self.camera.resize(new_size);
        CallbackResult::Continue
    }
}
