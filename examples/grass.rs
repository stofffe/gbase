use bytemuck::bytes_of;
use gbase::{
    filesystem, input,
    render::{self, InstaceTrait, InstanceGpuTrait, VertexColor, VertexUV},
    Callbacks, Context, ContextBuilder, LogLevel,
};
use glam::{vec2, vec3, Quat, Vec2, Vec3, Vec3Swizzles};
use std::{
    collections::hash_map::DefaultHasher,
    f32::consts::PI,
    hash::{Hash, Hasher},
    ops::Div,
    path::Path,
};
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

const TILE_SIZE: f32 = 16.0;
const BLADES_PER_SIDE: u32 = 16 * 1; // must be > 16 due to dispatch(B/16, B/16, 1) workgroups(16,16,1)
const BLADES_PER_TILE: u32 = BLADES_PER_SIDE * BLADES_PER_SIDE;

const PLANE_SIZE: f32 = 100.0;
// const LIGHT_INIT_POS: Vec3 = vec3(10.0, 10.0, 0.0);

struct App {
    plane_buffer: render::VertexBuffer<VertexColor>,
    plane_pipeline: wgpu::RenderPipeline,
    plane_transform: render::Transform,

    camera: render::PerspectiveCamera,

    depth_buffer: render::DepthBuffer,
    depth_buffer_renderer: render::DepthBufferRenderer,

    grass_renderer: GrassRenderer,
}

impl App {
    async fn new(ctx: &mut Context) -> Self {
        let device = render::device(ctx);
        let surface_config = render::surface_config(ctx);

        // Camera
        let camera = render::PerspectiveCamera::new(&device)
            .pos(vec3(0.0, 2.0, -1.0))
            .pitch(PI / 4.0);

        // Plane
        let plane_transform = render::Transform::new(&device)
            .rotation(Quat::from_rotation_x(PI / 2.0))
            .scale(vec3(PLANE_SIZE, PLANE_SIZE, 1.0));
        let plane_buffer = render::VertexBuffer::new(&device, CENTERED_QUAD_VERTICES);

        // Plane pipeline
        let shader_str = filesystem::load_string(ctx, Path::new("shader.wgsl"))
            .await
            .unwrap();
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(shader_str.into()),
        });

        let plane_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("render pipeline layout"),
                bind_group_layouts: &[
                    &camera.bind_group_layout,
                    &plane_transform.bind_group_layout,
                ],
                push_constant_ranges: &[],
            });

        let plane_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("render pipeline"),
            layout: Some(&plane_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[plane_buffer.desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_config.format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        let depth_buffer = render::DepthBuffer::new(&device, &surface_config);
        let depth_buffer_renderer = render::DepthBufferRenderer::new(ctx, &depth_buffer);

        let grass_renderer = GrassRenderer::new(ctx, &camera).await;

        Self {
            camera,
            plane_buffer,
            plane_pipeline,
            plane_transform,

            depth_buffer,
            depth_buffer_renderer,

            grass_renderer,
        }
    }
}

impl Callbacks for App {
    fn render(
        &mut self,
        ctx: &mut Context,
        encoder: &mut wgpu::CommandEncoder,
        screen_view: &wgpu::TextureView,
    ) -> bool {
        let device = render::device(ctx);
        let queue = render::queue(ctx);

        self.camera.update_buffer(ctx);
        self.plane_transform.update_buffer(ctx);

        self.grass_renderer.compute(ctx, encoder);

        // Render
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("render pass"),
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

        // Plane
        render_pass.set_pipeline(&self.plane_pipeline);
        render_pass.set_vertex_buffer(0, self.plane_buffer.buffer.slice(..));
        render_pass.set_bind_group(0, &self.camera.bind_group, &[]);
        render_pass.set_bind_group(1, &self.plane_transform.bind_group, &[]);
        render_pass.draw(0..self.plane_buffer.len(), 0..1);

        self.grass_renderer
            .render(ctx, &mut render_pass, &self.camera);

        drop(render_pass);

        if input::key_pressed(ctx, KeyCode::F2) {
            self.depth_buffer_renderer.render(encoder, screen_view);
        }

        false
    }

    fn resize(&mut self, ctx: &mut Context) {
        let device = render::device(ctx);
        let surface_config = render::surface_config(ctx);
        self.depth_buffer.resize(&device, &surface_config);
        self.depth_buffer_renderer
            .resize(&device, &self.depth_buffer);
    }

    fn update(&mut self, ctx: &mut Context) -> bool {
        self.plane_transform.pos.x = self.camera.pos.x;
        self.plane_transform.pos.z = self.camera.pos.z;

        self.camera_movement(ctx);

        self.grass_renderer.update(&self.camera);

        log::info!("{}", gbase::time::fps(ctx));
        false
    }
}

