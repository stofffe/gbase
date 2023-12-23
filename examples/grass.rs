use gbase::{
    filesystem, input,
    render::{self, Vertex, VertexColor},
    Callbacks, Context, ContextBuilder, LogLevel,
};
use glam::{ivec2, vec2, vec3, Quat, Vec2, Vec3, Vec3Swizzles};
use std::{
    collections::hash_map::DefaultHasher,
    f32::consts::PI,
    hash::{Hash, Hasher},
    ops::Div,
    path::Path,
};
use wgpu::util::DeviceExt;
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

const TILE_SIZE: f32 = 3.0;
const BLADES_PER_TILE_SIDE: u32 = 20;
const BLADES_PER_TILE: u32 = BLADES_PER_TILE_SIDE * BLADES_PER_TILE_SIDE;

const PLANE_SIZE: f32 = 100.0;
const LIGHT_INIT_POS: Vec3 = vec3(10.0, 10.0, 0.0);

struct App {
    grass_buffer: wgpu::Buffer,
    grass_pipeline: wgpu::RenderPipeline,
    plane_buffer: wgpu::Buffer,
    plane_pipeline: wgpu::RenderPipeline,
    plane_transform: render::Transform,
    instances: Instances,
    camera: render::PerspectiveCamera,
    depth_buffer: DepthBuffer,
}

