use gbase::{
    input,
    render::{self, VertexNormal},
    time, Callbacks, Context, ContextBuilder, LogLevel,
};
use glam::{vec3, Vec3};
use winit::keyboard::KeyCode;

#[pollster::main]
pub async fn main() {
    let (mut ctx, ev) = ContextBuilder::new()
        .log_level(LogLevel::Warn)
        .vsync(false)
        .build()
        .await;
    let app = App::new(&mut ctx).await;
    gbase::run(app, ctx, ev).await;
}

struct App {
    vertex_buffer: render::VertexBuffer<VertexNormal>,
    pipeline: render::RenderPipeline,
    camera: render::PerspectiveCamera,
    depth_buffer: render::DepthBuffer,
}

const STEPS: i32 = 7;

fn bez(t: f32, start: Vec3, start_handle: Vec3, end_handle: Vec3, end: Vec3) -> Vec3 {
    start * (-t.powi(3) + 3.0 * t.powi(2) - 3.0 * t + 1.0)
        + start_handle * (3.0 * t.powi(3) - 6.0 * t.powi(2) + 3.0 * t)
        + end_handle * (-3.0 * t.powi(3) + 3.0 * t.powi(2))
        + end * (t.powi(3))
}

fn bez_dx(t: f32, start: Vec3, start_handle: Vec3, end_handle: Vec3, end: Vec3) -> Vec3 {
    start * (-3.0 * t.powi(2) + 6.0 * t - 3.0)
        + start_handle * (9.0 * t.powi(2) - 12.0 * t + 3.0)
        + end_handle * (-9.0 * t.powi(2) + 6.0 * t)
        + end * (3.0 * t.powi(2))
}

impl App {
    async fn new(ctx: &mut Context) -> Self {
        let device = render::device(ctx);
        let surface_config = render::surface_config(ctx);

        // Camera
        let camera = render::PerspectiveCamera::new(device).pos(vec3(0.0, 2.0, 3.0));

        // Vertex buffer
        let vertex_buffer = render::VertexBufferBuilder::new()
            .source(render::BufferSource::Capacity(STEPS as usize * 2 - 1))
            .usages(wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST)
            .build(ctx);

        // Shader
        let shader = render::ShaderBuilder::new("bezier.wgsl")
            .default_target(surface_config)
            .buffers(vec![vertex_buffer.desc()])
            .bind_group_layouts(vec![camera.bind_group_layout()])
            .build(ctx)
            .await;

        let depth_buffer = render::DepthBuffer::new(ctx);
        // Pipeline
        let pipeline = render::RenderPipelineBuilder::new(&shader)
            .topology(wgpu::PrimitiveTopology::TriangleStrip)
            .depth_buffer(render::DepthBuffer::depth_stencil_state())
            .build(ctx);

        render::window(ctx).set_cursor_visible(false);

        Self {
            vertex_buffer,
            pipeline,
            camera,
            depth_buffer,
        }
    }

    fn camera_movement(&mut self, ctx: &mut Context) {
        let dt = gbase::time::delta_time(ctx);

        // Camera rotation
        // if input::mouse_button_pressed(ctx, input::MouseButton::Left) {}
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
                self.camera.pos += camera_movement_dir.normalize() * dt * 5.0 / 10.0;
            } else {
                self.camera.pos += camera_movement_dir.normalize() * dt * 5.0;
            }
        }
    }
}

impl Callbacks for App {
    fn update(&mut self, ctx: &mut Context) -> bool {
        if input::key_just_pressed(ctx, KeyCode::KeyR) {
            self.camera.yaw = 0.0;
            self.camera.pitch = 0.0;
        }

        self.camera_movement(ctx);

        // Vertices

        let facing = vec3(-1.0, 0.0, -1.0).normalize();
        let orth = vec3(facing.z, 0.0, -facing.x);
        let width = 0.1;

        let t = time::time_since_start(ctx);

        let height = 4.0;
        let start = vec3(0.0, 0.0, 0.0);
        let start_handle = vec3(0.0, 1.0 * height + t.sin(), 0.0);
        let end_handle = vec3(facing.x, 0.8 * height + t.cos(), facing.z);
        let end = vec3(facing.x, 0.75 * height, facing.z);

        let mut vertices = Vec::new();
        for i in 0..STEPS {
            let t = i as f32 / STEPS as f32;
            let inv_t = (1.0 - t) * 1.5;

            let pos = bez(t, start, start_handle, end_handle, end);
            let dx = bez_dx(t, start, start_handle, end_handle, end);
            let normal = orth.cross(dx).normalize();

            if i == STEPS - 1 {
                vertices.push(VertexNormal {
                    position: pos.to_array(),
                    normal: normal.to_array(),
                });
            } else {
                let pos1 = pos + orth * width * inv_t;
                vertices.push(VertexNormal {
                    position: pos1.to_array(),
                    normal: normal.to_array(),
                });
                let pos2 = pos - orth * width * inv_t;
                vertices.push(VertexNormal {
                    position: pos2.to_array(),
                    normal: normal.to_array(),
                });
            }
        }

        self.vertex_buffer.update_buffer(ctx, &vertices);

        false
    }

    fn render(&mut self, ctx: &mut Context, screen_view: &wgpu::TextureView) -> bool {
        let mut encoder = render::create_encoder(ctx, None);
        let queue = render::queue(ctx);
        // update camera uniform
        self.camera.update_buffer(ctx);

        // render
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("render pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: screen_view,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                    store: wgpu::StoreOp::Store,
                },
                resolve_target: None,
            })],
            depth_stencil_attachment: Some(self.depth_buffer.depth_stencil_attachment_clear()),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        render_pass.set_pipeline(self.pipeline.pipeline());
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_bind_group(0, self.camera.bind_group(), &[]);
        render_pass.draw(0..self.vertex_buffer.len(), 0..1);

        drop(render_pass);
        queue.submit(Some(encoder.finish()));

        false
    }
}