impl App {
    fn camera_movement(&mut self, ctx: &mut Context) {
        let dt = gbase::time::delta_time(ctx);

        // Camera rotation
        if input::mouse_button_pressed(ctx, input::MouseButton::Left) {
            let (mouse_dx, mouse_dy) = input::mouse_delta(ctx);
            self.camera.yaw += 1.0 * dt * mouse_dx;
            self.camera.pitch -= 1.0 * dt * mouse_dy;
        }

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
            self.camera.pos += camera_movement_dir.normalize() * dt;
        }
    }
}

#[rustfmt::skip]
const CENTERED_QUAD_VERTICES: &[VertexColor] = &[
    VertexColor { position: [-0.5, -0.5, 0.0], color: [0.7, 0.5, 0.2] }, // bottom left
    VertexColor { position: [ 0.5, -0.5, 0.0], color: [0.7, 0.5, 0.2] }, // bottom right
    VertexColor { position: [ 0.5,  0.5, 0.0], color: [0.7, 0.5, 0.2] }, // top right

    VertexColor { position: [-0.5, -0.5, 0.0], color: [0.7, 0.5, 0.2] }, // bottom left
    VertexColor { position: [ 0.5,  0.5, 0.0], color: [0.7, 0.5, 0.2] }, // top right
    VertexColor { position: [-0.5,  0.5, 0.0], color: [0.7, 0.5, 0.2] }, // top left

];

struct GrassRenderer {
    instances: render::InstanceBuffer<GrassInstanceGPU, GrassInstance>,
    grass_pipeline: wgpu::RenderPipeline,

    instance_compute_pipeline: wgpu::ComputePipeline,
    instance_compute_bindgroup: wgpu::BindGroup,
    instance_count: wgpu::Buffer,

    draw_compute_pipeline: wgpu::ComputePipeline,
    draw_compute_bindgroup: wgpu::BindGroup,
    indirect_buffer: wgpu::Buffer,
}

impl GrassRenderer {
    fn update(&mut self, camera: &render::PerspectiveCamera) {
        self.instances.vec.clear();

        let [tile_x, tile_z] = camera.pos.xz().div(TILE_SIZE).floor().as_ivec2().to_array();
        for row in 0..BLADES_PER_SIDE {
            for col in 0..BLADES_PER_SIDE {
                let x =
                    (row as f32 / BLADES_PER_SIDE as f32) * TILE_SIZE + tile_x as f32 * TILE_SIZE;
                let y = 0.0; // sample from texture height map
                let z =
                    (col as f32 / BLADES_PER_SIDE as f32) * TILE_SIZE + tile_z as f32 * TILE_SIZE;
                let hash = grass_hash([tile_x, tile_z], row, col);
                let hash_f32 = hash as f32 / u64::MAX as f32;
                // println!("hash {hash_f32}");
                let roty = hash_f32 * PI * 2.0;
                let rotz = hash_f32 * PI / 8.0;
                // let rotz = PI / 4.0;
                self.instances.vec.push(GrassInstance {
                    pos: vec3(x, y, z),
                    facing: vec2(roty, rotz),
                });
            }
        }
    }

