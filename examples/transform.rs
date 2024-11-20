use gbase::{
    filesystem,
    render::{self, ArcBindGroup, ArcRenderPipeline, TransformUniform, Vertex},
    Callbacks, Context, ContextBuilder, LogLevel,
};
use glam::{Quat, Vec3};

#[pollster::main]
pub async fn main() {
    let (mut ctx, ev) = ContextBuilder::new()
        .log_level(LogLevel::Info)
        .vsync(false)
        .build()
        .await;
    let app = App::new(&mut ctx).await;
    gbase::run(app, ctx, ev);
}

struct App {
    vertex_buffer: render::VertexBuffer<render::Vertex>,
    pipeline: ArcRenderPipeline,

    transform: render::Transform,
    transform_buffer: render::UniformBuffer<TransformUniform>,
    transform_bindgroup: ArcBindGroup,
}

impl App {
    async fn new(ctx: &mut Context) -> Self {
        // Shader
        let shader_str = filesystem::load_string(ctx, "shaders/transform.wgsl")
            .await
            .unwrap();
        let shader = render::ShaderBuilder::new(shader_str).build(ctx);

        // Vertex buffer
        let vertex_buffer = render::VertexBufferBuilder::new(render::VertexBufferSource::Data(
            TRIANGLE_VERTICES.to_vec(),
        ))
        .usage(wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST)
        .build(ctx);

        // Transform
        let transform = render::Transform::default();
        let transform_buffer =
            render::UniformBufferBuilder::new(render::UniformBufferSource::Empty)
                .usage(wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM)
                .build(ctx);
        let transform_bindgroup_layout = render::BindGroupLayoutBuilder::new()
            .entries(vec![
                // Transform
                render::BindGroupLayoutEntry::new().uniform().vertex(),
            ])
            .build(ctx);
        let transform_bindgroup = render::BindGroupBuilder::new(transform_bindgroup_layout.clone())
            .entries(vec![
                // Transform
                render::BindGroupEntry::Buffer(transform_buffer.buffer()),
            ])
            .build(ctx);

        // Pipeline
        let pipeline_layout = render::PipelineLayoutBuilder::new()
            .bind_groups(vec![transform_bindgroup_layout])
            .build(ctx);
        let pipeline = render::RenderPipelineBuilder::new(shader, pipeline_layout.clone())
            .buffers(vec![Vertex::desc()])
            .targets(vec![render::RenderPipelineBuilder::default_target(ctx)])
            .build_uncached(ctx);

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
