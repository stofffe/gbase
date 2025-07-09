use gbase::{
    asset::{self, AssetHandle},
    glam::{vec3, vec4, Quat, Vec3},
    input::{self, mouse_button_pressed},
    load_b,
    render::{self, Mesh},
    time, tracing, wgpu, winit, Callbacks, Context,
};
use gbase_utils::{
    Camera, GpuMaterial, PbrLightUniforms, PbrRenderer, PixelCache, SizeKind, Transform3D, Widget,
    RED, WHITE,
};
use gbase_utils::{MeshLod, ShadowPass};
use std::{f32::consts::PI, sync::Arc};

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub async fn run() {
    gbase::run::<App>().await;
}

const PLANE_SIZE: f32 = 1000.0;
const PLANE_COLOR: [f32; 4] = [0.3, 1.0, 0.2, 1.0];

struct App {
    hdr_framebuffer_1: render::FrameBuffer,
    ldr_framebuffer: render::FrameBuffer,
    framebuffer_renderer: gbase_utils::TextureRenderer,

    depth_buffer: render::DepthBuffer,
    pbr_renderer: gbase_utils::PbrRenderer,
    gizmo_renderer: gbase_utils::GizmoRenderer,
    ui_renderer: gbase_utils::GUIRenderer,

    camera: Camera,
    camera_buffer: render::UniformBuffer<gbase_utils::CameraUniform>,
    lights: PbrLightUniforms,
    lights_buffer: render::UniformBuffer<PbrLightUniforms>,

    ak47_mesh_handle: asset::AssetHandle<Mesh>,
    ak47_material: Arc<GpuMaterial>,
    helmet_mesh_handle: asset::AssetHandle<Mesh>,
    helmet_material: Arc<GpuMaterial>,
    helmet_mesh_lod: MeshLod,

    plane_mesh_handle: asset::AssetHandle<Mesh>,
    plane_material: Arc<GpuMaterial>,

    shadow_pass: ShadowPass,
}

fn load_simple_mesh(
    cache: &mut gbase::asset::AssetCache,
    bytes: &[u8],
    pixel_cache: &mut PixelCache,
    pbr_renderer: &PbrRenderer,
) -> (asset::AssetHandle<Mesh>, Arc<GpuMaterial>) {
    let prim = gbase_utils::parse_glb(bytes)[0].clone();
    let mesh = prim
        .mesh
        .extract_attributes(pbr_renderer.required_attributes().clone());
    let mesh_handle = asset::AssetBuilder::insert(mesh).build(cache);
    let material = prim.material.clone().to_material(cache, pixel_cache);
    (mesh_handle, Arc::new(material))
}

fn mesh_to_lod_mesh(mesh: AssetHandle<render::Mesh>, mat: Arc<GpuMaterial>) -> MeshLod {
    MeshLod {
        meshes: vec![(mesh, 0.0)],
        mat,
    }
}

fn load_lod_mesh(
    cache: &mut gbase::asset::AssetCache,
    bytes: &[u8],
    pixel_cache: &mut PixelCache,
    pbr_renderer: &PbrRenderer,
) -> MeshLod {
    let mut meshes = Vec::new();
    // NOTE: debug boxes defined separately
    let thresholds = [0.25, 0.125, 0.0];

    let mut material = None;

    // TODO: scuffed rev
    // need to create LOD by name
    for (i, prim) in gbase_utils::parse_glb(bytes).into_iter().rev().enumerate() {
        let mesh = prim
            .mesh
            .extract_attributes(pbr_renderer.required_attributes().clone());
        let mesh_handle = asset::AssetBuilder::insert(mesh).build(cache);
        material = Some(Arc::new(
            prim.material.clone().to_material(cache, pixel_cache),
        ));

        dbg!(mesh_handle.clone().get(cache).unwrap().vertex_count());
        meshes.push((mesh_handle, thresholds[i]));
    }

    MeshLod {
        meshes,
        mat: material.unwrap(),
    }
}

