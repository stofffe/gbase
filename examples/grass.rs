use encase::ShaderType;
use gbase::{
    filesystem, input,
    render::{self, BindGroupCombinedEntry, Transform, VertexColor},
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
        .log_level(LogLevel::Warn)
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
const PLANE_COLOR: [f32; 3] = [0.025, 0.1, 0.005];

struct App {
    plane_buffer: render::VertexBuffer<render::VertexColor>,
    plane_transform: render::Transform,
    plane_transform_buffer: render::UniformBuffer,
    plane_bind_group: wgpu::BindGroup,
    plane_pipeline: wgpu::RenderPipeline,

    camera: render::PerspectiveCamera,
    camera_buffer: render::UniformBuffer,

    depth_buffer: render::DepthBuffer,
    depth_buffer_renderer: render::DepthBufferRenderer,

    grass_renderer: GrassRenderer,
    gui_renderer: render::GUIRenderer,
    gizmo_renderer: render::GizmoRenderer,

    paused: bool,
}

impl App {
    async fn new(ctx: &mut Context) -> Self {
        let depth_buffer = render::DepthBufferBuilder::new()
            .screen_size(ctx)
            .build(ctx);
        let depth_buffer_renderer = render::DepthBufferRenderer::new(ctx, &depth_buffer);

        let plane_buffer = render::VertexBufferBuilder::new(CENTERED_QUAD_VERTICES)
            .usage(wgpu::BufferUsages::VERTEX)
            .build(ctx);

        // Camera
        let camera = render::PerspectiveCamera::new();
        let camera_buffer = render::UniformBufferBuilder::new()
            .label("camera buf")
            .usage(wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST)
            .build(ctx, render::PerspectiveCameraUniform::min_size());

        // Plane
        let plane_transform = render::Transform::new(
            vec3(0.0, 0.0, 0.0),
            Quat::from_rotation_x(PI / 2.0),
            vec3(PLANE_SIZE, PLANE_SIZE, 1.0),
        );
        let plane_transform_buffer = render::UniformBufferBuilder::new()
            .label("plane buf")
            .usage(wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST)
            .build_init(ctx, &plane_transform.uniform());

        // Bind group
        let (plane_bindgroup_layout, plane_bindgroup) = render::BindGroupCombinedBuilder::new()
            .entries(&[
                // camera
                render::BindGroupCombinedEntry::new(camera_buffer.resource()).uniform(),
                // plane transform
                render::BindGroupCombinedEntry::new(plane_transform_buffer.resource()).uniform(),
            ])
            .build(ctx);

        // Shader
        let shader_str = filesystem::load_string(ctx, "new_shader.wgsl")
            .await
            .unwrap();
        let shader = render::ShaderBuilder::new().build(ctx, &shader_str);

        // Pipeline
        let plane_pipeline = render::RenderPipelineBuilder::new(&shader)
            .buffers(&[VertexColor::desc()])
            .bind_groups(&[&plane_bindgroup_layout])
            .targets(&[render::RenderPipelineBuilder::default_target(ctx)])
            .depth_stencil(depth_buffer.depth_stencil_state())
            .build(ctx);

        let grass_renderer = GrassRenderer::new(ctx, &camera_buffer, &depth_buffer).await;

        let gui_renderer = render::GUIRenderer::new(
            ctx,
            1000 * 4,
            1000 * 6,
            &filesystem::load_bytes(ctx, "font.ttf").await.unwrap(),
            render::DEFAULT_SUPPORTED_CHARS,
        )
        .await;

        let gizmo_renderer = render::GizmoRenderer::new(ctx);

        Self {
            plane_buffer,

            camera,
            camera_buffer,

            plane_transform_buffer,
            plane_transform,

            plane_bind_group: plane_bindgroup,
            plane_pipeline,

            depth_buffer,
            depth_buffer_renderer,

            grass_renderer,
            gui_renderer,
            gizmo_renderer,

            paused: false,
        }
    }
}

impl Callbacks for App {
    fn render(&mut self, ctx: &mut Context, screen_view: &wgpu::TextureView) -> bool {
        let queue = render::queue(ctx);

        // Clear background and depth buffer
        let mut encoder = render::create_encoder(ctx, None);
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
            depth_stencil_attachment: Some(self.depth_buffer.depth_render_attachment_clear()),
            // depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
            //     view: self.depth_buffer.framebuffer().view(), // TODO
            //     depth_ops: Some(wgpu::Operations {
            //         load: wgpu::LoadOp::Clear(1.0),
            //         store: wgpu::StoreOp::Store,
            //     }),
            //     stencil_ops: None,
            // }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        drop(clear_pass);

        // update buffers
        self.camera_buffer.write(ctx, &self.camera.uniform(ctx));
        self.plane_transform_buffer
            .write(ctx, &self.plane_transform.uniform());

        // Render
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("render pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: screen_view,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
                resolve_target: None,
            })],
            depth_stencil_attachment: Some(self.depth_buffer.depth_render_attachment_load()),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        // Plane
        render_pass.set_pipeline(&self.plane_pipeline);
        render_pass.set_vertex_buffer(0, self.plane_buffer.slice(..));
        render_pass.set_bind_group(0, &self.plane_bind_group, &[]);
        render_pass.draw(0..CENTERED_QUAD_VERTICES.len() as u32, 0..1);

        drop(render_pass);

        queue.submit(Some(encoder.finish()));

        self.grass_renderer.render(
            ctx,
            screen_view,
            &self.camera,
            &self.depth_buffer,
            &mut self.gizmo_renderer,
        );

        self.gui_renderer.render(ctx, screen_view);

        if input::key_pressed(ctx, KeyCode::KeyP) {
            self.depth_buffer_renderer.render(ctx, screen_view);
        }

        self.gizmo_renderer
            .render(ctx, screen_view, &mut self.camera);

        false
    }

    fn resize(&mut self, ctx: &mut Context) {
        self.depth_buffer.resize(ctx);
        self.depth_buffer_renderer.resize(ctx, &self.depth_buffer);
        self.gizmo_renderer.resize(ctx);
    }

    fn init(&mut self, _ctx: &mut Context) {
        self.camera.pos = vec3(0.0, 2.0, 1.0);
        self.camera.pitch = -PI / 4.0;
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
                &self.camera_buffer,
                &self.depth_buffer,
            ));
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
        // if input::mouse_button_pressed(ctx, input::MouseButton::Left) {}
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
    debug_input: render::DebugInput,
    app_info: render::AppInfo,

    instance_pipeline: wgpu::ComputePipeline,
    instance_bindgroup: wgpu::BindGroup,

    draw_pipeline: wgpu::ComputePipeline,
    draw_bindgroup: wgpu::BindGroup,

    render_pipeline: wgpu::RenderPipeline,
    render_bindgroup: wgpu::BindGroup,

    lights: Lights,
    lights_buffer: render::UniformBuffer,
}

