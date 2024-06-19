use encase::ShaderType;
use gbase::{
    filesystem,
    render::{self, Vertex},
    Callbacks, Context, ContextBuilder, LogLevel,
};
use glam::{Quat, Vec3};

#[pollster::main]
pub async fn main() {
    let (mut ctx, ev) = ContextBuilder::new()
        .log_level(LogLevel::Warn)
        .vsync(false)
        .build()
        .await;
    let app = App::new(&mut ctx).await;
    gbase::run(app, ctx, ev);
}

struct App {
    vertex_buffer: render::VertexBuffer<render::Vertex>,
    pipeline: wgpu::RenderPipeline,

    transform: render::Transform,
    transform_buffer: render::UniformBuffer,
    transform_bindgroup: wgpu::BindGroup,
}

impl App {
    async fn new(ctx: &mut Context) -> Self {
        // Shader
        let shader_str = filesystem::load_string(ctx, "transform.wgsl")
            .await
            .unwrap();
        let shader = render::ShaderBuilder::new().build(ctx, &shader_str);

        // Vertex buffer
        let vertex_buffer = render::VertexBufferBuilder::new(TRIANGLE_VERTICES)
            .usage(wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST)
            .build(ctx);

        // Transform
        let transform = render::Transform::default();
        let transform_buffer = render::UniformBufferBuilder::new()
            .usage(wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM)
            .build(ctx, render::TransformUniform::min_size());
        let (transform_bindgroup_layout, transform_bindgroup) =
            render::BindGroupCombinedBuilder::new()
                .entries(&[render::BindGroupCombinedEntry::new(
                    transform_buffer.buf().as_entire_binding(),
                )
                .uniform()])
                .build(ctx);

        // Pipeline
        let pipeline = render::RenderPipelineBuilder::new(&shader)
            .buffers(&[Vertex::desc()])
            .bind_groups(&[&transform_bindgroup_layout])
            .targets(&[render::RenderPipelineBuilder::default_target(ctx)])
            .build(ctx);

        Self {
            vertex_buffer,

            transform,
            transform_bindgroup,
            transform_buffer,

            pipeline,
        }
    }
}

impl Callbacks for App {
    fn update(&mut self, ctx: &mut Context) -> bool {
        // Transform movement
        let t = gbase::time::time_since_start(ctx);
        self.transform.pos.x = t.sin() * 0.5;
        self.transform.pos.y = t.sin() * 0.5;
        self.transform.rot = Quat::from_rotation_z(t);
        self.transform.scale = Vec3::ONE * t.cos().abs().clamp(0.1, 1.0);

        false
    }

    fn render(&mut self, ctx: &mut Context, screen_view: &wgpu::TextureView) -> bool {
        let mut encoder = render::create_encoder(ctx, None);
        let queue = render::queue(ctx);

        // write to transform
        self.transform_buffer.write(ctx, &self.transform.uniform());

        // render
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
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.set_bind_group(0, &self.transform_bindgroup, &[]);
        render_pass.draw(0..TRIANGLE_VERTICES.len() as u32, 0..1);

        drop(render_pass);
        queue.submit(Some(encoder.finish()));

        false
    }
}

#[rustfmt::skip]
const TRIANGLE_VERTICES: &[Vertex] = &[
    Vertex { position: [-0.5, -0.5, 0.0] },
    Vertex { position: [ 0.5, -0.5, 0.0] },
    Vertex { position: [ 0.0,  0.5, 0.0] },
];
