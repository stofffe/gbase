// use crate::{render, Context};
//
// ///
// /// Debug depth buffer renderer
// ///
//
// pub struct DepthBufferRenderer {
//     sampler: render::Sampler,
//     vertex_buffer: render::VertexBuffer<render::VertexUV>,
//     bind_group: wgpu::BindGroup,
//     bind_group_layout: wgpu::BindGroupLayout,
//     pipeline: wgpu::RenderPipeline,
// }
//
// impl DepthBufferRenderer {
//     pub fn resize(&mut self, ctx: &Context, depth_buffer: &render::DepthBuffer) {
//         let (bgl, bg) = Self::create_bindgroups(ctx, &self.sampler, depth_buffer);
//         self.bind_group_layout = bgl;
//         self.bind_group = bg;
//     }
//
//     pub fn render(&mut self, ctx: &Context, screen_view: &wgpu::TextureView) {
//         let queue = render::queue(ctx);
//         let mut encoder = render::EncoderBuilder::new().build(ctx);
//
//         let attachments = &[Some(wgpu::RenderPassColorAttachment {
//             view: screen_view,
//             ops: wgpu::Operations {
//                 load: wgpu::LoadOp::Load,
//                 store: wgpu::StoreOp::Store,
//             },
//             resolve_target: None,
//         })];
//         let mut render_pass = render::RenderPassBuilder::new()
//             .color_attachments(attachments)
//             .build(&mut encoder);
//
//         render_pass.set_pipeline(&self.pipeline);
//         render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
//         render_pass.set_bind_group(0, &self.bind_group, &[]);
//         render_pass.draw(0..self.vertex_buffer.len(), 0..1);
//
//         drop(render_pass);
//
//         queue.submit(Some(encoder.finish()));
//     }
//
//     pub fn new(ctx: &Context, depth_buffer: &render::DepthBuffer) -> Self {
//         let vertex_buffer = render::VertexBufferBuilder::new(FULLSCREEN_VERTICES)
//             .usage(wgpu::BufferUsages::VERTEX)
//             .build(ctx);
//
//         let sampler = render::SamplerBuilder::new().build(ctx);
//         let (bind_group_layout, bind_group) = Self::create_bindgroups(ctx, &sampler, depth_buffer);
//         let shader = render::ShaderBuilder::new()
//             .source(include_str!("../../../assets/texture.wgsl").to_string())
//             .build(ctx);
//         let pipeline = render::RenderPipelineBuilder::new(&shader)
//             .buffers(&[vertex_buffer.desc()])
//             .targets(&[render::RenderPipelineBuilder::default_target(ctx)])
//             .bind_groups(&[&bind_group_layout])
//             .build(ctx);
//
//         Self {
//             sampler,
//             bind_group,
//             bind_group_layout,
//             vertex_buffer,
//             pipeline,
//         }
//     }
//
//     fn create_bindgroups(
//         ctx: &Context,
//         sampler: &render::Sampler,
//         depth_buffer: &render::DepthBuffer,
//     ) -> (wgpu::BindGroupLayout, wgpu::BindGroup) {
//         render::BindGroupCombinedBuilder::new()
//             .entries(&[
//                 render::BindGroupCombinedEntry::new(depth_buffer.framebuffer().resource())
//                     .visibility(wgpu::ShaderStages::VERTEX_FRAGMENT | wgpu::ShaderStages::COMPUTE)
//                     .ty(depth_buffer.framebuffer().binding_nonfilter()),
//                 render::BindGroupCombinedEntry::new(sampler.resource())
//                     .visibility(wgpu::ShaderStages::VERTEX_FRAGMENT | wgpu::ShaderStages::COMPUTE)
//                     .ty(sampler.binding_nonfiltering()),
//             ])
//             .build(ctx)
//     }
// }
//
// #[rustfmt::skip]
// const FULLSCREEN_VERTICES: &[render::VertexUV] = &[
//     render::VertexUV { position: [-1.0, -1.0, 0.0], uv: [0.0, 1.0] }, // bottom left
//     render::VertexUV { position: [ 1.0,  1.0, 0.0], uv: [1.0, 0.0] }, // top right
//     render::VertexUV { position: [-1.0,  1.0, 0.0], uv: [0.0, 0.0] }, // top left
//
//     render::VertexUV { position: [-1.0, -1.0, 0.0], uv: [0.0, 1.0] }, // bottom left
//     render::VertexUV { position: [ 1.0, -1.0, 0.0], uv: [1.0, 1.0] }, // bottom right
//     render::VertexUV { position: [ 1.0,  1.0, 0.0], uv: [1.0, 0.0] }, // top right
// ];
