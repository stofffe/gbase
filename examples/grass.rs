use encase::ShaderType;
use gbase::{
    filesystem, input,
    render::{self, InstaceTrait, InstanceGpuTrait, VertexColor},
    time, Callbacks, Context, ContextBuilder, LogLevel,
};
use glam::{vec2, vec3, vec4, Quat, Vec2, Vec3, Vec3Swizzles};
use std::{f32::consts::PI, ops::Div, path::Path};
use winit::keyboard::KeyCode;

#[pollster::main]
pub async fn main() {
    let (mut ctx, ev) = ContextBuilder::new()
        .log_level(LogLevel::Info)
        .vsync(false)
        .build()
        .await;
    let app = App::new(&mut ctx).await;
    gbase::run(app, ctx, ev).await;
}

const TILE_SIZE: u32 = 80;
const BLADES_PER_SIDE: u32 = 16 * 20; // must be > 16 due to dispatch(B/16, B/16, 1) workgroups(16,16,1)
const BLADES_PER_TILE: u32 = BLADES_PER_SIDE * BLADES_PER_SIDE;
const CAMERA_MOVE_SPEED: f32 = 15.0;

const PLANE_SIZE: f32 = 500.0;
const PLANE_COLOR: [f32; 3] = [0.025, 0.1, 0.005];

struct App {
    plane_buffer: render::VertexBuffer<VertexColor>,
    plane_pipeline: render::RenderPipeline,
    plane_transform: render::Transform,

    camera: render::PerspectiveCamera,

    depth_buffer: render::DepthBuffer,
    depth_buffer_renderer: render::DepthBufferRenderer,

    grass_renderer: GrassRenderer,

    gui_renderer: render::GUIRenderer,
}

