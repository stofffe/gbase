use encase::ShaderType;
use gbase::{
    filesystem, input,
    render::{
        self, ArcBindGroup, ArcComputePipeline, ArcRenderPipeline, DeferredRenderer, GpuGltfModel,
        MeshRenderer, Transform,
    },
    time, Callbacks, Context, ContextBuilder, LogLevel,
};
use glam::{vec2, vec3, vec4, Quat, Vec2, Vec3, Vec3Swizzles};
use std::{f32::consts::PI, mem::size_of, ops::Div};
use winit::{
    keyboard::KeyCode,
    window::{CursorGrabMode, WindowBuilder},
};

#[pollster::main]
pub async fn main() {
    let (mut ctx, ev) = ContextBuilder::new()
        .window_builder(WindowBuilder::new().with_maximized(true))
        .log_level(LogLevel::Info)
        .vsync(false)
        .build()
        .await;
    let app = App::new(&mut ctx).await;
    gbase::run(app, ctx, ev);
}

const TILE_SIZE: u32 = 150;
const BLADES_PER_SIDE: u32 = 16 * 30; // must be > 16 due to dispatch(B/16, B/16, 1) workgroups(16,16,1)
const BLADES_PER_TILE: u32 = BLADES_PER_SIDE * BLADES_PER_SIDE;
const CAMERA_MOVE_SPEED: f32 = 15.0;

const PLANE_SIZE: f32 = 500.0;
const PLANE_COLOR: [f32; 4] = [0.025, 0.1, 0.005, 1.0];

struct App {
    camera: render::PerspectiveCamera,
    camera_buffer: render::UniformBuffer,
    light: Vec3,
    light_buffer: render::UniformBuffer,
    deferred_buffers: render::DeferredBuffers,

    model: render::GpuGltfModel,

    mesh_renderer: render::MeshRenderer,
    deferred_renderer: render::DeferredRenderer,
    grass_renderer: GrassRenderer,
    gui_renderer: render::GUIRenderer,
    gizmo_renderer: render::GizmoRenderer,

    paused: bool,

    plane: render::GpuDrawCall,
    plane_transform: render::Transform,
    plane_transform_buffer: render::UniformBuffer,
}

impl App {
    async fn new(ctx: &mut Context) -> Self {
        // Camera
        let camera = render::PerspectiveCamera::new();
        let camera_buffer = render::UniformBufferBuilder::new()
            .label("camera buf".to_string())
            .usage(wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST)
            .build(ctx, render::PerspectiveCameraUniform::min_size());
        let light = vec3(10.0, 10.0, -10.0);
        let light_buffer = render::UniformBufferBuilder::new().build(ctx, Vec3::min_size());

        // Renderers
        let deferred_buffers = render::DeferredBuffers::new(ctx);
        let mesh_renderer = render::MeshRenderer::new(ctx, &deferred_buffers).await;
        let deferred_renderer = render::DeferredRenderer::new(
            ctx,
            wgpu::TextureFormat::Bgra8UnormSrgb,
            &deferred_buffers,
            &camera_buffer,
            &light_buffer,
        )
        .await;
        let grass_renderer = GrassRenderer::new(ctx, &deferred_buffers, &camera_buffer).await;
        let gui_renderer = render::GUIRenderer::new(
            ctx,
            1000 * 4,
            1000 * 6,
            &filesystem::load_bytes(ctx, "fonts/font.ttf").await.unwrap(),
            render::DEFAULT_SUPPORTED_CHARS,
        )
        .await;
        let gizmo_renderer =
            render::GizmoRenderer::new(ctx, wgpu::TextureFormat::Bgra8UnormSrgb, &camera_buffer)
                .await;

        // Plane mesh
        let plane_transform = render::Transform::new(
            vec3(0.0, 0.0, 0.0),
            Quat::from_rotation_x(-PI / 2.0),
            vec3(PLANE_SIZE, PLANE_SIZE, 1.0),
        );
        let plane_transform_buffer =
            render::UniformBufferBuilder::new().build_init(ctx, &plane_transform.uniform());
        let mesh = render::Mesh::new(
            CENTERED_QUAD_VERTICES.to_vec(),
            CENTERED_QUAD_INDICES.to_vec(),
        );
        let material = render::Material {
            color_factor: PLANE_COLOR,
            roughness_factor: 0.5,
            metalness_factor: 0.0,
            occlusion_strength: 0.5,
            albedo: None,
            normal: None,
            roughness: None,
        };
        let gpu_mesh = render::GpuMesh::from_mesh(ctx, mesh);
        let gpu_material = render::GpuMaterial::from_material(ctx, material);
        let plane = render::GpuDrawCall::new(
            ctx,
            gpu_mesh,
            gpu_material,
            &plane_transform_buffer,
            &camera_buffer,
            &mesh_renderer,
        );

        // Model
        let model_bytes = filesystem::load_bytes(ctx, "models/ak47.glb")
            .await
            .unwrap();
        let model = render::GltfModel::from_glb_bytes(&model_bytes);
        let model = GpuGltfModel::from_model(ctx, model, &camera_buffer, &mesh_renderer);

        Self {
            camera,
            camera_buffer,
            light,
            light_buffer,
            deferred_buffers,
            mesh_renderer,
            deferred_renderer,
            grass_renderer,
            gui_renderer,
            gizmo_renderer,

            model,

            paused: false,

            plane,
            plane_transform,
            plane_transform_buffer,
        }
    }
}

