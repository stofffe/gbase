use encase::ShaderType;
use gbase::{
    filesystem, input,
    render::{self, Vertex},
    Callbacks, Context, ContextBuilder, LogLevel,
};
use glam::{vec3, Vec3};
use std::path::Path;
use wgpu::util::DeviceExt;
use winit::keyboard::KeyCode;

#[pollster::main]
pub async fn main() {
    let (mut ctx, ev) = ContextBuilder::new()
        .log_level(LogLevel::Info)
        .build()
        .await;
    let app = App::new(&mut ctx).await;
    gbase::run(app, ctx, ev).await;
}

struct App {
    vertex_buffer: wgpu::Buffer,
    pipeline: wgpu::RenderPipeline,
    camera: Camera,
}

struct Camera {
    pos: Vec3,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
    buffer: wgpu::Buffer,
}

#[derive(encase::ShaderType)]
struct CameraUniform {
    pos: Vec3,
}

impl Camera {
    fn new(device: &wgpu::Device) -> Self {
        let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("camera buffer"),
            size: u64::from(CameraUniform::min_size()),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let camera_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("camera bg layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }],
            });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("camera bg"),
            layout: &camera_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: camera_buffer.as_entire_binding(),
            }],
        });

        Self {
            pos: vec3(0.0, 0.0, 0.0),
            bind_group_layout: camera_bind_group_layout,
            bind_group: camera_bind_group,
            buffer: camera_buffer,
        }
    }

    fn uniform(&self) -> CameraUniform {
        CameraUniform { pos: self.pos }
    }

    fn update_buffer(&self, queue: &wgpu::Queue) {
        let mut buffer = encase::UniformBuffer::new(Vec::new());
        buffer
            .write(&self.uniform())
            .expect("could not write to camera buffer");
        queue.write_buffer(&self.buffer, 0, &buffer.into_inner());
    }
}

impl App {
    async fn new(ctx: &mut Context) -> Self {
        let device = render::device(ctx);
        let surface_config = render::surface_config(ctx);

        // Shader
        let shader_bytes = filesystem::load_bytes(ctx, Path::new("camera_encase.wgsl"))
            .await
            .unwrap();
        let shader_str = String::from_utf8(shader_bytes).unwrap();
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(shader_str.into()),
        });

        // Vertex buffer
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("vertex buffer"),
            contents: bytemuck::cast_slice(TRIANGLE_VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        // Camera
        let camera = Camera::new(&device);

        // Pipeline
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("render pipeline layout"),
            bind_group_layouts: &[&camera.bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("render pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
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
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        Self {
            vertex_buffer,
            pipeline,
            camera,
        }
    }
}

impl Callbacks for App {
    fn update(&mut self, ctx: &mut Context) -> bool {
        let dt = gbase::time::delta_time(ctx);

        if input::key_pressed(ctx, KeyCode::KeyW) {
            self.camera.pos += vec3(0.0, 1.0, 0.0) * dt;
        }
        if input::key_pressed(ctx, KeyCode::KeyS) {
            self.camera.pos += vec3(0.0, -1.0, 0.0) * dt;
        }
        if input::key_pressed(ctx, KeyCode::KeyA) {
            self.camera.pos += vec3(-1.0, 0.0, 0.0) * dt;
        }
        if input::key_pressed(ctx, KeyCode::KeyD) {
            self.camera.pos += vec3(1.0, 0.0, 0.0) * dt;
        }

        // let fps = gbase::time::fps(ctx);
        // println!("fps {fps}");
        false
    }

    fn render(
        &mut self,
        ctx: &mut Context,
        encoder: &mut wgpu::CommandEncoder,
        screen_view: &wgpu::TextureView,
    ) -> bool {
        let queue = render::queue(ctx);

        // update camera uniform
        self.camera.update_buffer(&queue);

        // render
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("render pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: screen_view,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLUE),
                    store: wgpu::StoreOp::Store,
                },
                resolve_target: None,
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_bind_group(0, &self.camera.bind_group, &[]);
        render_pass.draw(0..TRIANGLE_VERTICES.len() as u32, 0..1);

        drop(render_pass);

        false
    }
}

#[rustfmt::skip]
const TRIANGLE_VERTICES: &[Vertex] = &[
    Vertex { position: [-0.5, -0.5, 0.0]  },
    Vertex { position: [0.5, -0.5, 0.0]   },
    Vertex { position: [0.0, 0.5, 0.0] },
];