    fn compute(&mut self, ctx: &Context, encoder: &mut wgpu::CommandEncoder) {
        let queue = render::queue(ctx);

        // clear indirect buffer
        // let cleared_indirect = wgpu::util::DrawIndirect {
        //     base_vertex: 0,
        //     vertex_count: 0,
        //     base_instance: 0,
        //     instance_count: 0,
        // };
        // queue.write_buffer(&self.indirect_buffer, 0, cleared_indirect.as_bytes());

        // clear instance count
        queue.write_buffer(&self.instance_count, 0, bytemuck::cast_slice(&[0u32]));

        // run compute
        let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("compute pass"),
            timestamp_writes: None,
        });

        //self.instances.update_buffer(&queue); // TODO create this in compute shader

        // instance
        compute_pass.set_pipeline(&self.instance_compute_pipeline);
        compute_pass.set_bind_group(0, &self.instance_compute_bindgroup, &[]);
        compute_pass.dispatch_workgroups(BLADES_PER_SIDE / 16, BLADES_PER_SIDE / 16, 1);
        // compute_pass.dispatch_workgroups(BLADES_PER_SIDE, BLADES_PER_SIDE, 1);
        // compute_pass.dispatch_workgroups(BLADES_PER_TILE / 256, 1, 1);

        // draw
        compute_pass.set_pipeline(&self.draw_compute_pipeline);
        compute_pass.set_bind_group(0, &self.draw_compute_bindgroup, &[]);
        compute_pass.dispatch_workgroups(1, 1, 1); // TODO increase here
    }

    fn render<'a>(
        &'a self,
        ctx: &Context,
        render_pass: &mut wgpu::RenderPass<'a>,
        camera: &'a render::PerspectiveCamera,
    ) {
        render_pass.set_pipeline(&self.grass_pipeline);
        render_pass.set_vertex_buffer(0, self.instances.buffer.slice(..));
        render_pass.set_bind_group(0, &camera.bind_group, &[]);
        render_pass.draw_indirect(&self.indirect_buffer, 0);
    }

    async fn new(ctx: &Context, camera: &render::PerspectiveCamera) -> Self {
        let device = render::device(ctx);
        let surface_config = render::surface_config(ctx);

        // Buffers
        let instances = render::InstanceBuffer::new_empty(&device, BLADES_PER_TILE as u64);
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

        // Compute 1
        let instance_compute_bindgroup_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("compute bind group layout)"),
                entries: &[
                    // instance buffer
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // instance count
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });
        let instance_compute_bindgroup = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("compute bind group"),
            layout: &instance_compute_bindgroup_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: instances.buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: instance_count.as_entire_binding(),
                },
            ],
        });
        let instance_shader_str =
            filesystem::load_string(ctx, Path::new("grass_compute_instance.wgsl"))
                .await
                .unwrap();
        let instance_compute_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(instance_shader_str.into()),
        });

        let instance_compute_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("draw compute pipeline layout"),
                bind_group_layouts: &[&instance_compute_bindgroup_layout],
                push_constant_ranges: &[],
            });

        let instance_compute_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("draw compute pipeline"),
                layout: Some(&instance_compute_pipeline_layout),
                module: &instance_compute_shader,
                entry_point: "cs_main",
            });

        // Compute 2
        let draw_compute_bindgroup_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("compute bind group layout)"),
                entries: &[
                    // indirect args
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // instance count
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        let draw_compute_bindgroup = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("compute bind group"),
            layout: &draw_compute_bindgroup_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: indirect_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: instance_count.as_entire_binding(),
                },
            ],
        });

        let draw_shader_str = filesystem::load_string(ctx, Path::new("grass_compute_draw.wgsl"))
            .await
            .unwrap();
        let draw_compute_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(draw_shader_str.into()),
        });

        let draw_compute_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("draw compute pipeline layout"),
                bind_group_layouts: &[&draw_compute_bindgroup_layout],
                push_constant_ranges: &[],
            });

        let draw_compute_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("draw compute pipeline"),
                layout: Some(&draw_compute_pipeline_layout),
                module: &draw_compute_shader,
                entry_point: "cs_main",
            });

        // Render pipeline
        let render_shader_str = filesystem::load_string(ctx, Path::new("grass.wgsl"))
            .await
            .unwrap();
        let render_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(render_shader_str.into()),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("grass render pipeline layout"),
                bind_group_layouts: &[&camera.bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("grass render pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &render_shader,
                entry_point: "vs_main",
                // buffers: &[grass_buffer.desc(), instances.desc()],
                buffers: &[instances.desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &render_shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_config.format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });
        Self {
            instances,
            grass_pipeline: render_pipeline,

            indirect_buffer,
            draw_compute_pipeline,
            draw_compute_bindgroup,

            instance_count,
            instance_compute_pipeline,
            instance_compute_bindgroup,
        }
    }
}

fn grass_hash(tile: [i32; 2], i: u32, j: u32) -> u64 {
    let mut hasher = DefaultHasher::new();
    tile.hash(&mut hasher);
    i.hash(&mut hasher);
    j.hash(&mut hasher);
    hasher.finish()
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
            wind: [0.0],
            pad: [0.0],
        }
    }
}

