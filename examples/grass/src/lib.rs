mod grass_renderer;

use gbase::glam;
use gbase::log;
use gbase::wgpu;
use gbase::winit;
use gbase::winit::window::Window;
use gbase::{filesystem, input, render, time, Callbacks, Context};
use gbase_utils::sobel_filter;
use gbase_utils::Transform3D;
use glam::{vec2, vec3, vec4, Quat, Vec3, Vec4};
use grass_renderer::GrassRenderer;
use std::f32::consts::PI;
use winit::dpi::PhysicalSize;
use winit::{keyboard::KeyCode, window::CursorGrabMode};

#[cfg_attr(target_arch = "wasm32", wasm_bindgen::prelude::wasm_bindgen)]
pub async fn run() {
    gbase::run::<App>().await;
}

const CAMERA_MOVE_SPEED: f32 = 15.0;

const PLANE_SIZE: f32 = 500.0;
const PLANE_COLOR: [f32; 4] = [0.0, 0.4, 0.0, 1.0];

pub struct App {
    camera: gbase_utils::Camera,
    camera_buffer: render::UniformBuffer<gbase_utils::CameraUniform>,
    light: Vec3,
    light_buffer: render::UniformBuffer<Vec3>,
    deferred_buffers: gbase_utils::DeferredBuffers,

    // mesh_renderer: gbase_utils::MeshRenderer,
    deferred_renderer: gbase_utils::DeferredRenderer,
    grass_renderer: GrassRenderer,
    gui_renderer: gbase_utils::GUIRenderer,
    gizmo_renderer: gbase_utils::GizmoRenderer,

    paused: bool,

    // plane: gbase_utils::GpuDrawCall,
    // plane_transform: gbase_utils::Transform3D,
    // plane_transform_buffer: render::UniformBuffer<gbase_utils::TransformUniform>,
    framebuffer: render::FrameBuffer,
    framebuffer_renderer: gbase_utils::TextureRenderer,
    sobel_filter: sobel_filter::SobelFilter,
}

impl Callbacks for App {
    #[no_mangle]
    fn init_ctx() -> gbase::ContextBuilder {
        gbase::ContextBuilder::new()
            .log_level(gbase::LogLevel::Info)
            .window_attributes(Window::default_attributes().with_maximized(true))
            .vsync(false)
    }
    #[no_mangle]
    fn new(ctx: &mut Context) -> Self {
        // Framebuffer
        let framebuffer = render::FrameBufferBuilder::new()
            .screen_size(ctx)
            .format(wgpu::TextureFormat::Rgba8Unorm)
            .usage(
                wgpu::TextureUsages::RENDER_ATTACHMENT
                    | wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::COPY_SRC
                    | wgpu::TextureUsages::STORAGE_BINDING,
            )
            .build(ctx);
        let framebuffer_renderer =
            gbase_utils::TextureRenderer::new(ctx, render::surface_format(ctx));

        // Camera
        let camera = gbase_utils::Camera::new(gbase_utils::CameraProjection::perspective(PI / 2.0))
            .pos(vec3(-1.0, 8.0, -1.0))
            .yaw(PI / 4.0);

        let camera_buffer = render::UniformBufferBuilder::new(render::UniformBufferSource::Empty)
            .label("camera buf".to_string())
            .usage(wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST)
            .build(ctx);
        let light = vec3(10.0, 10.0, -10.0);
        let light_buffer =
            render::UniformBufferBuilder::new(render::UniformBufferSource::Empty).build(ctx);

        // Renderers
        let deferred_buffers = gbase_utils::DeferredBuffers::new(ctx);
        // let mesh_renderer = gbase_utils::MeshRenderer::new(ctx, &deferred_buffers);
        let deferred_renderer = gbase_utils::DeferredRenderer::new(
            ctx,
            framebuffer.format(),
            &deferred_buffers,
            &camera_buffer,
            &light_buffer,
        );
        let grass_renderer = GrassRenderer::new(ctx, &deferred_buffers, &camera_buffer);
        let gui_renderer = gbase_utils::GUIRenderer::new(
            ctx,
            1024,
            &filesystem::load_b!("fonts/meslo.ttf").unwrap(),
            gbase_utils::DEFAULT_SUPPORTED_CHARS,
        );
        let gizmo_renderer = gbase_utils::GizmoRenderer::new(ctx);

        // // Plane mesh
        // let plane_transform = gbase_utils::Transform3D::new(
        //     // vec3(-10.0, 8.0, -10.0),
        //     vec3(0.0, 0.0, 0.0),
        //     Quat::from_rotation_x(-PI / 2.0),
        //     vec3(PLANE_SIZE, PLANE_SIZE, 1.0),
        // );
        // let plane_transform_buffer =
        //     render::UniformBufferBuilder::new(render::UniformBufferSource::Empty).build(ctx);
        // let gpu_mesh = gbase_utils::GpuMesh::from_mesh(
        //     ctx,
        //     gbase_utils::Mesh::new(
        //         CENTERED_QUAD_VERTICES.to_vec(),
        //         CENTERED_QUAD_INDICES.to_vec(),
        //     ),
        // );
        // let gpu_material = gbase_utils::GpuMaterial::from_material(
        //     ctx,
        //     gbase_utils::Material {
        //         color_factor: PLANE_COLOR,
        //         roughness_factor: 0.0,
        //         metalness_factor: 0.0,
        //         occlusion_strength: 1.0,
        //         albedo: None,
        //         normal: None,
        //         roughness: None,
        //     },
        // );
        // let plane = gbase_utils::GpuDrawCall::new(
        //     ctx,
        //     gpu_mesh,
        //     gpu_material,
        //     &plane_transform_buffer,
        //     &camera_buffer,
        //     &mesh_renderer,
        // );

        let sobel_filter = sobel_filter::SobelFilter::new(ctx);

        Self {
            camera,
            camera_buffer,
            light,
            light_buffer,
            deferred_buffers,
            // mesh_renderer,
            deferred_renderer,
            grass_renderer,
            gui_renderer,
            gizmo_renderer,

            paused: false,

            // plane,
            // plane_transform,
            // plane_transform_buffer,
            framebuffer,
            framebuffer_renderer,
            sobel_filter,
        }
    }

