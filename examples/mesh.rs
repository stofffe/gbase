fn main() {}

// use gbase::{render, Callbacks, Context, ContextBuilder};
//
// #[pollster::main]
// pub async fn main() {
//     let (ctx, ev) = ContextBuilder::new().build().await;
//     let app = App::new(&ctx).await;
//     gbase::run(app, ctx, ev).await;
// }
//
// struct App {
//     mesh_renderer: MeshRenderer,
// }
//
// impl App {
//     pub async fn new(ctx: &Context) -> Self {
//         let mesh_renderer = MeshRenderer::new(ctx).await;
//         Self { mesh_renderer }
//     }
// }
//
// impl Callbacks for App {
//     fn update(&mut self, _ctx: &mut Context) -> bool {
//         false
//     }
//
//     fn render(&mut self, ctx: &mut Context, screen_view: &wgpu::TextureView) -> bool {
//         self.mesh_renderer.render(ctx, screen_view);
//         false
//     }
// }
//
// struct MeshRenderer {
//     draws: Vec<(Mesh, render::Transform)>,
//
//     depth_buffer: render::DepthBuffer,
//     pipeline: render::RenderPipeline,
// }
//
// impl MeshRenderer {
//     async fn new(ctx: &Context, camera: &render::PerspectiveCamera) -> Self {
//         let depth_buffer = render::DepthBuffer::new(ctx);
//
//         let shader = render::ShaderBuilder::new("shader.wgsl")
//             .buffers(vec![])
//             .default_target(render::surface_config(ctx))
//             .bind_group_layouts(vec![camera.bind_group_layout()])
//             .build(ctx)
//             .await;
//
//         let pipeline = render::RenderPipelineBuilder::new(&shader)
//             .depth_buffer(render::DepthBuffer::depth_stencil_state())
//             .build(ctx);
//
//         let draws = Vec::new();
//
//         Self {
//             draws,
//             depth_buffer,
//             pipeline,
//         }
//     }
//
//     fn draw_mesh(&mut self, ctx: &Context, mesh: Mesh, transform: render::Transform) {
//         self.draws.push((mesh, transform));
//     }
//
//     fn render(&self, ctx: &Context, screen_view: &wgpu::TextureView) {
//         let mut encoder = render::create_encoder(ctx, None);
//
//         let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
//             label: None,
//             color_attachments: &[],
//             depth_stencil_attachment: Some(self.depth_buffer.depth_stencil_attachment_clear()),
//             timestamp_writes: None,
//             occlusion_query_set: None,
//         });
//     }
// }
//
// struct Mesh {
//     verticies: wgpu::Buffer,
//     indices: wgpu::Buffer,
// }