impl Callbacks for App {
    fn render(&mut self, ctx: &mut Context, screen_view: &wgpu::TextureView) -> bool {
        let queue = render::queue(ctx);

        // Clear background and depth buffer
        let mut encoder = render::EncoderBuilder::new().build(ctx);
        let clear_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: screen_view,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
                resolve_target: None,
            })],
            depth_stencil_attachment: Some(self.deferred_buffers.depth_stencil_attachment_clear()),
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        drop(clear_pass);
        queue.submit(Some(encoder.finish()));
        self.deferred_buffers.clear(ctx);

        // update buffers
        self.camera_buffer.write(ctx, &self.camera.uniform(ctx));
        let t = time::time_since_start(ctx);
        self.light = vec3(t.cos(), 1.0, t.sin()) * 10.0;
        self.light_buffer.write(ctx, &self.light);
        self.plane_transform_buffer
            .write(ctx, &self.plane_transform.uniform());

        // Render
        self.grass_renderer
            .render(ctx, &self.camera, &self.deferred_buffers);

        //Mesh
        self.mesh_renderer
            .render_models(ctx, &self.deferred_buffers, &[&self.model]);
        self.mesh_renderer
            .render(ctx, &self.deferred_buffers, &[&self.plane]);

        self.deferred_renderer.render(ctx, screen_view);
        self.gui_renderer.render(ctx, screen_view);
        self.gizmo_renderer.draw_sphere(
            1.0,
            &Transform::new(self.light, Quat::IDENTITY, Vec3::ONE),
            vec3(1.0, 0.0, 0.0),
        );
        self.gizmo_renderer.render(ctx, screen_view);

        false
    }

    fn resize(&mut self, ctx: &mut Context) {
        log::info!("resize");
        self.deferred_buffers.resize_screen(ctx);
        self.deferred_renderer.resize(
            ctx,
            &self.deferred_buffers,
            &self.camera_buffer,
            &self.light_buffer,
        );
        self.gizmo_renderer.resize(ctx);
    }

    fn init(&mut self, _ctx: &mut Context) {
        self.camera.pos = vec3(0.0, 2.0, 1.0);
        // self.camera.pitch = -PI / 4.0;
    }

    fn update(&mut self, ctx: &mut Context) -> bool {
        // pausing
        if input::key_just_pressed(ctx, KeyCode::Escape) {
            self.paused = !self.paused;

            #[cfg(not(target_arch = "wasm32"))]
            {
                render::window(ctx)
                    .set_cursor_grab(if self.paused {
                        CursorGrabMode::None
                    } else {
                        CursorGrabMode::Locked
                    })
                    .expect("could not set grab mode");
                render::window(ctx).set_cursor_visible(self.paused);
            }
        }
        if self.paused {
            return false;
        }

        self.plane_transform.pos.x = self.camera.pos.x;
        self.plane_transform.pos.z = self.camera.pos.z;

        self.camera_movement(ctx);

        // hot reload
        #[cfg(not(target_arch = "wasm32"))]
        if input::key_just_pressed(ctx, KeyCode::KeyR) {
            self.grass_renderer = pollster::block_on(GrassRenderer::new(
                ctx,
                &self.deferred_buffers,
                &self.camera_buffer,
            ));
            self.deferred_renderer = pollster::block_on(DeferredRenderer::new(
                ctx,
                wgpu::TextureFormat::Bgra8UnormSrgb,
                &self.deferred_buffers,
                &self.camera_buffer,
                &self.light_buffer,
            ));
            self.mesh_renderer = pollster::block_on(MeshRenderer::new(ctx, &self.deferred_buffers));
            println!("reload");
        }

        // debug camera pos
        if input::key_pressed(ctx, KeyCode::KeyC) {
            log::info!("{}", self.camera.pos);
        }

        // fps counter
        if input::key_pressed(ctx, KeyCode::KeyF) {
            let fps_text = (1.0 / time::frame_time(ctx)).to_string();
            self.gui_renderer.draw_text(
                &fps_text,
                vec2(0.0, 0.0),
                0.05,
                vec4(1.0, 1.0, 1.0, 1.0),
                None,
            );
        }

        false
    }
}