    #[no_mangle]
    fn render(&mut self, ctx: &mut Context, screen_view: &wgpu::TextureView) -> bool {
        // log::warn!("RENDER");
        self.deferred_buffers.clear(ctx);

        // update buffers
        self.camera_buffer.write(ctx, &self.camera.uniform(ctx));
        let t = time::time_since_start(ctx);
        self.light = vec3(t.cos(), 1.0, t.sin()) * 10.0;
        self.light_buffer.write(ctx, &self.light);
        // self.plane_transform_buffer
        //     .write(ctx, &self.plane_transform.uniform());

        // Render
        // let pt = time::ProfileTimer::new("GRASS");
        self.grass_renderer
            .render(ctx, &self.camera, &self.deferred_buffers);
        // pt.log();

        //Mesh
        // self.mesh_renderer
        //     .render(ctx, &self.deferred_buffers, &[&self.plane]);
        self.deferred_renderer
            .render(ctx, self.framebuffer.view_ref());
        self.gui_renderer
            .render(ctx, self.framebuffer.view_ref(), self.framebuffer.format());
        self.gizmo_renderer.draw_sphere(
            &Transform3D::new(self.light, Quat::IDENTITY, Vec3::ONE),
            vec3(1.0, 0.0, 0.0),
        );
        self.gizmo_renderer.render(
            ctx,
            self.framebuffer.view_ref(),
            self.framebuffer.format(),
            &self.camera_buffer,
        );

        if input::key_pressed(ctx, KeyCode::KeyP) {
            self.sobel_filter.apply_filter(
                ctx,
                &self.framebuffer,
                &sobel_filter::SobelFilterParams::new(1),
            );
        }

        self.framebuffer_renderer
            .render(ctx, self.framebuffer.view(), screen_view);

        false
    }

    #[no_mangle]
    fn resize(&mut self, ctx: &mut Context, new_size: PhysicalSize<u32>) {
        self.gizmo_renderer.resize(ctx, new_size);
        self.deferred_buffers.resize(ctx, new_size);
        self.framebuffer.resize(ctx, new_size);
        self.deferred_renderer.rebuild_bindgroup(
            ctx,
            &self.deferred_buffers,
            &self.camera_buffer,
            &self.light_buffer,
        );
    }