impl App {
    async fn new(ctx: &mut Context) -> Self {
        let device = render::device(ctx);
        let surface_config = render::surface_config(ctx);

        // Camera
        let camera = render::PerspectiveCamera::new(device)
            .pos(vec3(0.0, 2.0, 1.0))
            .pitch(-PI / 4.0);

        // Plane
        let plane_transform = render::Transform::new(device)
            .rotation(Quat::from_rotation_x(PI / 2.0))
            .scale(vec3(PLANE_SIZE, PLANE_SIZE, 1.0))
            .pos(vec3(0.0, -0.1, 0.0)); // TODO TEMP

        let plane_buffer = render::VertexBufferBuilder::new()
            .source(render::BufferSource::Values(CENTERED_QUAD_VERTICES))
            .build(ctx);

        let depth_buffer = render::DepthBuffer::new(ctx);

        // Plane pipeline
        let shader = render::ShaderBuilder::new("shader.wgsl")
            .buffers(vec![plane_buffer.desc()])
            .targets(vec![Some(wgpu::ColorTargetState {
                format: surface_config.format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })])
            .bind_group_layouts(vec![
                camera.bind_group_layout(),
                plane_transform.bind_group_layout(),
            ])
            .build(ctx)
            .await;

        let plane_pipeline = render::RenderPipelineBuilder::new(&shader)
            .depth_buffer(render::DepthBuffer::depth_stencil_state())
            .build(ctx);

        let depth_buffer_renderer = render::DepthBufferRenderer::new(ctx, &depth_buffer);

        let grass_renderer = GrassRenderer::new(ctx, &camera).await;

        let gui_renderer = render::GUIRenderer::new(
            ctx,
            1000 * 4,
            1000 * 6,
            &filesystem::load_bytes(ctx, "font.ttf").await.unwrap(),
            render::DEFAULT_SUPPORTED_CHARS,
        )
        .await;

        Self {
            camera,
            plane_buffer,
            plane_pipeline,
            plane_transform,

            depth_buffer,
            depth_buffer_renderer,

            grass_renderer,

            gui_renderer,
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
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &self.depth_buffer.view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        drop(clear_pass);

        // update buffers
        self.camera.update_buffer(ctx);
        self.plane_transform.update_buffer(ctx);

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
            depth_stencil_attachment: Some(self.depth_buffer.depth_stencil_attachment_load()),
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        // Plane
        render_pass.set_pipeline(self.plane_pipeline.pipeline());
        render_pass.set_vertex_buffer(0, self.plane_buffer.slice(..));
        render_pass.set_bind_group(0, self.camera.bind_group(), &[]);
        render_pass.set_bind_group(1, self.plane_transform.bind_group(), &[]);
        render_pass.draw(0..self.plane_buffer.len(), 0..1);

        drop(render_pass);

        queue.submit(Some(encoder.finish()));

        self.grass_renderer
            .render(ctx, &self.camera, screen_view, &self.depth_buffer);

        self.gui_renderer.render(ctx, screen_view);

        if input::key_pressed(ctx, KeyCode::F2) {
            self.depth_buffer_renderer.render(ctx, screen_view);
        }

        false
    }

    fn resize(&mut self, ctx: &mut Context) {
        let device = render::device(ctx);
        let surface_config = render::surface_config(ctx);
        self.depth_buffer.resize(device, surface_config);
        self.depth_buffer_renderer
            .resize(device, &self.depth_buffer);
    }

    fn update(&mut self, ctx: &mut Context) -> bool {
        self.plane_transform.pos.x = self.camera.pos.x;
        self.plane_transform.pos.z = self.camera.pos.z;

        self.camera_movement(ctx);

        // hot reload
        #[cfg(not(target_arch = "wasm32"))]
        if input::key_just_pressed(ctx, KeyCode::KeyR) {
            self.grass_renderer = pollster::block_on(GrassRenderer::new(ctx, &self.camera));
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
    instances: render::InstanceBuffer<GrassInstanceGPU, GrassInstance>,
    grass_pipeline: render::RenderPipeline,

    instance_compute_pipeline: render::ComputePipeline,
    instance_compute_bindgroup: render::BindGroup,
    instance_count: wgpu::Buffer,

    draw_compute_pipeline: render::ComputePipeline,
    draw_compute_bindgroup: render::BindGroup,
    indirect_buffer: wgpu::Buffer,

    perlin_noise_texture: render::Texture,
    tile_buffer: wgpu::Buffer,
}

impl GrassRenderer {
    fn render(
        &mut self,
        ctx: &Context,
        camera: &render::PerspectiveCamera,
        screen_view: &wgpu::TextureView,
        depth_buffer: &render::DepthBuffer,
    ) {
        let queue = render::queue(ctx);

        //let [tile_x, tile_z] = camera.pos.xz().div(TILE_SIZE).floor().as_ivec2().to_array();
        let tile_size = TILE_SIZE as f32;
        let curr_tile = camera.pos.xz().div(tile_size).floor() * tile_size;
        let tiles = [
            // vec2(0.0, 0.0),
            vec2(curr_tile.x, curr_tile.y),                    // mid
            vec2(curr_tile.x + tile_size, curr_tile.y + 0.0),  // mid right
            vec2(curr_tile.x + -tile_size, curr_tile.y + 0.0), // mid left
            vec2(curr_tile.x + 0.0, curr_tile.y + tile_size),  // top
            vec2(curr_tile.x + tile_size, curr_tile.y + tile_size), // top right
            vec2(curr_tile.x + -tile_size, curr_tile.y + tile_size), // top left
            vec2(curr_tile.x + 0.0, curr_tile.y - tile_size),  // bot
            vec2(curr_tile.x + tile_size, curr_tile.y - tile_size), // bot right
            vec2(curr_tile.x + -tile_size, curr_tile.y - tile_size), // bot left
        ];
        // TODO use one compute pass but buffers of instance counts and tiles?
        for tile in tiles {
            // update buffers
            queue.write_buffer(&self.instance_count, 0, bytemuck::cast_slice(&[0u32])); // clear instance count
            let mut buffer = encase::UniformBuffer::new(Vec::new());
            buffer
                .write(&Tile {
                    pos: tile,
                    size: TILE_SIZE as f32,
                    blades_per_side: BLADES_PER_SIDE as f32,
                })
                .unwrap();
            queue.write_buffer(&self.tile_buffer, 0, &buffer.into_inner());

            // run compute
            let mut encoder = render::create_encoder(ctx, None);
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("compute pass"),
                timestamp_writes: None,
            });

            let time_info = render::time_info(ctx);

            // instance
            compute_pass.set_pipeline(self.instance_compute_pipeline.pipeline());
            compute_pass.set_bind_group(0, self.instance_compute_bindgroup.bind_group(), &[]);
            compute_pass.set_bind_group(1, self.perlin_noise_texture.bind_group(), &[]);
            compute_pass.set_bind_group(2, camera.bind_group(), &[]);
            compute_pass.set_bind_group(3, time_info.bind_group(), &[]);
            compute_pass.dispatch_workgroups(BLADES_PER_SIDE / 16, BLADES_PER_SIDE / 16, 1);

            // draw
            compute_pass.set_pipeline(self.draw_compute_pipeline.pipeline());
            compute_pass.set_bind_group(0, self.draw_compute_bindgroup.bind_group(), &[]);
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
                depth_stencil_attachment: Some(depth_buffer.depth_stencil_attachment_load()),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            render_pass.set_pipeline(self.grass_pipeline.pipeline());
            render_pass.set_vertex_buffer(0, self.instances.slice(..));
            render_pass.set_bind_group(0, camera.bind_group(), &[]);
            render_pass.set_bind_group(1, time_info.bind_group(), &[]);
            render_pass.draw_indirect(&self.indirect_buffer, 0);

            drop(render_pass);

            queue.submit(Some(encoder.finish()));
        }
    }
    async fn new(ctx: &Context, camera: &render::PerspectiveCamera) -> Self {
        let device = render::device(ctx);
        let surface_config = render::surface_config(ctx);

        // Buffers
        let instances = render::InstanceBuffer::new_empty(device, BLADES_PER_TILE as u64);
        let instance_count = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("instance count"),
            size: std::mem::size_of::<u32>() as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST, // TODO DST temp
            mapped_at_creation: false,
        });
        let indirect_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("grass tiles buffer"),
            size: std::mem::size_of::<wgpu::util::DrawIndirect>() as u64,
            usage: wgpu::BufferUsages::INDIRECT
                | wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let perlin_noise_bytes = filesystem::load_bytes(ctx, Path::new("perlin_noise.png"))
            .await
            .unwrap();
        let perlin_noise_texture =
            render::TextureBuilder::new(render::TextureSource::FormattedBytes(perlin_noise_bytes))
                .visibility(wgpu::ShaderStages::COMPUTE)
                .build(ctx);