#[derive(Debug, Clone)]
struct DrawCall {
    mesh: MeshLod,
    transform: Transform3D,
}

impl Callbacks for App {
    #[no_mangle]
    fn init_ctx() -> gbase::ContextBuilder {
        gbase::ContextBuilder::new()
            .log_level(tracing::Level::INFO)
            .vsync(false)
            .device_features(
                wgpu::Features::POLYGON_MODE_LINE
                    | wgpu::Features::TIMESTAMP_QUERY
                    | wgpu::Features::RG11B10UFLOAT_RENDERABLE,
            )
            .window_attributes(winit::window::Window::default_attributes().with_maximized(true))
            .gpu_profiler_enabled(false)
    }

    #[no_mangle]
    fn new(ctx: &mut Context, cache: &mut gbase::asset::AssetCache) -> Self {
        tracing::error!("start of  new");
        let mut pixel_cache = PixelCache::new();

        let hdr_format = if render::device(ctx)
            .features()
            .contains(wgpu::Features::RG11B10UFLOAT_RENDERABLE)
        {
            wgpu::TextureFormat::Rg11b10Ufloat
        } else {
            wgpu::TextureFormat::Rgba16Float
        };
        let hdr_framebuffer_builder = render::FrameBufferBuilder::new()
            .screen_size(ctx)
            .format(hdr_format)
            .usage(wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING);
        let hdr_framebuffer = hdr_framebuffer_builder.clone().label("hdr 1").build(ctx);

        let ldr_framebuffer = render::FrameBufferBuilder::new()
            .screen_size(ctx)
            .format(wgpu::TextureFormat::Rgba8Unorm)
            .usage(
                wgpu::TextureUsages::STORAGE_BINDING
                    | wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::RENDER_ATTACHMENT,
            )
            .label("ldr")
            .build(ctx);
        let framebuffer_renderer = gbase_utils::TextureRenderer::new(ctx, cache);

        let depth_buffer = render::DepthBufferBuilder::new()
            .screen_size(ctx)
            .build(ctx);

        let pbr_renderer = PbrRenderer::new(ctx, cache);

        let (ak47_mesh_handle, ak47_material) = load_simple_mesh(
            cache,
            &load_b!("models/ak47.glb").unwrap(),
            &mut pixel_cache,
            &pbr_renderer,
        );

        let (helmet_mesh_handle, helmet_material) = load_simple_mesh(
            cache,
            &load_b!("models/helmet.glb").unwrap(),
            &mut pixel_cache,
            &pbr_renderer,
        );

        let camera = gbase_utils::Camera::new_with_screen_size(
            ctx,
            gbase_utils::CameraProjection::Perspective { fov: PI / 2.0 },
        )
        .pos(vec3(0.0, 0.0, 8.0));
        let camera_buffer = render::UniformBufferBuilder::new().build(ctx);

        let ui_renderer = gbase_utils::GUIRenderer::new(
            ctx,
            1024,
            &load_b!("fonts/font.ttf").unwrap(),
            gbase_utils::DEFAULT_SUPPORTED_CHARS,
        );
        let gizmo_renderer = gbase_utils::GizmoRenderer::new(ctx);

        let lights = PbrLightUniforms {
            main_light_dir: vec3(1.0, -1.0, 1.0).normalize(),
            main_light_insensity: 1.0,
        };
        let lights_buffer = render::UniformBufferBuilder::new().build(ctx);

        let plane_mesh_handle = asset::AssetBuilder::insert(
            render::MeshBuilder::quad()
                .build()
                .extract_attributes(pbr_renderer.required_attributes().clone()),
        )
        .build(cache);
        let plane_material = Arc::new(
            gbase_utils::PbrMaterial {
                base_color_texture: None,
                color_factor: PLANE_COLOR,
                metallic_roughness_texture: None,
                roughness_factor: 1.0,
                metallic_factor: 0.0,
                occlusion_texture: None,
                occlusion_strength: 1.0,
                normal_texture: None,
                normal_scale: 1.0,
                emissive_texture: None,
                emissive_factor: [0.0, 0.0, 0.0],
            }
            .to_material(cache, &mut pixel_cache), // TODO: remove this?
        );

        let shadow_pass = ShadowPass::new(ctx, cache);

        let helmet_mesh_lod = load_lod_mesh(
            cache,
            &load_b!("models/helmet_lod.glb").unwrap(),
            &mut pixel_cache,
            &pbr_renderer,
        );

        dbg!(&helmet_mesh_lod);

        Self {
            hdr_framebuffer_1: hdr_framebuffer,
            ldr_framebuffer,
            depth_buffer,
            framebuffer_renderer,

            pbr_renderer,
            ui_renderer,
            gizmo_renderer,
            lights,
            lights_buffer,

            camera,
            camera_buffer,

            ak47_mesh_handle,
            ak47_material,

            helmet_mesh_handle,
            helmet_material,

            plane_mesh_handle,
            plane_material,

            shadow_pass,
            helmet_mesh_lod,
        }
    }

