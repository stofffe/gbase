use gbase::{
    filesystem,
    glam::{vec3, Quat},
    input, load_b, log,
    render::{self, GpuImage, GpuMesh, Image, Mesh, ShaderBuilder},
    time, wgpu, Callbacks, Context,
};
use gbase_utils::{
    AssetCache, AssetHandle, GlbLoader, GpuMaterial, PbrLightUniforms, PbrRenderer, Transform3D,
};
use std::{f32::consts::PI, sync::Arc};

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub async fn run() {
    gbase::run::<App>().await;
}

struct App {
    depth_buffer: render::DepthBuffer,
    pbr_renderer: gbase_utils::PbrRenderer,
    gizmo_renderer: gbase_utils::GizmoRenderer,
    ui_renderer: gbase_utils::GUIRenderer,

    camera: gbase_utils::Camera,
    camera_buffer: render::UniformBuffer<gbase_utils::CameraUniform>,
    lights_buffer: render::UniformBuffer<PbrLightUniforms>,

    ak47_mesh_handle: AssetHandle<Mesh>,
    ak47_material: Arc<GpuMaterial>,

    // cube_model: GpuModel,
    // penguin_model: GpuModel,
    // helmet_model: GpuModel,
    cube_mesh_handle: gbase_utils::AssetHandle<Mesh>,
    cube_material: Arc<GpuMaterial>,

    image_cache: AssetCache<Image, GpuImage>,
    mesh_cache: AssetCache<Mesh, GpuMesh>,
    shader_cache: AssetCache<ShaderBuilder, wgpu::ShaderModule>,
}

impl Callbacks for App {
    #[no_mangle]
    fn init_ctx() -> gbase::ContextBuilder {
        gbase::ContextBuilder::new()
            .log_level(gbase::LogLevel::Info)
            .vsync(true)
            .device_features(wgpu::Features::POLYGON_MODE_LINE)
    }

