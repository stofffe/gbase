use gbase::{
    filesystem,
    glam::{vec3, Quat, Vec3, Vec4Swizzles},
    load_b, log, render, time, wgpu, Callbacks, Context,
};
use gbase_utils::{
    GpuMaterial, GpuMesh, GpuModel, PbrLightUniforms, PbrMaterial, Transform3D, RED,
};
use std::{f32::consts::PI, sync::Arc};

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub async fn run() {
    gbase::run::<App>().await;
}

struct App {
    depth_buffer: render::DepthBuffer,
    mesh_renderer: gbase_utils::PbrRenderer,
    gizmo_renderer: gbase_utils::GizmoRenderer,
    ui_renderer: gbase_utils::GUIRenderer,

    camera: gbase_utils::Camera,
    camera_buffer: render::UniformBuffer<gbase_utils::CameraUniform>,

    ak47_mesh: gbase_utils::Mesh,
    ak47_material: Arc<gbase_utils::GpuMaterial>,
    ak47_gpu_mesh: Arc<gbase_utils::GpuMesh>,

    cube_model: GpuModel,

    penguin_model: GpuModel,

    helmet_model: GpuModel,

    lights_buffer: render::UniformBuffer<PbrLightUniforms>,
}

impl Callbacks for App {
    #[no_mangle]
    fn init_ctx() -> gbase::ContextBuilder {
        gbase::ContextBuilder::new()
            .log_level(gbase::LogLevel::Info)
            .vsync(true)
        // .device_features(wgpu::Features::POLYGON_MODE_LINE)
    }
    #[no_mangle]
    fn new(ctx: &mut Context) -> Self {
        let depth_buffer = render::DepthBufferBuilder::new()
            .screen_size(ctx)
            .build(ctx);
        let mesh_renderer = gbase_utils::PbrRenderer::new(ctx, &depth_buffer);

        let ak47_prim =
            gbase_utils::parse_glb(ctx, &filesystem::load_b!("models/ak47.glb").unwrap())[0]
                .clone();
        let ak47_mesh = ak47_prim
            .mesh
            .extract_attributes(mesh_renderer.required_attributes());
        let ak47_material = ak47_prim.material.to_material(ctx);
        let ak47_gpu_mesh = gbase_utils::GpuMesh::new(ctx, &ak47_mesh);

        let cube_prim =
            gbase_utils::parse_glb(ctx, &filesystem::load_b!("models/cube.glb").unwrap());
        let mut cube_model = GpuModel { meshes: Vec::new() };
        for prim in cube_prim {
            let mesh_with_attr = &prim
                .mesh
                .extract_attributes(mesh_renderer.required_attributes());

            cube_model.meshes.push((
                Arc::new(gbase_utils::GpuMesh::new(ctx, mesh_with_attr)),
                Arc::new(prim.material.to_material(ctx)),
                Transform3D::from_matrix(prim.transform),
            ));
        }

        let penguin_prim =
            gbase_utils::parse_glb(ctx, &filesystem::load_b!("models/penguin.glb").unwrap());
        let mut penguin_model = GpuModel { meshes: Vec::new() };
        for prim in penguin_prim {
            let mesh_with_attr = &prim
                .mesh
                .extract_attributes(mesh_renderer.required_attributes());
            let penguin_gpu_mesh = gbase_utils::GpuMesh::new(ctx, mesh_with_attr);
            let penguin_material = prim.material.to_material(ctx);
            let penguin_local_transform = Transform3D::from_matrix(prim.transform);

            penguin_model.meshes.push((
                Arc::new(penguin_gpu_mesh),
                Arc::new(penguin_material),
                penguin_local_transform,
            ));
        }

        let helmet_prim =
            gbase_utils::parse_glb(ctx, &filesystem::load_b!("models/helmet.glb").unwrap());
        let mut helmet_model = GpuModel { meshes: Vec::new() };
        for prim in helmet_prim {
            let mesh_with_attr = &prim
                .mesh
                .extract_attributes(mesh_renderer.required_attributes());
            let helmet_gpu_mesh = gbase_utils::GpuMesh::new(ctx, mesh_with_attr);
            let helmet_material = prim.material.to_material(ctx);
            let helmet_local_transform = Transform3D::from_matrix(prim.transform);

            helmet_model.meshes.push((
                Arc::new(helmet_gpu_mesh),
                Arc::new(helmet_material),
                helmet_local_transform,
            ));
        }

        let camera =
            gbase_utils::Camera::new(gbase_utils::CameraProjection::Perspective { fov: PI / 2.0 })
                .pos(vec3(0.0, 0.0, 3.0));
        let camera_buffer = render::UniformBufferBuilder::new(render::UniformBufferSource::Data(
            camera.uniform(ctx),
        ))
        .build(ctx);

        let ui_renderer = gbase_utils::GUIRenderer::new(
            ctx,
            render::surface_format(ctx),
            1024,
            &load_b!("fonts/font.ttf").unwrap(),
            gbase_utils::DEFAULT_SUPPORTED_CHARS,
        );
        let gizmo_renderer =
            gbase_utils::GizmoRenderer::new(ctx, render::surface_format(ctx), &camera_buffer);

        // lock and hide cursor
        // render::window(ctx)
        //     .set_cursor_grab(gbase::winit::window::CursorGrabMode::Locked)
        //     .unwrap();
        // render::window(ctx).set_cursor_visible(false);

        let lights_buffer = render::UniformBufferBuilder::new(render::UniformBufferSource::Data(
            PbrLightUniforms {
                main_light_dir: vec3(0.0, -1.0, 1.0).normalize(),
            },
        ))
        .build(ctx);

        Self {
            mesh_renderer,
            ui_renderer,
            gizmo_renderer,

            camera,
            camera_buffer,

            depth_buffer,

            ak47_mesh,
            ak47_material: Arc::new(ak47_material),
            ak47_gpu_mesh: Arc::new(ak47_gpu_mesh),
            cube_model,
            penguin_model,
            helmet_model,
            lights_buffer,
        }
    }