        let tile_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: u64::from(Tile::min_size()),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Compute instance
        let instance_compute_bindgroup = render::BindGroupBuilder::new(vec![
            // instances
            render::BindGroupEntry::new(instances.buffer().as_entire_binding())
                .visibility(wgpu::ShaderStages::COMPUTE)
                .storage(false),
            // instance count
            render::BindGroupEntry::new(instance_count.as_entire_binding())
                .visibility(wgpu::ShaderStages::COMPUTE)
                .storage(false),
            // tile
            render::BindGroupEntry::new(tile_buffer.as_entire_binding())
                .visibility(wgpu::ShaderStages::COMPUTE)
                .uniform(),
        ])
        .build(ctx);

        let time_info = render::time_info(ctx);
        let instance_compute_shader =
            render::ShaderBuilder::new("grass_compute_instance.wgsl".to_string())
                .bind_group_layouts(vec![
                    instance_compute_bindgroup.bind_group_layout(),
                    perlin_noise_texture.bind_group_layout(),
                    camera.bind_group_layout(),
                    time_info.bind_group_layout(),
                ])
                .build(ctx)
                .await;

        let instance_compute_pipeline =
            render::ComputePipelineBuilder::new(&instance_compute_shader).build(ctx);

        // Compute draw
        let draw_compute_bindgroup = render::BindGroupBuilder::new(vec![
            render::BindGroupEntry::new(indirect_buffer.as_entire_binding())
                .visibility(wgpu::ShaderStages::COMPUTE)
                .storage(false),
            render::BindGroupEntry::new(instance_count.as_entire_binding())
                .visibility(wgpu::ShaderStages::COMPUTE)
                .storage(false),
        ])
        .build(ctx);

        let draw_compute_shader = render::ShaderBuilder::new("grass_compute_draw.wgsl".to_string())
            .bind_group_layouts(vec![draw_compute_bindgroup.bind_group_layout()])
            .build(ctx)
            .await;
        let draw_compute_pipeline =
            render::ComputePipelineBuilder::new(&draw_compute_shader).build(ctx);

