use gbase::{
    input::{self, KeyCode},
    render::{self, CameraUniform, Transform},
    time, Callbacks, Context, ContextBuilder, LogLevel,
};
use glam::{vec2, vec3, Quat, Vec3};

struct App {
    camera: render::PerspectiveCamera,
    camera_buffer: render::UniformBuffer<CameraUniform>,
    gizmo_renderer: render::GizmoRenderer,
}

impl App {
    async fn new(ctx: &mut Context) -> Self {
        let camera = render::PerspectiveCamera::new();
        let camera_buffer =
            render::UniformBufferBuilder::new(render::UniformBufferSource::Empty).build(ctx);
        let gizmo_renderer =
            render::GizmoRenderer::new(ctx, wgpu::TextureFormat::Bgra8UnormSrgb, &camera_buffer)
                .await;

        Self {
            camera,
            camera_buffer,
            gizmo_renderer,
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
    fn init(&mut self, _ctx: &mut Context) {
        self.camera.pos = vec3(0.0, 0.0, 1.0);
    }
    fn render(&mut self, ctx: &mut Context, screen_view: &wgpu::TextureView) -> bool {
        let t = time::time_since_start(ctx);
        self.camera_buffer.write(ctx, &self.camera.uniform(ctx));

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
    fn resize(&mut self, ctx: &mut Context) {
        self.gizmo_renderer.resize_screen(ctx);
    }
}

#[pollster::main]
pub async fn main() {
    let (mut ctx, ev) = ContextBuilder::new()
        .log_level(LogLevel::Info)
        .build()
        .await;
    let app = App::new(&mut ctx).await;
    gbase::run(app, ctx, ev);
}
