use gbase::{
    input::{self, KeyCode},
    render::{self, Transform},
    time, Callbacks, Context, ContextBuilder, LogLevel,
};
use glam::{vec2, vec3, Quat, Vec3};

struct App {
    gizmo_renderer: render::GizmoRenderer,
    camera: render::PerspectiveCamera,
}

impl App {
    fn new(ctx: &Context) -> Self {
        let gizmo_renderer = render::GizmoRenderer::new(ctx);
        let camera = render::PerspectiveCamera::new().pos(vec3(0.0, 0.0, 1.0));

        Self {
            gizmo_renderer,
            camera,
        }
    }
}

const RED: Vec3 = vec3(1.0, 0.0, 0.0);
const GREEN: Vec3 = vec3(0.0, 1.0, 0.0);
const BLUE: Vec3 = vec3(0.0, 0.0, 1.0);
const CYAN: Vec3 = vec3(0.0, 1.0, 1.0);
const MAGENTA: Vec3 = vec3(1.0, 0.0, 1.0);
const YELLOW: Vec3 = vec3(1.0, 1.0, 0.0);
const WHITE: Vec3 = vec3(1.0, 1.0, 1.0);

impl Callbacks for App {
    fn render(&mut self, ctx: &mut Context, screen_view: &wgpu::TextureView) -> bool {
        let t = time::time_since_start(ctx);

        self.gizmo_renderer
            .draw_sphere(0.01, &Transform::default(), WHITE);
        self.gizmo_renderer.draw_sphere(
            0.5,
            &Transform::new(vec3(t.sin(), 0.0, 0.0), Quat::from_rotation_x(t), Vec3::ONE),
            BLUE,
        );
        self.gizmo_renderer
            .draw_cube(vec3(0.5, 1.0, 0.5), &Transform::default(), GREEN);

        self.gizmo_renderer.draw_quad(
            vec2(2.0, 1.0),
            &Transform::new(Vec3::ZERO, Quat::from_rotation_y(t), Vec3::ONE),
            WHITE,
        );
        self.gizmo_renderer.draw_circle(
            1.0,
            &Transform::new(Vec3::ZERO, Quat::default(), Vec3::ONE),
            WHITE,
        );

        self.gizmo_renderer
            .render(ctx, screen_view, &mut self.camera);
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
    fn resize(&mut self, ctx: &mut Context) {
        self.gizmo_renderer.resize(ctx);
    }
}

#[pollster::main]
pub async fn main() {
    let (ctx, ev) = ContextBuilder::new()
        .log_level(LogLevel::Info)
        .build()
        .await;
    let app = App::new(&ctx);
    gbase::run(app, ctx, ev);
}