        // Render pipeline
        let render_shader = render::ShaderBuilder::new("grass.wgsl".to_string())
            .buffers(vec![instances.desc()])
            .default_target(surface_config)
            .bind_group_layouts(vec![
                &camera.bind_group_layout(),
                &time_info.bind_group_layout(),
            ])
            .build(ctx)
            .await;
        let render_pipeline = render::RenderPipelineBuilder::new(&render_shader)
            .topology(wgpu::PrimitiveTopology::TriangleStrip)
            .depth_buffer(render::DepthBuffer::depth_stencil_state())
            .build(ctx);

        Self {
            instances,
            grass_pipeline: render_pipeline,

            indirect_buffer,
            draw_compute_pipeline,
            draw_compute_bindgroup,

            instance_count,
            instance_compute_pipeline,
            instance_compute_bindgroup,

            perlin_noise_texture,
            tile_buffer,
        }
    }
}

#[derive(ShaderType)]
struct Tile {
    pos: Vec2,
    size: f32,
    blades_per_side: f32,
}

// TODO MUST ALIGN TO 16 (wgpu requirement)
struct GrassInstance {
    pos: Vec3,
    facing: Vec2,
}

impl GrassInstance {
    fn to_gpu(&self) -> GrassInstanceGPU {
        GrassInstanceGPU {
            position: self.pos.to_array(),
            facing: self.facing.to_array(),
            hash: [0],
            wind: [0.0, 0.0],
            pad: [0.0, 0.0, 0.0],
            height: [0.0],
        }
    }
}

impl InstaceTrait<GrassInstanceGPU> for GrassInstance {
    fn to_gpu(&self) -> GrassInstanceGPU {
        self.to_gpu()
    }
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

impl InstanceGpuTrait for GrassInstanceGPU {
    const SIZE: u64 = GrassInstanceGPU::SIZE;
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        GrassInstanceGPU::desc()
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

// fn compute(
//     &mut self,
//     ctx: &Context,
//     encoder: &mut wgpu::CommandEncoder,
//     camera: &render::PerspectiveCamera,
// ) {
//     let queue = render::queue(ctx);
//
//     // clear instance count
//     queue.write_buffer(&self.instance_count, 0, bytemuck::cast_slice(&[0u32]));
//
//     // tile
//     let mut buffer = encase::UniformBuffer::new(Vec::new());
//     buffer
//         .write(&Tile {
//             pos: camera.pos.xz(),
//         })
//         .unwrap();
//     queue.write_buffer(&self.tile_buffer, 0, &buffer.into_inner());
//
//     // run compute
//     let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
//         label: Some("compute pass"),
//         timestamp_writes: None,
//     });
//
//     let time_info = render::time_info(ctx);
//     // instance
//     compute_pass.set_pipeline(self.instance_compute_pipeline.pipeline());
//     compute_pass.set_bind_group(0, self.instance_compute_bindgroup.bind_group(), &[]);
//     compute_pass.set_bind_group(1, self.perlin_noise_texture.bind_group(), &[]);
//     compute_pass.set_bind_group(2, camera.bind_group(), &[]);
//     compute_pass.set_bind_group(3, time_info.bind_group(), &[]);
//     compute_pass.dispatch_workgroups(BLADES_PER_SIDE / 16, BLADES_PER_SIDE / 16, 1);
//
//     // draw
//     compute_pass.set_pipeline(self.draw_compute_pipeline.pipeline());
//     compute_pass.set_bind_group(0, self.draw_compute_bindgroup.bind_group(), &[]);
//     compute_pass.dispatch_workgroups(1, 1, 1);
// }
//
// fn render<'a>(
//     &'a self,
//     ctx: &'a Context,
//     render_pass: &mut wgpu::RenderPass<'a>,
//     camera: &'a render::PerspectiveCamera,
// ) {
//     let time_info = render::time_info(ctx);
//
//     render_pass.set_pipeline(self.grass_pipeline.pipeline());
//     render_pass.set_vertex_buffer(0, self.instances.slice(..));
//     render_pass.set_bind_group(0, camera.bind_group(), &[]);
//     render_pass.set_bind_group(1, time_info.bind_group(), &[]);
//     render_pass.draw_indirect(&self.indirect_buffer, 0);
// }
//
