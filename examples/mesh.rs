use encase::ShaderType;
use gbase::{
    filesystem, input,
    render::{self, Transform},
    Callbacks, Context,
};
use glam::{vec3, Quat, Vec3};

#[pollster::main]
async fn main() {
    let (ctx, ev) = gbase::ContextBuilder::new().build().await;
    let app = App::new(&ctx).await;
    gbase::run(app, ctx, ev);
}

struct App {
    mesh_renderer: MeshRenderer,
    deferred_buffers: render::DeferredBuffers,
    deferred_renderer: render::DeferredRenderer,
    camera: render::PerspectiveCamera,
    camera_buffer: render::UniformBuffer,
    light: Vec3,
    light_buffer: render::UniformBuffer,
    gizmo_renderer: render::GizmoRenderer,
    debug_input: render::DebugInput,
    model1: render::GpuModel,
    model1_transform: Transform,
    model1_transform_uni: render::UniformBuffer,

    model2: render::GpuModel,
    model2_transform: Transform,
    model2_transform_uni: render::UniformBuffer,
}

impl App {
    async fn new(ctx: &Context) -> Self {
        let deferred_buffers = render::DeferredBuffers::new(ctx);
        let camera = render::PerspectiveCamera::new();
        let camera_buffer = render::UniformBufferBuilder::new()
            .build(ctx, render::PerspectiveCameraUniform::min_size());
        let light = Vec3::ZERO;
        let light_buffer = render::UniformBufferBuilder::new().build_init(ctx, &light);
        let deferred_renderer =
            render::DeferredRenderer::new(ctx, &deferred_buffers, &camera_buffer, &light_buffer)
                .await;
        let debug_input = render::DebugInput::new(ctx);
        let gizmo_renderer = render::GizmoRenderer::new(ctx);

        // Model 1
        let model1_transform = render::Transform::new(Vec3::ZERO, Quat::IDENTITY, Vec3::splat(2.0));
        let model1_transform_uni =
            render::UniformBufferBuilder::new().build(ctx, render::TransformUniform::min_size());
        let model1_bytes = filesystem::load_bytes(ctx, "armor.glb").await.unwrap();
        let model1 = render::load_glb(ctx, &model1_bytes);
        let model1 =
            render::GpuModel::from_model(ctx, model1, &camera_buffer, &model1_transform_uni);

        // Model 2
        let model2_transform = render::Transform::new(Vec3::ZERO, Quat::IDENTITY, Vec3::splat(2.0));
        let model2_transform_uni =
            render::UniformBufferBuilder::new().build(ctx, render::TransformUniform::min_size());
        let model2_bytes = filesystem::load_bytes(ctx, "ak47.glb").await.unwrap();
        let model2 = render::load_glb(ctx, &model2_bytes);
        let model2 =
            render::GpuModel::from_model(ctx, model2, &camera_buffer, &model2_transform_uni);

        let mesh_renderer = MeshRenderer::new(ctx, &deferred_buffers, &model1).await;
        Self {
            mesh_renderer,
            deferred_buffers,
            deferred_renderer,
            camera,
            camera_buffer,
            light,
            light_buffer,
            gizmo_renderer,
            debug_input,

            model1_transform,
            model1_transform_uni,
            model1,

            model2_transform,
            model2_transform_uni,
            model2,
        }
    }
}