impl InstaceTrait<GrassInstanceGPU> for GrassInstance {
    fn to_gpu(&self) -> GrassInstanceGPU {
        self.to_gpu()
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct GrassInstanceGPU {
    position: [f32; 3],
    hash: [u32; 1],
    facing: [f32; 2],
    wind: [f32; 1],
    pad: [f32; 1],
}

impl GrassInstanceGPU {
    const SIZE: u64 = std::mem::size_of::<Self>() as u64;
    const ATTRIBUTES: [wgpu::VertexAttribute; 5] = wgpu::vertex_attr_array![
        1=>Float32x3,   // pos
        2=>Uint32,      // hash
        3=>Float32x2,   // facing
        4=>Float32x2,   // wind
        5=>Float32x2,   // pad
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

// #[rustfmt::skip]
// const GRASS_VERTICES: &[Vertex] = &[
//     Vertex { position: [-0.05, 0.0, 0.0] },
//     Vertex { position: [ 0.05, 0.0, 0.0] },
//     Vertex { position: [-0.05, 0.3, 0.0] },
//     Vertex { position: [ 0.05, 0.3, 0.0] },
//     Vertex { position: [-0.05, 0.6, 0.0] },
//     Vertex { position: [ 0.05, 0.6, 0.0] },
//     Vertex { position: [-0.05, 0.9, 0.0] },
//     Vertex { position: [ 0.05, 0.9, 0.0] },
//     Vertex { position: [-0.05, 1.2, 0.0] },
//     Vertex { position: [ 0.05, 1.2, 0.0] },
//     Vertex { position: [ 0.00, 1.5, 0.0] },
// ];

// struct Instances {
//     vec: Vec<Instance>,
//     buffer: wgpu::Buffer,
// }
//
// impl Instances {
//     fn new(device: &wgpu::Device, size: u64) -> Self {
//         let vec = Vec::with_capacity(size as usize);
//         let buffer = device.create_buffer(&wgpu::BufferDescriptor {
//             label: Some("instance buffer"),
//             size: GPUInstance::SIZE * size,
//             usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
//             mapped_at_creation: false,
//         });
//         Self { vec, buffer }
//     }
//
//     fn update_buffer(&self, ctx: &mut Context) {
//         let queue = render::queue(ctx);
//         let gpu_vec = self.vec.iter().map(Instance::to_gpu).collect::<Vec<_>>();
//         queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&gpu_vec));
//     }
// }

// let [x, z] = self
//     .camera
//     .pos
//     .xz()
//     .div(TILE_SIZE)
//     .floor()
//     .as_ivec2()
//     .to_array();
// #[rustfmt::skip]
// let active_tiles = [
//     [x - 1, z + 1], [x, z + 1], [x + 1, z + 1],
//     [x - 1, z], [x, z], [x + 1, z],
//     [x - 1, z - 1], [x, z - 1], [x + 1, z - 1],
// ];
//
// println!("current tile {:?}", [x, z]);
// println!("active tiles {:?}", active_tiles);
//
// // update instances
// self.instances.vec.clear();
//
// for [tile_x, tile_z] in active_tiles {
//     for row in 0..BLADES_PER_SIDE {
//         for col in 0..BLADES_PER_SIDE {
//             let x = (row as f32 / BLADES_PER_SIDE as f32) * TILE_SIZE
//                 + tile_x as f32 * TILE_SIZE;
//             let y = 0.0; // sample from texture height map
//             let z = (col as f32 / BLADES_PER_SIDE as f32) * TILE_SIZE
//                 + tile_z as f32 * TILE_SIZE;
//             let hash = grass_hash([tile_x, tile_z], row, col);
//             let hash_f32 = (hash % u64::MAX) as f32 / u64::MAX as f32;
//             let roty = hash_f32 * PI * 2.0;
//             let rotz = hash_f32 * PI / 6.0;
//             // let rotz = PI / 4.0;
//             self.instances.vec.push(GrassInstance {
//                 pos: vec3(x, y, z),
//                 rot: vec2(roty, rotz),
//             });
//         }
//     }
// }

// let current_tile = self.camera.pos.xz().div(TILE_SIZE).floor().as_ivec2();
// for i in 0..BLADES_PER_TILE_SIDE {
//     for j in 0..BLADES_PER_TILE_SIDE {
//         let x = current_tile.x as f32 * TILE_SIZE + i as f32 / TILE_SIZE;
//         let y = 0.0;
//         let z = current_tile.y as f32 * TILE_SIZE + j as f32 / TILE_SIZE;
//         let hash = grass_hash(current_tile.to_array(), i, j);
//         let hash_f32 = (hash % u64::MAX) as f32 / u64::MAX as f32;
//         let roty = hash_f32 * PI * 2.0;
//         let rotz = hash_f32 * PI / 4.0;
//         // let rotz = PI / 4.0;
//         self.instances.vec.push(GrassInstance {
//             pos: vec3(x, y, z),
//             rot: vec2(roty, rotz),
//         });
//     }
// }

// for i in 0..BLADES_PER_SIDE {
//     for j in 0..BLADES_PER_SIDE {
//         let x = (i as f32 / BLADES_PER_SIDE as f32) * TILE_SIZE;
//         let y = 0.0;
//         let z = (j as f32 / BLADES_PER_SIDE as f32) * TILE_SIZE;
//         let hash = grass_hash([0, 0], i, j);
//         let hash_f32 = (hash % u64::MAX) as f32 / u64::MAX as f32;
//         let roty = hash_f32 * PI * 2.0;
//         let rotz = hash_f32 * PI / 4.0;
//         // let rotz = PI / 4.0;
//         self.instances.vec.push(GrassInstance {
//             pos: vec3(x, y, z),
//             rot: vec2(roty, rotz),
//         });
//     }
// }