impl App {
    fn camera_movement(&mut self, ctx: &mut Context) {
        let dt = gbase::time::delta_time(ctx);

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
        camera_movement_dir.y = 0.0;
        if input::key_pressed(ctx, KeyCode::Space) {
            camera_movement_dir += self.camera.world_up();
        }
        if input::key_pressed(ctx, KeyCode::ShiftLeft) {
            camera_movement_dir -= self.camera.world_up();
        }
        if camera_movement_dir != Vec3::ZERO {
            if input::key_pressed(ctx, KeyCode::KeyM) {
                self.camera.pos += camera_movement_dir.normalize() * dt * CAMERA_MOVE_SPEED / 10.0;
            } else {
                self.camera.pos += camera_movement_dir.normalize() * dt * CAMERA_MOVE_SPEED;
            }
        }
    }
}

struct GrassRenderer {
    instances: render::RawBuffer,
    instance_count: render::RawBuffer,
    indirect_buffer: render::RawBuffer,
    tile_buffer: render::UniformBuffer,
    app_info: render::AppInfo,

    instance_pipeline: ArcComputePipeline,
    instance_bindgroup: ArcBindGroup,

    draw_pipeline: ArcComputePipeline,
    draw_bindgroup: ArcBindGroup,

    render_pipeline: ArcRenderPipeline,
    render_bindgroup: ArcBindGroup,

    debug_input: render::DebugInput,
}

impl GrassRenderer {
    fn render(
        &mut self,
        ctx: &Context,
        camera: &render::PerspectiveCamera,
        deferred_buffers: &render::DeferredBuffers,
    ) {
        let queue = render::queue(ctx);
        self.app_info.update_buffer(ctx);
        self.debug_input.update_buffer(ctx);

        let tile_size = TILE_SIZE as f32;
        let curr_tile = camera.pos.xz().div(tile_size).floor() * tile_size;
        let tiles = [
            vec2(curr_tile.x, curr_tile.y),                          // mid
            vec2(curr_tile.x + tile_size, curr_tile.y + 0.0),        // mid right
            vec2(curr_tile.x + -tile_size, curr_tile.y + 0.0),       // mid left
            vec2(curr_tile.x + 0.0, curr_tile.y + tile_size),        // top
            vec2(curr_tile.x + tile_size, curr_tile.y + tile_size),  // top right
            vec2(curr_tile.x + -tile_size, curr_tile.y + tile_size), // top left
            vec2(curr_tile.x + 0.0, curr_tile.y - tile_size),        // bot
            vec2(curr_tile.x + tile_size, curr_tile.y - tile_size),  // bot right
            vec2(curr_tile.x + -tile_size, curr_tile.y - tile_size), // bot left
        ];

        // TODO use one compute pass but buffers of instance counts and tiles?
        for tile in tiles {
            // update buffers
            self.instance_count.write(ctx, &[0u32]);
            self.tile_buffer.write(
                ctx,
                &Tile {
                    pos: tile,
                    size: TILE_SIZE as f32,
                    blades_per_side: BLADES_PER_SIDE as f32,
                },
            );

            let mut encoder = render::EncoderBuilder::new().build(ctx);
            // Compute
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("compute pass"),
                timestamp_writes: None,
            });

            // instance
            compute_pass.set_pipeline(&self.instance_pipeline);
            compute_pass.set_bind_group(0, &self.instance_bindgroup, &[]);
            compute_pass.dispatch_workgroups(BLADES_PER_SIDE / 16, BLADES_PER_SIDE / 16, 1);