impl Callbacks for App {
    fn init(&mut self, _ctx: &mut Context) {
        self.camera.pos = vec3(0.5, 0.0, 1.0);
    }
    fn update(&mut self, ctx: &mut Context) -> bool {
        let dt = gbase::time::delta_time(ctx);

        if input::key_just_pressed(ctx, input::KeyCode::KeyR) {
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
        if input::key_pressed(ctx, input::KeyCode::KeyW) {
            camera_movement_dir += self.camera.forward();
        }

        if input::key_pressed(ctx, input::KeyCode::KeyS) {
            camera_movement_dir -= self.camera.forward();
        }
        if input::key_pressed(ctx, input::KeyCode::KeyA) {
            camera_movement_dir -= self.camera.right();
        }
        if input::key_pressed(ctx, input::KeyCode::KeyD) {
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

    fn render(&mut self, ctx: &mut Context, screen_view: &wgpu::TextureView) -> bool {
        let t = gbase::time::time_since_start(ctx);
        self.light = vec3(5.0, 1.5, 5.0);
        // self.light = vec3(t.sin() * 5.0, 0.0, t.cos() * 5.0);
        self.light_buffer.write(ctx, &self.light);
        self.camera_buffer.write(ctx, &self.camera.uniform(ctx));

        self.model2_transform = Transform::new(
            vec3(0.0, 0.5, 0.0),
            Quat::from_rotation_y(t / 2.0),
            Vec3::ONE,
        );
        self.model2_transform_uni
            .write(ctx, &self.model2_transform.uniform());

        self.model1_transform = Transform::new(Vec3::ZERO, Quat::IDENTITY, Vec3::ONE);
        self.model1_transform_uni
            .write(ctx, &self.model1_transform.uniform());
        let queue = render::queue(ctx);

        let mut encoder = render::EncoderBuilder::new().build(ctx);
        // Render albedo
        self.debug_input.update_buffer(ctx);
        self.mesh_renderer.render(
            ctx,
            &mut encoder,
            &self.deferred_buffers,
            &[&self.model1, &self.model2],
        );

        self.deferred_renderer
            .render(ctx, screen_view, &mut encoder);
        queue.submit(Some(encoder.finish()));

        self.gizmo_renderer.draw_sphere(
            0.1,
            &render::Transform::new(self.light, Quat::IDENTITY, Vec3::ONE),
            vec3(1.0, 0.0, 0.0),
        );
        self.gizmo_renderer
            .render(ctx, screen_view, &mut self.camera);
        false
    }

    fn resize(&mut self, ctx: &mut Context) {
        self.gizmo_renderer.resize(ctx);

        self.deferred_buffers.resize(ctx);
        self.deferred_renderer.resize(
            ctx,
            &self.deferred_buffers,
            &self.camera_buffer,
            &self.light_buffer,
        );
    }
}

pub struct MeshRenderer {
    pipeline: wgpu::RenderPipeline,
}

impl MeshRenderer {
    pub async fn new(
        ctx: &Context,
        deferred_buffers: &render::DeferredBuffers,
        model: &render::GpuModel,
    ) -> Self {
        eprintln!("LEN {}", model.primitives.len());
        let bindgroup_layout = &model.primitives[0].bindgroup_layout;
        let shader_str = filesystem::load_string(ctx, "mesh.wgsl").await.unwrap();
        let shader = render::ShaderBuilder::new(&shader_str).build(ctx);
        let pipeline = render::RenderPipelineBuilder::new(&shader)
            .buffers(&[model.primitives[0].mesh.vertex_buffer.desc()])
            .targets(&deferred_buffers.targets())
            .bind_groups(&[&bindgroup_layout])
            .depth_stencil(deferred_buffers.depth_stencil_state())
            .cull_mode(wgpu::Face::Back)
            .build(ctx);

        Self { pipeline }
    }

    fn render(
        &mut self,
        ctx: &gbase::Context,
        encoder: &mut wgpu::CommandEncoder,
        deferred_buffers: &render::DeferredBuffers,
        models: &[&render::GpuModel],
    ) {
        let color_attachments = deferred_buffers.color_attachments();
        let mut mesh_pass = render::RenderPassBuilder::new()
            .color_attachments(&color_attachments)
            .depth_stencil_attachment(deferred_buffers.depth_stencil_attachment_clear())
            .build(encoder);

        mesh_pass.set_pipeline(&self.pipeline);

        for model in models {
            for prim in model.primitives.iter() {
                let mesh = &prim.mesh;
                mesh_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                mesh_pass.set_index_buffer(mesh.index_buffer.slice(..), mesh.index_buffer.format());
                mesh_pass.set_bind_group(0, &prim.bindgroup, &[]);
                mesh_pass.draw_indexed(0..mesh.index_buffer.len(), 0, 0..1);
            }
        }
    }
}
