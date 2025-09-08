use gbase::{
    glam::{vec3, Quat, Vec3},
    render, time, tracing, wgpu, CallbackResult, Callbacks, Context,
};
use gbase_utils::Transform3D;
use std::f32::consts::PI;

use rapier3d::prelude::*;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub async fn run() {
    gbase::run::<App>().await;
}

struct App {
    camera: gbase_utils::Camera,
    camera_buffer: render::UniformBuffer<gbase_utils::CameraUniform>,
    gizmo_renderer: gbase_utils::GizmoRenderer,

    // physics
    ball_body_handle: RigidBodyHandle,
    ball_body_handle_2: RigidBodyHandle,
    ball_collider_handle: ColliderHandle,
    floor_collider_handle: ColliderHandle,

    rigid_body_set: RigidBodySet,
    collider_set: ColliderSet,

    integration_parameters: IntegrationParameters,
    physics_pipeline: PhysicsPipeline,
    island_manager: IslandManager,
    broad_phase: DefaultBroadPhase,
    narrow_phase: NarrowPhase,
    impulse_joint_set: ImpulseJointSet,
    multibody_joint_set: MultibodyJointSet,
    ccd_solver: CCDSolver,
    physics_hooks: (),
    event_handler: (),
}

impl Callbacks for App {
    #[no_mangle]
    fn init_ctx() -> gbase::ContextBuilder {
        gbase::ContextBuilder::new()
            .log_level(tracing::Level::INFO)
            .device_features(wgpu::Features::TIMESTAMP_QUERY)
    }

    #[no_mangle]
    fn new(ctx: &mut Context, _cache: &mut gbase::asset::AssetCache) -> Self {
        let camera = gbase_utils::Camera::new_with_screen_size(
            ctx,
            gbase_utils::CameraProjection::perspective(PI / 2.0),
        )
        .pos(vec3(0.0, 2.0, 5.0));

        let camera_buffer = render::UniformBufferBuilder::new().build(ctx);
        let gizmo_renderer = gbase_utils::GizmoRenderer::new(ctx);

        let mut rigid_body_set = RigidBodySet::new();
        let mut collider_set = ColliderSet::new();

        let floor = ColliderBuilder::cuboid(2.0, 0.1, 2.0);
        let floor_collider_handle = collider_set.insert(floor);

        let ball_body = RigidBodyBuilder::dynamic().translation(vector![0.0, 10.0, 0.0]);
        let ball_collider = ColliderBuilder::ball(0.5).restitution(0.7).build();

        let ball_body_handle = rigid_body_set.insert(ball_body.clone());
        let ball_body_handle_2 =
            rigid_body_set.insert(ball_body.translation(vector![0.5, 15.0, 0.1]).clone());
        let ball_collider_handle = collider_set.insert_with_parent(
            ball_collider.clone(),
            ball_body_handle,
            &mut rigid_body_set,
        );
        let _ball_collider_handle_2 =
            collider_set.insert_with_parent(ball_collider, ball_body_handle_2, &mut rigid_body_set);

        let integration_parameters = IntegrationParameters {
            dt: time::FIXED_UPDATE_TIME,
            ..Default::default()
        };
        let physics_pipeline = PhysicsPipeline::new();
        let island_manager = IslandManager::new();
        let broad_phase = DefaultBroadPhase::new();
        let narrow_phase = NarrowPhase::new();
        let impulse_joint_set = ImpulseJointSet::new();
        let multibody_joint_set = MultibodyJointSet::new();
        let ccd_solver = CCDSolver::new();
        let physics_hooks = ();
        let event_handler = ();

        Self {
            camera,
            camera_buffer,
            gizmo_renderer,

            rigid_body_set,
            collider_set,

            integration_parameters,
            physics_pipeline,
            island_manager,
            broad_phase,
            narrow_phase,
            impulse_joint_set,
            multibody_joint_set,
            ccd_solver,
            physics_hooks,
            event_handler,

            ball_body_handle,
            ball_body_handle_2,
            ball_collider_handle,
            floor_collider_handle,
        }
    }

    #[no_mangle]
    fn fixed_update(
        &mut self,
        _ctx: &mut Context,
        _cache: &mut gbase::asset::AssetCache,
    ) -> CallbackResult {
        let gravity = vector![0.0, -9.81, 0.0];
        self.physics_pipeline.step(
            &gravity,
            &self.integration_parameters,
            &mut self.island_manager,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut self.rigid_body_set,
            &mut self.collider_set,
            &mut self.impulse_joint_set,
            &mut self.multibody_joint_set,
            &mut self.ccd_solver,
            &self.physics_hooks,
            &self.event_handler,
        );

        CallbackResult::Continue
    }

    #[no_mangle]
    fn render(
        &mut self,
        ctx: &mut Context,
        _cache: &mut gbase::asset::AssetCache,
        screen_view: &wgpu::TextureView,
    ) -> CallbackResult {
        self.camera.flying_controls(ctx);

        self.camera_buffer.write(ctx, &self.camera.uniform());

        let floor = &self.collider_set[self.floor_collider_handle];
        let floor_transform = floor.position();

        let floor_pos = floor_transform.translation;
        let aabb = floor.compute_aabb();

        // Compute size from mins/maxs
        let size = vec3(
            aabb.maxs.x - aabb.mins.x,
            aabb.maxs.y - aabb.mins.y,
            aabb.maxs.z - aabb.mins.z,
        );
        self.gizmo_renderer.draw_cube(
            &Transform3D::new(
                vec3(floor_pos.x, floor_pos.y, floor_pos.z),
                Quat::IDENTITY,
                size,
            ),
            vec3(1.0, 0.0, 0.0),
        );

        let ball_body = &self.rigid_body_set[self.ball_body_handle];
        let ball_pos = ball_body.position().translation;

        let ball_collider = &self.collider_set[self.ball_collider_handle];
        let radius = ball_collider.shape().as_ball().unwrap().radius;

        self.gizmo_renderer.draw_sphere(
            &Transform3D::new(
                vec3(ball_pos.x, ball_pos.y, ball_pos.z),
                Quat::IDENTITY,
                Vec3::splat(radius * 2.0),
            ),
            vec3(1.0, 1.0, 1.0),
        );

        let ball_body = &self.rigid_body_set[self.ball_body_handle_2];
        let ball_pos = ball_body.position().translation;
        self.gizmo_renderer.draw_sphere(
            &Transform3D::new(
                vec3(ball_pos.x, ball_pos.y, ball_pos.z),
                Quat::IDENTITY,
                Vec3::splat(radius * 2.0),
            ),
            vec3(1.0, 1.0, 1.0),
        );

        self.gizmo_renderer.render(
            ctx,
            screen_view,
            render::surface_format(ctx),
            &self.camera_buffer,
        );

        CallbackResult::Continue
    }
}

#[no_mangle]
fn hot_reload() {
    App::init_ctx().init_logging();
}