impl App {
    async fn new(ctx: &mut Context) -> Self {
        let device = render::device(ctx);
        let queue = render::queue(ctx);
        let surface_config = render::surface_config(ctx);

        // Shader
        let shader_str = filesystem::load_string(ctx, Path::new("grass.wgsl"))
            .await
            .unwrap();
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(shader_str.into()),
        });

        // Camera
        let camera = render::PerspectiveCamera::new(&device)
            .pos(vec3(0.0, 1.0, -1.0))
            .pitch(PI / 4.0);

        // Vertex buffer
        let grass_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("grass vertex buffer"),
            contents: bytemuck::cast_slice(GRASS_VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        // Instances
        let instances = Instances::new(&device, BLADES_PER_TILE as u64);

        // Pipeline
        let grass_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("grass render pipeline layout"),
                bind_group_layouts: &[&camera.bind_group_layout],
                push_constant_ranges: &[],
            });

        let grass_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("grass render pipeline"),
            layout: Some(&grass_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc(), GPUInstance::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
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

        // Plane
        let plane_transform = render::Transform::new(&device)
            .rotation(Quat::from_rotation_x(PI / 2.0))
            .scale(vec3(PLANE_SIZE, PLANE_SIZE, 1.0));

        let shader_bytes = filesystem::load_bytes(ctx, Path::new("shader.wgsl"))
            .await
            .unwrap();
        let shader_str = String::from_utf8(shader_bytes).unwrap();
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(shader_str.into()),
        });
        let plane_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("plane vertex buffer"),
            contents: bytemuck::cast_slice(CENTERED_QUAD_VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
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
                buffers: &[VertexColor::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
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

        let depth_buffer = DepthBuffer::new(ctx);

        Self {
            grass_buffer,
            grass_pipeline,
            camera,
            plane_buffer,
            plane_pipeline,
            plane_transform,
            instances,
            depth_buffer,
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
        self.camera.update_buffer(ctx);
        self.plane_transform.update_buffer(ctx);
        self.instances.update_buffer(ctx);
        // update instance buffer?

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

        render_pass.set_pipeline(&self.plane_pipeline);
        render_pass.set_vertex_buffer(0, self.plane_buffer.slice(..));
        render_pass.set_bind_group(0, &self.camera.bind_group, &[]);
        render_pass.set_bind_group(1, &self.plane_transform.bind_group, &[]);
        render_pass.draw(0..CENTERED_QUAD_VERTICES.len() as u32, 0..1);

        render_pass.set_pipeline(&self.grass_pipeline);
        render_pass.set_vertex_buffer(0, self.grass_buffer.slice(..));
        render_pass.set_vertex_buffer(1, self.instances.buffer.slice(..));
        render_pass.set_bind_group(0, &self.camera.bind_group, &[]);
        render_pass.draw(
            0..GRASS_VERTICES.len() as u32,
            0..self.instances.vec.len() as u32,
        );

        drop(render_pass);

        false
    }

    fn update(&mut self, ctx: &mut Context) -> bool {
        let current_tile = self.camera.pos.xz().div(TILE_SIZE).floor().as_ivec2();
        // println!("pos: {}, tile {}", self.camera.pos, current_tile);
        self.plane_transform.pos.x = self.camera.pos.x;
        self.plane_transform.pos.z = self.camera.pos.z;

        // update instances
        self.instances.vec.clear();
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
        //         self.instances.vec.push(Instance {
        //             pos: vec3(x, y, z),
        //             rot: vec2(roty, rotz),
        //         });
        //     }
        // }
        for i in 0..BLADES_PER_TILE_SIDE {
            for j in 0..BLADES_PER_TILE_SIDE {
                let x = i as f32 / TILE_SIZE;
                let y = 0.0;
                let z = j as f32 / TILE_SIZE;
                let hash = grass_hash([0, 0], i, j);
                let hash_f32 = (hash % u64::MAX) as f32 / u64::MAX as f32;
                let roty = hash_f32 * PI * 2.0;
                let rotz = hash_f32 * PI / 4.0;
                // let rotz = PI / 4.0;
                self.instances.vec.push(Instance {
                    pos: vec3(x, y, z),
                    rot: vec2(roty, rotz),
                });
            }
        }

        self.camera_movement(ctx);
        // log::info!("{}", gbase::time::fps(ctx));
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

fn grass_hash(tile: [i32; 2], i: u32, j: u32) -> u64 {
    let mut hasher = DefaultHasher::new();
    tile.hash(&mut hasher);
    i.hash(&mut hasher);
    j.hash(&mut hasher);
    hasher.finish()
}

#[rustfmt::skip]
const GRASS_VERTICES: &[Vertex] = &[
    Vertex { position: [-0.05, 0.0, 0.0]  },
    Vertex { position: [ 0.05, 0.0, 0.0]  },
    Vertex { position: [-0.05, 0.3, 0.0]  },
    Vertex { position: [ 0.05, 0.3, 0.0]  },
    Vertex { position: [-0.05, 0.6, 0.0]  },
    Vertex { position: [ 0.05, 0.6, 0.0]  },
    Vertex { position: [-0.05, 0.9, 0.0]  },
    Vertex { position: [ 0.05, 0.9, 0.0]  },
    Vertex { position: [-0.05, 1.2, 0.0]  },
    Vertex { position: [ 0.05, 1.2, 0.0]  },
    Vertex { position: [ 0.00, 1.5, 0.0]  },
];

#[rustfmt::skip]
const CENTERED_QUAD_VERTICES: &[VertexColor] = &[
    // VertexColor { position: [ 0.0,  0.0, 0.0], color: [0.7, 0.5, 0.2] }, // bottom left
    // VertexColor { position: [ 1.0,  0.0, 0.0], color: [0.7, 0.5, 0.2] }, // bottom right
    // VertexColor { position: [ 1.0,  1.0, 0.0], color: [0.7, 0.5, 0.2] }, // top right
    //
    // VertexColor { position: [ 0.0,  0.0, 0.0], color: [0.7, 0.5, 0.2] }, // bottom left
    // VertexColor { position: [ 1.0,  1.0, 0.0], color: [0.7, 0.5, 0.2] }, // top right
    // VertexColor { position: [ 0.0,  1.0, 0.0], color: [0.7, 0.5, 0.2] }, // top left
    VertexColor { position: [-0.5, -0.5, 0.0], color: [0.7, 0.5, 0.2] }, // bottom left
    VertexColor { position: [ 0.5, -0.5, 0.0], color: [0.7, 0.5, 0.2] }, // bottom right
    VertexColor { position: [ 0.5,  0.5, 0.0], color: [0.7, 0.5, 0.2] }, // top right

    VertexColor { position: [-0.5, -0.5, 0.0], color: [0.7, 0.5, 0.2] }, // bottom left
    VertexColor { position: [ 0.5,  0.5, 0.0], color: [0.7, 0.5, 0.2] }, // top right
    VertexColor { position: [-0.5,  0.5, 0.0], color: [0.7, 0.5, 0.2] }, // top left

];

struct DepthBuffer {
    // TODO bindgroup
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    sampler: wgpu::Sampler,
}

impl DepthBuffer {
    fn new(ctx: &Context) -> Self {
        let device = render::device(ctx);
        let surface_conf = render::surface_config(ctx);

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("depth texture"),
            size: wgpu::Extent3d {
                width: surface_conf.width,
                height: surface_conf.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[wgpu::TextureFormat::Depth32Float],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("depth texture view"),
            ..Default::default()
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("depth texture sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            compare: Some(wgpu::CompareFunction::LessEqual),
            lod_min_clamp: 0.0,
            lod_max_clamp: 100.0,
            anisotropy_clamp: 1,
            border_color: None,
        });

        Self {
            texture,
            view,
            sampler,
        }
    }
}

struct Instances {
    vec: Vec<Instance>,
    buffer: wgpu::Buffer,
}

impl Instances {
    fn new(device: &wgpu::Device, size: u64) -> Self {
        let vec = Vec::with_capacity(size as usize);
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("instance buffer"),
            size: GPUInstance::SIZE * size,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Self { vec, buffer }
    }

    fn update_buffer(&self, ctx: &mut Context) {
        let queue = render::queue(ctx);
        let gpu_vec = self.vec.iter().map(Instance::to_gpu).collect::<Vec<_>>();
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&gpu_vec));
    }
}

struct Instance {
    pos: Vec3,
    rot: Vec2,
}

impl Instance {
    fn to_gpu(&self) -> GPUInstance {
        GPUInstance {
            pos: self.pos.to_array(),
            rot: self.rot.to_array(),
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct GPUInstance {
    pos: [f32; 3],
    rot: [f32; 2],
}

impl GPUInstance {
    const SIZE: u64 = std::mem::size_of::<Self>() as u64;
    const ATTRIBUTES: [wgpu::VertexAttribute; 2] = wgpu::vertex_attr_array![
        1=>Float32x3,
        2=>Float32x2,
    ];
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: Self::SIZE,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &Self::ATTRIBUTES,
        }
    }
}