    #[no_mangle]
    fn update(&mut self, ctx: &mut Context) -> bool {
        self.camera.flying_controls(ctx);

        if gbase::input::key_just_pressed(ctx, gbase::input::KeyCode::KeyR) {
            log::warn!("RESTART");
            *self = Self::new(ctx);
        }

        false
    }

    #[no_mangle]
    fn render(&mut self, ctx: &mut Context, screen_view: &gbase::wgpu::TextureView) -> bool {
        self.depth_buffer.clear(ctx);

        self.camera_buffer.write(ctx, &self.camera.uniform(ctx));

        let elems = 500u32;
        for x in 0..(elems.isqrt()) {
            for z in 0..(elems.isqrt()) {
                let transform = Transform3D::from_pos(vec3(10.0 * x as f32, 0.0, 10.0 * z as f32))
                    .with_rot(Quat::from_rotation_y(
                        (time::time_since_start(ctx) + (x + z) as f32) * 1.0,
                    ));

                if (x + z) % 2 == 0 {
                    let ak47_bouding = self.ak47_mesh.calculate_bounding_box();
                    let max_radius = f32::max(
                        -ak47_bouding.min.min_element(),
                        ak47_bouding.max.max_element(),
                    );
                    log::info!("RAD {:?}", max_radius);
                    self.gizmo_renderer.draw_sphere(
                        &Transform3D::default()
                            .with_pos(transform.pos)
                            .with_rot(transform.rot)
                            .with_scale(Vec3::ONE * max_radius * 2.0),
                        RED.xyz(),
                    );
                    self.mesh_renderer.add_mesh(
                        self.ak47_gpu_mesh.clone(),
                        self.ak47_material.clone(),
                        transform,
                    );
                } else {
                    self.mesh_renderer.add_model(&self.helmet_model, transform);
                }
            }
        }

        // let plane_mesh = gbase_utils::MeshBuilder::quad().build();
        // self.mesh_renderer.add_mesh(
        //     GpuMesh::new(ctx, &plane_mesh).into(),
        //     PbrMaterial::new_colored([1.0, 0.0, 0.0, 1.0])
        //         .to_material(ctx)
        //         .into(),
        //     Transform3D::default().with_rot(Quat::from_rotation_x(-PI / 2.0)),
        // );
        //
        //

        self.mesh_renderer.render(
            ctx,
            screen_view,
            &self.camera,
            &self.camera_buffer,
            &self.lights_buffer,
            &self.depth_buffer,
        );
        self.gizmo_renderer.render(ctx, screen_view);
        self.ui_renderer.render(ctx, screen_view);

        false
    }

    #[no_mangle]
    fn resize(&mut self, ctx: &mut Context, new_size: gbase::winit::dpi::PhysicalSize<u32>) {
        self.depth_buffer
            .resize(ctx, new_size.width, new_size.height);
        self.ui_renderer.resize(ctx, new_size);
        self.gizmo_renderer
            .resize(ctx, new_size.width, new_size.height);
    }
}

impl App {
    #[no_mangle]
    fn hot_reload(&mut self, _ctx: &mut Context) {
        Self::init_ctx().init_logging();
    }
}