    #[no_mangle]
    fn new(ctx: &mut Context) -> Self {
        let mut image_cache = AssetCache::new();
        let mut mesh_cache = AssetCache::new();
        let shader_cache = AssetCache::new();

        let depth_buffer = render::DepthBufferBuilder::new()
            .screen_size(ctx)
            .build(ctx);

        let pbr_renderer = PbrRenderer::new(ctx);

        let mut glb_loader = GlbLoader::new();

        let ak47_prim = glb_loader.parse_glb(
            ctx,
            &mut image_cache,
            &filesystem::load_b!("models/ak47.glb").unwrap(),
        )[0]
        .clone();
        let ak47_mesh = ak47_prim
            .mesh
            .extract_attributes(pbr_renderer.required_attributes().clone());
        let ak47_mesh_handle = mesh_cache.allocate(ak47_mesh);
        let ak47_material = ak47_prim
            .material
            .clone()
            .to_material(&mut image_cache)
            .into();

        // let cube_prim = glb_loader.parse_glb(
        //     ctx,
        //     &mut image_cache,
        //     &filesystem::load_b!("models/cube.glb").unwrap(),
        // );
        // let mut cube_model = GpuModel { meshes: Vec::new() };
        // for prim in cube_prim {
        //     let mesh_with_attr = &prim
        //         .mesh
        //         .extract_attributes(pbr_renderer.required_attributes());
        //
        //     let mesh_handle = mesh_cache.allocate(mesh_with_attr.clone());
        //     cube_model.meshes.push((
        //         mesh_handle,
        //         Arc::new(prim.material.to_material(ctx, &mut image_cache)),
        //         Transform3D::from_matrix(prim.transform),
        //     ));
        // }

        // let penguin_prim = gbase_utils::parse_glb(
        //     ctx,
        //     &mut assets,
        //     &filesystem::load_b!("models/penguin.glb").unwrap(),
        // );
        // let mut penguin_model = GpuModel { meshes: Vec::new() };
        // for prim in penguin_prim {
        //     let mesh_with_attr = &prim
        //         .mesh
        //         .extract_attributes(pbr_renderer.required_attributes());
        //     let penguin_gpu_mesh = gbase_utils::GpuMesh::new(ctx, mesh_with_attr);
        //     let penguin_material = prim.material.to_material(ctx, &mut assets);
        //     let penguin_local_transform = Transform3D::from_matrix(prim.transform);
        //
        //     penguin_model.meshes.push((
        //         Arc::new(penguin_gpu_mesh),
        //         Arc::new(penguin_material),
        //         penguin_local_transform,
        //     ));
        // }
        //
        // let helmet_prim = gbase_utils::parse_glb(
        //     ctx,
        //     &mut assets,
        //     &filesystem::load_b!("models/helmet.glb").unwrap(),
        // );
        // let mut helmet_model = GpuModel { meshes: Vec::new() };
        // for prim in helmet_prim {
        //     let mesh_with_attr = &prim
        //         .mesh
        //         .extract_attributes(pbr_renderer.required_attributes());
        //     let helmet_gpu_mesh = gbase_utils::GpuMesh::new(ctx, mesh_with_attr);
        //     let helmet_material = prim.material.to_material(ctx, &mut assets);
        //     let helmet_local_transform = Transform3D::from_matrix(prim.transform);
        //
        //     helmet_model.meshes.push((
        //         Arc::new(helmet_gpu_mesh),
        //         Arc::new(helmet_material),
        //         helmet_local_transform,
        //     ));
        // }

        let camera =
            gbase_utils::Camera::new(gbase_utils::CameraProjection::Perspective { fov: PI / 2.0 })
                .pos(vec3(0.0, 0.0, 3.0));
        let camera_buffer = render::UniformBufferBuilder::new(render::UniformBufferSource::Data(
            camera.uniform(ctx),
        ))
        .build(ctx);

        let ui_renderer = gbase_utils::GUIRenderer::new(
            ctx,
            1024,
            &load_b!("fonts/font.ttf").unwrap(),
            gbase_utils::DEFAULT_SUPPORTED_CHARS,
        );
        let gizmo_renderer = gbase_utils::GizmoRenderer::new(ctx);

        let lights_buffer = render::UniformBufferBuilder::new(render::UniformBufferSource::Data(
            PbrLightUniforms {
                main_light_dir: vec3(0.0, -1.0, 1.0).normalize(),
            },
        ))
        .build(ctx);

        let cube_prim = glb_loader.parse_glb(
            ctx,
            &mut image_cache,
            &filesystem::load_b!("models/cube.glb").unwrap(),
        )[0]
        .clone();
        let cube_mesh = cube_prim
            .mesh
            .extract_attributes(pbr_renderer.required_attributes().clone());
        let cube_material = cube_prim.material.to_material(&mut image_cache).into();
        let cube_mesh_handle = mesh_cache.allocate(cube_mesh);

        Self {
            pbr_renderer,
            ui_renderer,
            gizmo_renderer,
            lights_buffer,

            camera,
            camera_buffer,

            depth_buffer,

            image_cache,
            mesh_cache,
            shader_cache,

            ak47_mesh_handle,
            ak47_material,

            cube_mesh_handle,
            cube_material,
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
        self.mesh_cache.check_watch(ctx);
        self.image_cache.check_watch(ctx);
        self.shader_cache.check_watch(ctx);

        self.depth_buffer.clear(ctx);

        self.camera_buffer.write(ctx, &self.camera.uniform(ctx));

        let elems = 20u32;
        for x in 0..(elems.isqrt()) {
            for z in 0..(elems.isqrt()) {
                let transform = Transform3D::from_pos(vec3(15.0 * x as f32, 0.0, 10.0 * z as f32))
                    .with_rot(Quat::from_rotation_y(
                        (time::time_since_start(ctx) + (x + z) as f32) * 1.0,
                    ));

                self.pbr_renderer.add_mesh(
                    self.ak47_mesh_handle.clone(),
                    self.ak47_material.clone(),
                    transform,
                );

                // if (x + z) % 2 == 0 {
                //     self.pbr_renderer.add_mesh(
                //         self.ak47_gpu_mesh.clone(),
                //         self.ak47_gpu_material.clone(),
                //         transform,
                //     );
                // } else {
                //     self.pbr_renderer.add_model(&self.helmet_model, transform);
                // }
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

        // if input::key_just_pressed(ctx, input::KeyCode::F1) {
        //     let cube_mesh = self.mesh_cache.get_mut(self.cube_mesh_handle.clone());
        //     // let cube_mesh = self.assets.get_mesh_mut(self.cube_mesh_handle.clone());
        //     let pos_verts = cube_mesh
        //         .attributes
        //         .get_mut(&render::VertexAttributeId::Position)
        //         .unwrap();
        //
        //     let shifted = pos_verts
        //         .as_type::<[f32; 3]>()
        //         .iter()
        //         .map(|&[x, y, z]| [x * 1.1, y * 1.1, z * 1.1])
        //         .collect::<Vec<_>>();
        //
        //     // cube_mesh.add_attribute(
        //     //     render::VertexAttributeId::Position,
        //     //     render::VertexAttributeValues::Float32x3(shifted),
        //     // );
        // }

        self.pbr_renderer.add_mesh(
            self.cube_mesh_handle.clone(),
            self.cube_material.clone(),
            Transform3D::default(),
        );

        self.pbr_renderer.render_bounding_boxes(
            ctx,
            &mut self.gizmo_renderer,
            &mut self.mesh_cache,
        );
        self.pbr_renderer.render(
            ctx,
            screen_view,
            render::surface_format(ctx),
            &mut self.mesh_cache,
            &mut self.image_cache,
            &self.camera,
            &self.camera_buffer,
            &self.lights_buffer,
            &self.depth_buffer,
        );
        self.gizmo_renderer.render(
            ctx,
            screen_view,
            render::surface_format(ctx),
            &self.camera_buffer,
        );
        self.ui_renderer
            .render(ctx, screen_view, render::surface_format(ctx));

        false
    }

    #[no_mangle]
    fn resize(&mut self, ctx: &mut Context, new_size: gbase::winit::dpi::PhysicalSize<u32>) {
        self.depth_buffer.resize(ctx, new_size);
        self.ui_renderer.resize(ctx, new_size);
        self.gizmo_renderer.resize(ctx, new_size);
    }
}

impl App {
    #[no_mangle]
    fn hot_reload(&mut self, _ctx: &mut Context) {
        Self::init_ctx().init_logging();
    }
}
