use std::f32::consts::PI;

use gbase::{
    filesystem,
    glam::{vec3, Vec3},
    input,
    render::{self},
    wgpu,
    winit::keyboard::KeyCode,
    Callbacks, Context,
};

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub async fn run() {
    gbase::run::<App>().await;
}

struct App {
    vertex_buffer: render::VertexBuffer<render::Vertex>,
    pipeline: render::ArcRenderPipeline,
    camera: gbase_utils::Camera,
    camera_bindgroup: render::ArcBindGroup,
    camera_buffer: render::UniformBuffer<gbase_utils::CameraUniform>,
}

impl Callbacks for App {
    #[no_mangle]
    fn new(ctx: &mut Context) -> Self {
        // Shader
        let shader_str = filesystem::load_s!("shaders/camera.wgsl").unwrap();
        let shader = render::ShaderBuilder::new(shader_str).build_uncached(ctx);

        // Vertex buffer
        let vertex_buffer = render::VertexBufferBuilder::new(render::VertexBufferSource::Data(
            TRIANGLE_VERTICES.to_vec(),
        ))
        .build(ctx);

        // Camera
        let camera = gbase_utils::Camera::new(gbase_utils::CameraProjection::perspective(PI / 2.0))
            .pos(vec3(0.0, 0.0, 2.0));

        let buffer =
            render::UniformBufferBuilder::new(render::UniformBufferSource::Empty).build(ctx);
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
            .single_target(render::ColorTargetState::from_current_screen(ctx))
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
    #[no_mangle]
    fn render(&mut self, ctx: &mut Context, screen_view: &wgpu::TextureView) -> bool {
        self.camera_buffer.write(ctx, &self.camera.uniform(ctx));

        render::RenderPassBuilder::new()
            .color_attachments(&[Some(
                render::RenderPassColorAttachment::new(screen_view).clear(wgpu::Color::BLACK),
            )])
            .build_run_submit(ctx, |mut pass| {
                pass.set_pipeline(&self.pipeline);
                pass.set_vertex_buffer(0, self.vertex_buffer.buf_ref().slice(..));
                pass.set_bind_group(0, Some(self.camera_bindgroup.as_ref()), &[]);
                pass.draw(0..TRIANGLE_VERTICES.len() as u32, 0..1);
            });

        false
    }
    #[no_mangle]
    fn update(&mut self, ctx: &mut Context) -> bool {
        let dt = gbase::time::delta_time(ctx);

        if input::key_just_pressed(ctx, KeyCode::KeyR) {
            self.camera.yaw = 0.0;
            self.camera.pitch = 0.0;
        }

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
        if camera_movement_dir != Vec3::ZERO {
            self.camera.pos += camera_movement_dir.normalize() * dt;
        }

        // Camera zoom
        self.camera.zoom(input::scroll_delta(ctx).1 * dt);

        false
    }
}

#[rustfmt::skip]
const TRIANGLE_VERTICES: &[render::Vertex] = &[
    render::Vertex { position: [-0.5, -0.5, 0.0] },
    render::Vertex { position: [ 0.5, -0.5, 0.0] },
    render::Vertex { position: [ 0.0,  0.5, 0.0] },
];