    #[no_mangle]
    fn update(&mut self, ctx: &mut Context) -> bool {
        // hot reload
        #[cfg(debug_assertions)]
        if input::key_just_pressed(ctx, KeyCode::KeyR) {
            self.grass_renderer =
                GrassRenderer::new(ctx, &self.deferred_buffers, &self.camera_buffer);
            self.deferred_renderer = gbase_utils::DeferredRenderer::new(
                ctx,
                self.framebuffer.format(),
                &self.deferred_buffers,
                &self.camera_buffer,
                &self.light_buffer,
            );
            // self.mesh_renderer = gbase_utils::MeshRenderer::new(ctx, &self.deferred_buffers);
            println!("reload");
        }

        // pausing
        if input::key_just_pressed(ctx, KeyCode::Escape) {
            self.paused = !self.paused;

            // #[cfg(not(target_arch = "wasm32"))]
            // {
            //     render::window(ctx)
            //         .set_cursor_grab(if self.paused {
            //             CursorGrabMode::None
            //         } else {
            //             CursorGrabMode::Locked
            //         })
            //         .expect("could not set grab mode");
            //     render::window(ctx).set_cursor_visible(self.paused);
            // }
        }
        if self.paused {
            self.gui_renderer.text(
                "pause (esc)",
                vec2(0.0, 0.0),
                vec2(0.5, 0.5),
                0.05,
                vec4(1.0, 1.0, 1.0, 1.0),
                false,
            );
            return false;
        }

        // self.plane_transform.pos.x = self.camera.pos.x;
        // self.plane_transform.pos.z = self.camera.pos.z;

        self.camera.flying_controls(ctx);

        // debug camera pos
        if input::key_pressed(ctx, KeyCode::KeyC) {
            log::info!("{}", self.camera.pos);
        }

        // debug text
        if input::key_pressed(ctx, KeyCode::KeyF) {
            let avg_ms = time::frame_time(ctx) * 1000.0;
            let avg_fps = 1.0 / time::frame_time(ctx);
            let strings = [format!("fps {avg_fps:.5}"), format!("ms  {avg_ms:.5}")];

            const DEBUG_HEIGH: f32 = 0.05;
            const DEBUG_COLOR: Vec4 = vec4(1.0, 1.0, 1.0, 1.0);
            for (i, text) in strings.iter().enumerate() {
                self.gui_renderer.text(
                    text,
                    vec2(0.0, DEBUG_HEIGH * i as f32),
                    vec2(0.5, 0.5),
                    DEBUG_HEIGH,
                    DEBUG_COLOR,
                    false,
                );
            }
        }

        false
    }
}

#[rustfmt::skip]
const CENTERED_QUAD_VERTICES: &[render::VertexFull] = &[
    render::VertexFull { position: [-0.5, -0.5, 0.0], color: PLANE_COLOR, uv: [0.0, 1.0], normal: [0.0, 0.0, 1.0], tangent: [1.0, 0.0, 0.0, 1.0] }, // bottom left
    render::VertexFull { position: [ 0.5, -0.5, 0.0], color: PLANE_COLOR, uv: [1.0, 1.0], normal: [0.0, 0.0, 1.0], tangent: [1.0, 0.0, 0.0, 1.0] }, // bottom right
    render::VertexFull { position: [ 0.5,  0.5, 0.0], color: PLANE_COLOR, uv: [1.0, 0.0], normal: [0.0, 0.0, 1.0], tangent: [1.0, 0.0, 0.0, 1.0] }, // top right

    render::VertexFull { position: [-0.5, -0.5, 0.0], color: PLANE_COLOR, uv: [0.0, 1.0], normal: [0.0, 0.0, 1.0], tangent: [1.0, 0.0, 0.0, 1.0] }, // bottom left
    render::VertexFull { position: [ 0.5,  0.5, 0.0], color: PLANE_COLOR, uv: [1.0, 0.0], normal: [0.0, 0.0, 1.0], tangent: [1.0, 0.0, 0.0, 1.0] }, // top right
    render::VertexFull { position: [-0.5,  0.5, 0.0], color: PLANE_COLOR, uv: [0.0, 0.0], normal: [0.0, 0.0, 1.0], tangent: [1.0, 0.0, 0.0, 1.0] }, // top left

];

#[rustfmt::skip]
const CENTERED_QUAD_INDICES: &[u32] = &[
    0, 1, 2,
    3, 4, 5
];