impl GrassRenderer {
    fn render(
        &mut self,
        ctx: &Context,
        screen_view: &wgpu::TextureView,
        camera: &render::PerspectiveCamera,
        depth_buffer: &render::DepthBuffer,
        gizmo_renderer: &mut render::GizmoRenderer,
    ) {
        self.debug_input.update_buffer(ctx);
        self.app_info.update_buffer(ctx);
        self.lights_buffer.write(ctx, &self.lights);

        // DEBUG ligth pos
        gizmo_renderer.draw_sphere(
            1.0,
            &Transform::new(self.lights.main, Quat::default(), Vec3::ONE),
            vec3(1.0, 0.0, 0.0),
        );

        let queue = render::queue(ctx);

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

            // run compute
            let mut encoder = render::EncoderBuilder::new().build(ctx);
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
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: screen_view,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                    resolve_target: None,
                })],
                depth_stencil_attachment: Some(depth_buffer.depth_render_attachment_load()),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_vertex_buffer(0, self.instances.slice(..));
            render_pass.set_bind_group(0, &self.render_bindgroup, &[]);
            render_pass.draw_indirect(self.indirect_buffer.buf(), 0);

            drop(render_pass);

            queue.submit(Some(encoder.finish()));
        }
    }

    async fn new(
        ctx: &Context,
        camera_buffer: &render::UniformBuffer,
        depth_buffer: &render::DepthBuffer,
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
        let perlin_noise_bytes = filesystem::load_bytes(ctx, "perlin_noise.png")
            .await
            .unwrap();
        let perlin_noise_texture =
            render::TextureBuilder::new().build_init(ctx, &perlin_noise_bytes);
        let perlin_noise_sampler = render::SamplerBuilder::new().build(ctx);
        let tile_buffer = render::UniformBufferBuilder::new().build(ctx, Tile::min_size());
        let app_info = render::AppInfo::new(ctx);
        let debug_input = render::DebugInput::new(ctx);

        let lights = Lights {
            main: vec3(30.0, 10.0, 30.0),
        };
        let lights_buffer = render::UniformBufferBuilder::new().build(ctx, Lights::min_size());

        // Instance

        let (instance_bindgroup_layout, instance_bindgroup) =
            render::BindGroupCombinedBuilder::new()
                .entries(&[
                    // instances
                    render::BindGroupCombinedEntry::new(instances.resource())
                        .visibility(wgpu::ShaderStages::COMPUTE)
                        .storage(false),
                    // instance count
                    render::BindGroupCombinedEntry::new(instance_count.resource())
                        .visibility(wgpu::ShaderStages::COMPUTE)
                        .storage(false),
                    // tile
                    render::BindGroupCombinedEntry::new(tile_buffer.resource())
                        .visibility(wgpu::ShaderStages::COMPUTE)
                        .uniform(),
                    // perlin texture
                    render::BindGroupCombinedEntry::new(perlin_noise_texture.resource())
                        .ty(perlin_noise_texture.binding_type())
                        .visibility(wgpu::ShaderStages::COMPUTE),
                    // perlin texture sampler
                    render::BindGroupCombinedEntry::new(perlin_noise_sampler.resource())
                        .ty(perlin_noise_sampler.binding_filtering())
                        .visibility(wgpu::ShaderStages::COMPUTE),
                    // camera
                    render::BindGroupCombinedEntry::new(camera_buffer.resource())
                        .visibility(wgpu::ShaderStages::COMPUTE)
                        .uniform(),
                    // app info
                    render::BindGroupCombinedEntry::new(app_info.buffer().as_entire_binding())
                        .uniform()
                        .visibility(wgpu::ShaderStages::COMPUTE),
                    // debug input
                    render::BindGroupCombinedEntry::new(debug_input.buffer().as_entire_binding())
                        .uniform()
                        .visibility(wgpu::ShaderStages::COMPUTE),
                ])
                .build(ctx);

        let instance_shader_str = filesystem::load_string(ctx, "grass_compute_instance.wgsl")
            .await
            .unwrap();
        let instance_shader = render::ShaderBuilder::new().build(ctx, &instance_shader_str);

        let instance_pipeline = render::ComputePipelineBuilder::new(&instance_shader)
            .label("instance")
            .bind_groups(&[&instance_bindgroup_layout])
            .build(ctx);

        // Draw
        let (draw_bindgroup_layout, draw_bindgroup) = render::BindGroupCombinedBuilder::new()
            .entries(&[
                render::BindGroupCombinedEntry::new(indirect_buffer.resource())
                    .visibility(wgpu::ShaderStages::COMPUTE)
                    .storage(false),
                render::BindGroupCombinedEntry::new(instance_count.resource())
                    .visibility(wgpu::ShaderStages::COMPUTE)
                    .storage(false),
            ])
            .build(ctx);

        let draw_shader_str = filesystem::load_string(ctx, "grass_compute_draw.wgsl")
            .await
            .unwrap();
        let draw_compute_shader = render::ShaderBuilder::new().build(ctx, &draw_shader_str);

        let draw_pipeline = render::ComputePipelineBuilder::new(&draw_compute_shader)
            .label("draw")
            .bind_groups(&[&draw_bindgroup_layout])
            .build(ctx);

        // Render
        let (render_bindgroup_layout, render_bindgroup) = render::BindGroupCombinedBuilder::new()
            .entries(&[
                BindGroupCombinedEntry::new(camera_buffer.resource())
                    .visibility(wgpu::ShaderStages::VERTEX_FRAGMENT)
                    .uniform(),
                BindGroupCombinedEntry::new(app_info.buffer().as_entire_binding())
                    .visibility(wgpu::ShaderStages::VERTEX_FRAGMENT)
                    .uniform(),
                BindGroupCombinedEntry::new(debug_input.buffer().as_entire_binding())
                    .visibility(wgpu::ShaderStages::VERTEX_FRAGMENT)
                    .uniform(),
                BindGroupCombinedEntry::new(lights_buffer.resource())
                    .visibility(wgpu::ShaderStages::VERTEX_FRAGMENT)
                    .uniform(),
            ])
            .build(ctx);

        let render_shader_str = filesystem::load_string(ctx, "grass.wgsl").await.unwrap();
        let render_shader = render::ShaderBuilder::new().build(ctx, &render_shader_str);
        let render_pipeline = render::RenderPipelineBuilder::new(&render_shader)
            .label("render")
            .buffers(&[GrassInstanceGPU::desc()])
            .bind_groups(&[&render_bindgroup_layout])
            .targets(&[render::RenderPipelineBuilder::default_target(ctx)])
            .depth_stencil(depth_buffer.depth_stencil_state())
            .topology(wgpu::PrimitiveTopology::TriangleStrip)
            .build(ctx);

        Self {
            instances,
            instance_count,
            indirect_buffer,
            tile_buffer,
            app_info,
            debug_input,

            instance_pipeline,
            instance_bindgroup,
            draw_pipeline,
            draw_bindgroup,
            render_pipeline,
            render_bindgroup,

            lights,
            lights_buffer,
        }
    }
}

#[derive(ShaderType)]
struct Lights {
    main: Vec3,
}

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
const CENTERED_QUAD_VERTICES: &[VertexColor] = &[
    VertexColor { position: [-0.5, -0.5, 0.0], color: PLANE_COLOR }, // bottom left
    VertexColor { position: [ 0.5, -0.5, 0.0], color: PLANE_COLOR }, // bottom right
    VertexColor { position: [ 0.5,  0.5, 0.0], color: PLANE_COLOR }, // top right

    VertexColor { position: [-0.5, -0.5, 0.0], color: PLANE_COLOR }, // bottom left
    VertexColor { position: [ 0.5,  0.5, 0.0], color: PLANE_COLOR }, // top right
    VertexColor { position: [-0.5,  0.5, 0.0], color: PLANE_COLOR }, // top left

];
