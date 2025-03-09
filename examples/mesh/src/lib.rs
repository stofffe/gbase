use std::{f32::consts::PI, marker::PhantomData};

use gbase::{
    filesystem,
    glam::{vec3, Quat},
    log,
    render::{self, Vertex, VertexBuffer, VertexTrait},
    time,
    wgpu::{self},
    Callbacks, Context,
};

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub async fn run() {
    gbase::run::<App>().await;
}

struct App {
    mesh_renderer: gbase_utils::MeshRenderer<render::VertexFull>,

    mesh: gbase_utils::Mesh<render::VertexFull>,
    transform: gbase_utils::Transform3D,
    transform_buffer: render::UniformBuffer<gbase_utils::TransformUniform>,
    albedo: render::TextureWithView,
    albedo_sampler: render::ArcSampler,

    camera: gbase_utils::Camera,
    camera_buffer: render::UniformBuffer<gbase_utils::CameraUniform>,
}

impl Callbacks for App {
    #[no_mangle]
    fn init_ctx() -> gbase::ContextBuilder {
        gbase::ContextBuilder::new().log_level(gbase::LogLevel::Warn)
    }
    #[no_mangle]
    fn new(ctx: &mut Context) -> Self {
        let mesh = gbase_utils::MeshBuilder::new().cube().build(ctx);
        let transform = gbase_utils::Transform3D::default();
        let transform_buffer =
            render::UniformBufferBuilder::new(render::UniformBufferSource::Empty).build(ctx);
        let albedo = gbase_utils::texture_builder_from_image_bytes(
            &filesystem::load_b!("textures/texture.jpeg").unwrap(),
        )
        .unwrap()
        .build(ctx)
        .with_default_view(ctx);
        let albedo_sampler = render::SamplerBuilder::new().build(ctx);

        let mesh_renderer = gbase_utils::MeshRenderer::new(ctx);

        let camera =
            gbase_utils::Camera::new(gbase_utils::CameraProjection::Perspective { fov: PI / 2.0 })
                .pos(vec3(0.0, 0.0, 1.0));
        let camera_buffer = render::UniformBufferBuilder::new(render::UniformBufferSource::Data(
            camera.uniform(ctx),
        ))
        .build(ctx);

        Self {
            mesh,
            transform,
            transform_buffer,
            albedo,
            albedo_sampler,

            mesh_renderer,
            camera,
            camera_buffer,
        }
    }

    #[no_mangle]
    fn update(&mut self, ctx: &mut Context) -> bool {
        let t = time::time_since_start(ctx);

        self.camera.flying_controls(ctx);
        self.transform = gbase_utils::Transform3D::default()
            .with_rot(Quat::from_rotation_y(t) * Quat::from_rotation_x(t / 2.0));

        if gbase::input::key_just_pressed(ctx, gbase::input::KeyCode::KeyR) {
            log::warn!("RESTART");
            *self = Self::new(ctx);
        }

        false
    }
    #[no_mangle]
    fn render(&mut self, ctx: &mut Context, screen_view: &gbase::wgpu::TextureView) -> bool {
        self.camera_buffer.write(ctx, &self.camera.uniform(ctx));
        self.transform_buffer.write(ctx, &self.transform.uniform());

        self.mesh_renderer.render(
            ctx,
            screen_view,
            &self.camera_buffer,
            &self.mesh,
            &self.transform_buffer,
            &self.albedo,
            &self.albedo_sampler,
        );

        false
    }
}

impl App {
    #[no_mangle]
    fn hot_reload(&mut self, _ctx: &mut Context) {
        Self::init_ctx().init_logging();
    }
}
