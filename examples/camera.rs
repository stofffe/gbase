use encase::ShaderType;
use gbase::{filesystem, input, render, Callbacks, Context, ContextBuilder, LogLevel};
use glam::{vec3, Vec3};
use std::path::Path;
use winit::keyboard::KeyCode;

#[pollster::main]
pub async fn main() {
    let (mut ctx, ev) = ContextBuilder::new()
        .log_level(LogLevel::Warn)
        .vsync(false)
        .build()
        .await;
    let app = App::new(&mut ctx).await;
    gbase::run(app, ctx, ev);
}

struct App {
    vertex_buffer: render::VertexBuffer<render::Vertex>,
    pipeline: render::ArcRenderPipeline,
    camera: render::PerspectiveCamera,
    camera_bindgroup: render::ArcBindGroup,
    camera_buffer: render::UniformBuffer,
}

impl App {
    async fn new(ctx: &mut Context) -> Self {
        // Shader
        let shader_str = filesystem::load_string(ctx, Path::new("shaders/camera.wgsl"))
            .await
            .unwrap();
        let shader = render::ShaderBuilder::new(shader_str).build_uncached(ctx);

        // Vertex buffer
        let vertex_buffer = render::VertexBufferBuilder::new(TRIANGLE_VERTICES)
            .usage(wgpu::BufferUsages::VERTEX)
            .build(ctx);

        // Camera
        let camera = render::PerspectiveCamera::new();
        let buffer = render::UniformBufferBuilder::new()
            .build(ctx, render::PerspectiveCameraUniform::min_size());
        let bindgroup_layout = render::BindGroupLayoutBuilder::new()
            .entries(vec![
                // Camera
                render::BindGroupLayoutEntry::new().uniform().vertex(),
            ])
            .build_uncached(ctx);
        let bindgroup = render::BindGroupBuilder::new(bindgroup_layout.clone())
            .entries(vec![
                // Camera
                render::BindGroupEntry::Buffer(buffer.buffer()),
            ])
            .build_uncached(ctx);

        // Pipeline
        let pipeline_layoyt = render::PipelineLayoutBuilder::new()
            .bind_groups(vec![bindgroup_layout])
            .build(ctx);
        let pipeline = render::RenderPipelineBuilder::new(shader, pipeline_layoyt)
            .buffers(vec![vertex_buffer.desc()])
            .targets(vec![render::RenderPipelineBuilder::default_target(ctx)])
            .build_uncached(ctx);

        render::window(ctx).set_cursor_visible(false);

        Self {
            vertex_buffer,
            pipeline,
            camera,
            camera_bindgroup: bindgroup,
            camera_buffer: buffer,
        }
    }
}

impl Callbacks for App {
    fn render(&mut self, ctx: &mut Context, screen_view: &wgpu::TextureView) -> bool {
        let mut encoder = render::EncoderBuilder::new().build(ctx);
        let queue = render::queue(ctx);

        self.camera.pos = vec3(0.0, 0.0, 2.0);
        self.camera_buffer.write(ctx, &self.camera.uniform(ctx));

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("render pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: screen_view,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
                resolve_target: None,
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.buf_ref().slice(..));
        render_pass.set_bind_group(0, &self.camera_bindgroup, &[]);
        render_pass.draw(0..TRIANGLE_VERTICES.len() as u32, 0..1);
        drop(render_pass);

        queue.submit(Some(encoder.finish()));

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
}

#[rustfmt::skip]
const TRIANGLE_VERTICES: &[render::Vertex] = &[
    render::Vertex { position: [-0.5, -0.5, 0.0] },
    render::Vertex { position: [ 0.5, -0.5, 0.0] },
    render::Vertex { position: [ 0.0,  0.5, 0.0] },
];
