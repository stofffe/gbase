use gbase::{
    filesystem,
    glam::{vec3, Quat},
    load_b, log, render, time, Callbacks, Context,
};
use gbase_utils::Transform3D;
use std::f32::consts::PI;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub async fn run() {
    gbase::run::<App>().await;
}

struct App {
    mesh_renderer: gbase_utils::PbrRenderer,
    depth_buffer: render::DepthBuffer,

    camera: gbase_utils::Camera,

    camera_buffer: render::UniformBuffer<gbase_utils::CameraUniform>,
    ui: gbase_utils::GUIRenderer,

    ak47_mesh: gbase_utils::Mesh,
    ak47_material: gbase_utils::GpuMaterial,
    ak47_gpu_mesh: gbase_utils::GpuMesh,

    cube_mesh: gbase_utils::Mesh,
    cube_material: gbase_utils::GpuMaterial,
    cube_gpu_mesh: gbase_utils::GpuMesh,
}

impl Callbacks for App {
    #[no_mangle]
    fn init_ctx() -> gbase::ContextBuilder {
        gbase::ContextBuilder::new()
            .log_level(gbase::LogLevel::Info)
            .vsync(false)
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
            .require_exact_attributes(mesh_renderer.required_attributes());
        let ak47_material = ak47_prim.material.to_material(ctx);
        let ak47_gpu_mesh = gbase_utils::GpuMesh::new(ctx, &ak47_mesh);

        let cube_prim =
            gbase_utils::parse_glb(ctx, &filesystem::load_b!("models/cube.glb").unwrap())[0]
                .clone();
        let cube_mesh = cube_prim
            .mesh
            .require_exact_attributes(mesh_renderer.required_attributes());
        let cube_material = cube_prim.material.to_material(ctx);
        let cube_gpu_mesh = gbase_utils::GpuMesh::new(ctx, &cube_mesh);

        let camera =
            gbase_utils::Camera::new(gbase_utils::CameraProjection::Perspective { fov: PI / 2.0 })
                .pos(vec3(0.0, 0.0, 1.0));
        let camera_buffer = render::UniformBufferBuilder::new(render::UniformBufferSource::Data(
            camera.uniform(ctx),
        ))
        .build(ctx);

        let ui = gbase_utils::GUIRenderer::new(
            ctx,
            render::surface_format(ctx),
            1024,
            &load_b!("fonts/font.ttf").unwrap(),
            gbase_utils::DEFAULT_SUPPORTED_CHARS,
        );

        Self {
            mesh_renderer,
            camera,
            camera_buffer,

            depth_buffer,
            ui,

            ak47_mesh,
            ak47_material,
            ak47_gpu_mesh,
            cube_mesh,
            cube_material,
            cube_gpu_mesh,
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
        let mut draw_calls = Vec::new();
        for x in 0..(elems.isqrt()) {
            for z in 0..(elems.isqrt()) {
                let transform = Transform3D::from_pos(vec3(10.0 * x as f32, 0.0, 10.0 * z as f32))
                    .with_rot(Quat::from_rotation_x(
                        time::time_since_start(ctx) + (x + z) as f32,
                    ));
                if (x + z) % 2 == 0 {
                    draw_calls.push((&self.ak47_gpu_mesh, &self.ak47_material, transform));
                } else {
                    draw_calls.push((&self.cube_gpu_mesh, &self.cube_material, transform));
                }
            }
        }
        self.mesh_renderer.render(
            ctx,
            screen_view,
            &self.camera_buffer,
            &self.depth_buffer,
            &draw_calls,
        );

        self.ui.render(ctx, screen_view);

        false
    }

    #[no_mangle]
    fn resize(&mut self, ctx: &mut Context, new_size: gbase::winit::dpi::PhysicalSize<u32>) {
        self.depth_buffer
            .resize(ctx, new_size.width, new_size.height);
        self.ui.resize(ctx, new_size);
    }
}

impl App {
    #[no_mangle]
    fn hot_reload(&mut self, _ctx: &mut Context) {
        Self::init_ctx().init_logging();
    }
}