    #[no_mangle]
    fn render(
        &mut self,
        ctx: &mut Context,
        cache: &mut gbase::asset::AssetCache,
        screen_view: &gbase::wgpu::TextureView,
    ) -> bool {
        if mouse_button_pressed(ctx, input::MouseButton::Left) {
            self.camera.flying_controls(ctx);
        }

        if gbase::input::key_just_pressed(ctx, gbase::input::KeyCode::KeyR) {
            tracing::warn!("RESTART");
            *self = Self::new(ctx, cache);
        }

        let _render_timer = time::ProfileTimer::new(ctx, "render");

        // update buffers
        self.camera_buffer.write(ctx, &self.camera.uniform());
        self.lights_buffer.write(ctx, &self.lights);

        // clear textures
        // TODO: clear in some other pass
        self.hdr_framebuffer_1.clear(ctx, wgpu::Color::BLACK);
        self.depth_buffer.clear(ctx);

        let mut draw_calls = vec![
            DrawCall {
                mesh: mesh_to_lod_mesh(self.plane_mesh_handle.clone(), self.plane_material.clone()),
                transform: Transform3D::default()
                    .with_pos(vec3(0.0, -2.0, 0.0))
                    .with_rot(Quat::from_rotation_x(-PI / 2.0))
                    .with_scale(Vec3::ONE * PLANE_SIZE),
            },
            // add meshes
            DrawCall {
                mesh: self.helmet_mesh_lod.clone(),
                transform: Transform3D::default()
                    .with_rot(Quat::from_rotation_y(time::time_since_start(ctx)))
                    .with_pos(vec3(0.0, 0.0, 0.0))
                    .with_scale(Vec3::ONE * 1.0),
            },
            DrawCall {
                mesh: self.helmet_mesh_lod.clone(),
                transform: Transform3D::default()
                    .with_pos(vec3(-3.0, 10.0, 0.0))
                    .with_scale(Vec3::ONE * 1.0),
            },
            DrawCall {
                mesh: mesh_to_lod_mesh(self.ak47_mesh_handle.clone(), self.ak47_material.clone()),
                transform: Transform3D::default()
                    .with_pos(vec3(3.0, 0.0, -1.0))
                    .with_scale(Vec3::ONE * 1.0),
            },
        ];

        let amount = 10;
        let gap = 20;
        for x in -amount..amount {
            for z in -amount..amount {
                draw_calls.push(DrawCall {
                    mesh: self.helmet_mesh_lod.clone(), // TODO: should this even be clone
                    transform: Transform3D::default()
                        .with_pos(vec3(gap as f32 * x as f32, 10.0, gap as f32 * z as f32))
                        .with_scale(Vec3::ONE * 1.0),
                });
            }
        }

        // shadow pass
        // TODO: scuffed
        let shadow_meshes = draw_calls
            .clone()
            .into_iter()
            .map(|draw_call| (draw_call.mesh, draw_call.transform))
            .collect::<Vec<_>>();

        self.lights.main_light_dir = vec3(1.0, -1.0, 0.0);

        self.shadow_pass.render(
            ctx,
            cache,
            &shadow_meshes,
            &self.camera,
            // TODO: doesnt work for (0,-1,0)
            self.lights.main_light_dir.normalize(),
        );

        // pbr pass
        for draw_call in draw_calls.iter() {
            self.pbr_renderer.add_mesh_lod(
                ctx,
                cache,
                &draw_call.mesh,
                draw_call.transform.clone(),
                &self.camera,
            );
        }

        self.pbr_renderer.add_mesh_lod(
            ctx,
            cache,
            &self.helmet_mesh_lod,
            Transform3D {
                pos: vec3(10.0, 0.0, 10.0),
                rot: Quat::IDENTITY,
                scale: Vec3::ONE,
            },
            &self.camera,
        );

        // self.pbr_renderer
        //     .render_bounding_boxes(ctx, cache, &mut self.gizmo_renderer, &self.camera);
        self.pbr_renderer.render(
            ctx,
            cache,
            self.hdr_framebuffer_1.view_ref(),
            self.hdr_framebuffer_1.format(),
            &self.camera_buffer,
            &self.lights_buffer,
            &self.depth_buffer,
            &self.camera.calculate_frustum(),
            &self.shadow_pass.shadow_map,
            &self.shadow_pass.light_matrices_buffer,
            &self.shadow_pass.light_matrices_distances,
        );

        self.gizmo_renderer.render(
            ctx,
            self.hdr_framebuffer_1.view_ref(),
            self.hdr_framebuffer_1.format(),
            &self.camera_buffer,
        );

        self.framebuffer_renderer.render(
            ctx,
            cache,
            self.hdr_framebuffer_1.view(),
            screen_view,
            render::surface_format(ctx),
        );

        let view = render::TextureViewBuilder::new(self.shadow_pass.shadow_map.clone())
            .base_array_layer(0)
            .dimension(wgpu::TextureViewDimension::D2)
            .build(ctx);
        self.framebuffer_renderer.render_depth(
            ctx,
            cache,
            view,
            screen_view,
            render::surface_format(ctx),
            &self.camera_buffer,
        );

        _render_timer.finish();

        Widget::new()
            .width(SizeKind::PercentOfParent(1.0))
            .height(SizeKind::PercentOfParent(1.0))
            .layout(&mut self.ui_renderer, |renderer| {
                Widget::new()
                    .height(SizeKind::PercentOfParent(0.5))
                    .render(renderer);

                for (label, time) in time::profiler(ctx).get_cpu_samples() {
                    Widget::new()
                        .width(SizeKind::TextSize)
                        .height(SizeKind::TextSize)
                        .text(format!("CPU: {:.5} {}", time * 1000.0, label))
                        .text_color(WHITE)
                        .render(renderer);
                }
            });

        self.ui_renderer.display_debug_info(ctx);
        self.ui_renderer
            .render(ctx, screen_view, render::surface_format(ctx));

        false
    }

    #[no_mangle]
    fn resize(
        &mut self,
        ctx: &mut Context,
        _cache: &mut gbase::asset::AssetCache,
        new_size: gbase::winit::dpi::PhysicalSize<u32>,
    ) {
        self.depth_buffer.resize(ctx, new_size);
        self.ui_renderer.resize(ctx, new_size);
        self.gizmo_renderer.resize(ctx, new_size);
        self.hdr_framebuffer_1.resize(ctx, new_size);
        self.ldr_framebuffer.resize(ctx, new_size);
        self.camera.resize(new_size);
    }
}

impl App {
    #[no_mangle]
    fn hot_reload(&mut self, _ctx: &mut Context) {
        Self::init_ctx().init_logging();
        render::set_vsync(_ctx, false);
    }
}