            // draw
            compute_pass.set_pipeline(&self.draw_pipeline);
            compute_pass.set_bind_group(0, &self.draw_bindgroup, &[]);
            compute_pass.dispatch_workgroups(1, 1, 1);

            drop(compute_pass);

            // Render
            let attachments = &deferred_buffers.color_attachments();
            let mut render_pass = render::RenderPassBuilder::new()
                .color_attachments(attachments)
                .depth_stencil_attachment(deferred_buffers.depth_stencil_attachment_load())
                .build(&mut encoder);

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_vertex_buffer(0, self.instances.slice(..));
            render_pass.set_bind_group(0, &self.render_bindgroup, &[]);
            render_pass.draw_indirect(self.indirect_buffer.buffer_ref(), 0);

            drop(render_pass);

            queue.submit(Some(encoder.finish()));
        }
    }

    async fn new(
        ctx: &mut Context,
        deferred_buffers: &render::DeferredBuffers,
        camera_buffer: &render::UniformBuffer,
    ) -> Self {
        let instances = render::RawBufferBuilder::new()
            .usage(wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::STORAGE)
            .build(ctx, GrassInstanceGPU::SIZE * BLADES_PER_TILE as u64);
        let instance_count = render::RawBufferBuilder::new()
            .usage(wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST)
            .build(ctx, size_of::<u32>() as u64);
        #[rustfmt::skip]
        let indirect_buffer = render::RawBufferBuilder::new()
            .usage( wgpu::BufferUsages::INDIRECT | wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,)
            .build(ctx, size_of::<wgpu::util::DrawIndirectArgs>() as u64);
        let perlin_noise_bytes = filesystem::load_bytes(ctx, "textures/perlin_noise.png")
            .await
            .unwrap();
        let perlin_noise_texture =
            render::TextureBuilder::new(render::TextureSource::Bytes(perlin_noise_bytes))
                .build(ctx);
        let perlin_noise_sampler = render::SamplerBuilder::new().build(ctx);
        let tile_buffer = render::UniformBufferBuilder::new().build(ctx, Tile::min_size());
        let app_info = render::AppInfo::new(ctx);
        let debug_input = render::DebugInput::new(ctx);

        // Instance
        let instance_bindgroup_layout = render::BindGroupLayoutBuilder::new()
            .entries(vec![
                // instances
                render::BindGroupLayoutEntry::new().storage().compute(),
                // instance count
                render::BindGroupLayoutEntry::new().storage().compute(),
                // tile
                render::BindGroupLayoutEntry::new().uniform().compute(),
                // perlin texture
                render::BindGroupLayoutEntry::new()
                    .texture_float_filterable()
                    .compute(),
                // perlin texture sampler
                render::BindGroupLayoutEntry::new()
                    .sampler_filtering()
                    .compute(),
                // camera
                render::BindGroupLayoutEntry::new().uniform().compute(),
                // app info
                render::BindGroupLayoutEntry::new().uniform().compute(),
                // debug input
                render::BindGroupLayoutEntry::new().uniform().compute(),
            ])
            .build(ctx);
        let instance_bindgroup = render::BindGroupBuilder::new(instance_bindgroup_layout.clone())
            .entries(vec![
                // instances
                render::BindGroupEntry::Buffer(instances.buffer()),
                // instance count
                render::BindGroupEntry::Buffer(instance_count.buffer()),
                // tile
                render::BindGroupEntry::Buffer(tile_buffer.buffer()),
                // perlin texture
                render::BindGroupEntry::Texture(perlin_noise_texture.view()),
                // perlin texture sampler
                render::BindGroupEntry::Sampler(perlin_noise_sampler),
                // camera
                render::BindGroupEntry::Buffer(camera_buffer.buffer()),
                // app info
                render::BindGroupEntry::Buffer(app_info.buffer()),
                // debug input
                render::BindGroupEntry::Buffer(debug_input.buffer()),
            ])
            .build(ctx);

        let instance_shader_str =
            filesystem::load_string(ctx, "shaders/grass_compute_instance.wgsl")
                .await
                .unwrap();
        let instance_shader = render::ShaderBuilder::new(instance_shader_str).build(ctx);

        let instance_pipeline_layout = render::PipelineLayoutBuilder::new()
            .bind_groups(vec![instance_bindgroup_layout])
            .build(ctx);
        let instance_pipeline =
            render::ComputePipelineBuilder::new(instance_shader, instance_pipeline_layout)
                .label("instance".to_string())
                .build(ctx);

        // Draw
        let draw_bindgroup_layout = render::BindGroupLayoutBuilder::new()
            .entries(vec![
                // indirect buffer
                render::BindGroupLayoutEntry::new().storage().compute(),
                // instance count
                render::BindGroupLayoutEntry::new().storage().compute(),
            ])
            .build(ctx);
        let draw_bindgroup = render::BindGroupBuilder::new(draw_bindgroup_layout.clone())
            .entries(vec![
                // indirect buffer
                render::BindGroupEntry::Buffer(indirect_buffer.buffer()),
                // instance count
                render::BindGroupEntry::Buffer(instance_count.buffer()),
            ])
            .build(ctx);

        let draw_shader_str = filesystem::load_string(ctx, "shaders/grass_compute_draw.wgsl")
            .await
            .unwrap();
        let draw_compute_shader = render::ShaderBuilder::new(draw_shader_str).build(ctx);

        let draw_pipeline_layout = render::PipelineLayoutBuilder::new()
            .bind_groups(vec![draw_bindgroup_layout])
            .build(ctx);
        let draw_pipeline =
            render::ComputePipelineBuilder::new(draw_compute_shader, draw_pipeline_layout.clone())
                .label("draw".to_string())
                .build(ctx);

        // Render
        let render_bindgroup_layout = render::BindGroupLayoutBuilder::new()
            .entries(vec![
                // Camera
                render::BindGroupLayoutEntry::new()
                    .uniform()
                    .vertex()
                    .fragment(),
                // Debug input
                render::BindGroupLayoutEntry::new()
                    .uniform()
                    .vertex()
                    .fragment(),
            ])
            .build(ctx);
        let render_bindgroup = render::BindGroupBuilder::new(render_bindgroup_layout.clone())
            .entries(vec![
                // Camera
                render::BindGroupEntry::Buffer(camera_buffer.buffer()),
                // Debug
                render::BindGroupEntry::Buffer(debug_input.buffer()),
            ])
            .build(ctx);

        let render_shader_str = filesystem::load_string(ctx, "shaders/grass.wgsl")
            .await
            .unwrap();
        let render_shader = render::ShaderBuilder::new(render_shader_str).build(ctx);
        let render_pipeline_layout = render::PipelineLayoutBuilder::new()
            .bind_groups(vec![render_bindgroup_layout])
            .build(ctx);
        let render_pipeline =
            render::RenderPipelineBuilder::new(render_shader, render_pipeline_layout)
                .label("render".to_string())
                .buffers(vec![GrassInstanceGPU::desc()])
                .targets(deferred_buffers.targets().to_vec())
                .depth_stencil(deferred_buffers.depth_stencil_state())
                .topology(wgpu::PrimitiveTopology::TriangleStrip)
                .build(ctx);

        Self {
            instances,
            instance_count,
            indirect_buffer,
            tile_buffer,
            app_info,
            instance_pipeline,
            instance_bindgroup,
            draw_pipeline,
            draw_bindgroup,
            render_pipeline,
            render_bindgroup,

            debug_input,
        }
    }
}

// #[derive(ShaderType)]
// struct Lights {
//     main: Vec3,
// }

#[derive(ShaderType)]
struct Tile {
    pos: Vec2,
    size: f32,
    blades_per_side: f32,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable, Default)]
struct GrassInstanceGPU {
    position: [f32; 3],
    hash: [u32; 1],
    facing: [f32; 2],
    wind: [f32; 2],
    pad: [f32; 3],
    height: [f32; 1],
}

impl GrassInstanceGPU {
    const SIZE: u64 = std::mem::size_of::<Self>() as u64;
    const ATTRIBUTES: [wgpu::VertexAttribute; 6] = wgpu::vertex_attr_array![
        1=>Float32x3,   // pos
        2=>Uint32,      // hash
        3=>Float32x2,   // facing
        4=>Float32x2,   // wind
        5=>Float32x3,   // pad
        6=>Float32,     // height
    ];
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: Self::SIZE,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &Self::ATTRIBUTES,
        }
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
