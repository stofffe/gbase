use gbase::{
    filesystem, input,
    render::{self, CameraUniform},
    Callbacks, Context, ContextBuilder,
};
use glam::{vec3, Quat, Vec3};

fn main() {
    gbase::run_app_with_builder::<App>(
        ContextBuilder::new()
            .log_level(gbase::LogLevel::Warn)
            .vsync(true),
    );
}

struct App {
    mesh_renderer: render::MeshRenderer,
    deferred_buffers: render::DeferredBuffers,
    deferred_renderer: render::DeferredRenderer,
    camera: render::PerspectiveCamera,
    camera_buffer: render::UniformBuffer<CameraUniform>,
    light: Vec3,
    light_buffer: render::UniformBuffer<Vec3>,
    debug_input: render::DebugInput,
    model1: render::GpuGltfModel,
    model2: render::GpuGltfModel,
    gizmo_renderer: render::GizmoRenderer,

    framebuffer: render::FrameBuffer,
    framebuffer_renderer: render::TextureRenderer,
    sobel_filter: render::SobelFilter,
}

impl Callbacks for App {
    fn new(ctx: &mut Context) -> Self {
        let deferred_buffers = render::DeferredBuffers::new(ctx);
        let mut camera = render::PerspectiveCamera::new();
        camera.pos = vec3(0.5, 0.0, 1.0);
        let camera_buffer =
            render::UniformBufferBuilder::new(render::UniformBufferSource::Empty).build(ctx);
        let light = Vec3::ZERO;
        let light_buffer =
            render::UniformBufferBuilder::new(render::UniformBufferSource::Data(light)).build(ctx);
        let deferred_renderer = render::DeferredRenderer::new(
            ctx,
            wgpu::TextureFormat::Rgba8Unorm,
            &deferred_buffers,
            &camera_buffer,
            &light_buffer,
        );
        let debug_input = render::DebugInput::new(ctx);
        let gizmo_renderer =
            render::GizmoRenderer::new(ctx, wgpu::TextureFormat::Rgba8Unorm, &camera_buffer);

        let mesh_renderer = render::MeshRenderer::new(ctx, &deferred_buffers);

        let model1_bytes = filesystem::load_b!("models/ak47.glb").unwrap();
        let model1 = render::GltfModel::from_glb_bytes(&model1_bytes);
        let model1 = render::GpuGltfModel::from_model(ctx, model1, &camera_buffer, &mesh_renderer);

        let model2_bytes = filesystem::load_b!("models/coord2.glb").unwrap();
        let model2 = render::GltfModel::from_glb_bytes(&model2_bytes);
        let model2 = render::GpuGltfModel::from_model(ctx, model2, &camera_buffer, &mesh_renderer);

        let framebuffer = render::FrameBufferBuilder::new()
            .usage(
                wgpu::TextureUsages::STORAGE_BINDING
                    | wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::COPY_SRC,
            )
            .screen_size(ctx)
            .build(ctx);
        let framebuffer_renderer =
            render::TextureRenderer::new(ctx, wgpu::TextureFormat::Bgra8UnormSrgb);
        let sobel_filter = render::SobelFilter::new(ctx);

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
            model1,
            model2,

            framebuffer,
            framebuffer_renderer,
            sobel_filter,
        }
    }
    // fn new(&mut self, _ctx: &mut Context) {
    //     self.camera.pos = vec3(0.5, 0.0, 1.0);
    // }
    fn update(&mut self, ctx: &mut Context) -> bool {
        let dt = gbase::time::delta_time(ctx);

        if input::key_just_pressed(ctx, input::KeyCode::KeyR) {
            // self.camera.yaw = 0.0;
            // self.camera.pitch = 0.0;
            self.mesh_renderer = render::MeshRenderer::new(ctx, &self.deferred_buffers);
            self.deferred_renderer = render::DeferredRenderer::new(
                ctx,
                wgpu::TextureFormat::Rgba8Unorm,
                &self.deferred_buffers,
                &self.camera_buffer,
                &self.light_buffer,
            );

            let model1_bytes = filesystem::load_b!("models/ak47.glb").unwrap();
            let model1 = render::GltfModel::from_glb_bytes(&model1_bytes);
            self.model1 = render::GpuGltfModel::from_model(
                ctx,
                model1,
                &self.camera_buffer,
                &self.mesh_renderer,
            );

            let model2_bytes = filesystem::load_b!("models/coord2.glb").unwrap();
            let model2 = render::GltfModel::from_glb_bytes(&model2_bytes);
            self.model2 = render::GpuGltfModel::from_model(
                ctx,
                model2,
                &self.camera_buffer,
                &self.mesh_renderer,
            );
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
        // eprintln!("FPS {}", time::fps(ctx));
        // let t = gbase::time::time_since_start(ctx);
        self.light = vec3(5.0, 1.5, 5.0); // self.light = vec3(t.sin() * 5.0, 0.0, t.cos() * 5.0);
        self.light_buffer.write(ctx, &self.light);
        self.camera_buffer.write(ctx, &self.camera.uniform(ctx));
        self.debug_input.update_buffer(ctx);

        // Render into gbuffer
        self.deferred_buffers.clear(ctx);
        let meshes = &[&self.model1, &self.model2];
        self.mesh_renderer
            .render_models(ctx, &self.deferred_buffers, meshes);
        self.deferred_renderer
            .render(ctx, self.framebuffer.view_ref());
        self.gizmo_renderer.draw_sphere(
            0.1,
            &render::Transform::new(self.light, Quat::IDENTITY, Vec3::ONE),
            vec3(1.0, 0.0, 0.0),
        );
        self.gizmo_renderer.render(ctx, self.framebuffer.view_ref());

        if input::key_pressed(ctx, input::KeyCode::KeyP) {
            self.sobel_filter.apply_filter(
                ctx,
                &self.framebuffer,
                &render::SobelFilterParams::new(1),
            );
        }

        self.framebuffer_renderer
            .render(ctx, self.framebuffer.view(), screen_view);

        false
    }

    fn resize(&mut self, ctx: &mut Context) {
        self.gizmo_renderer.resize_screen(ctx);
        self.framebuffer.resize_screen(ctx);
        self.deferred_buffers.resize_screen(ctx);
        self.deferred_renderer.rebuild_bindgroup(
            ctx,
            &self.deferred_buffers,
            &self.camera_buffer,
            &self.light_buffer,
        );
    }
}
