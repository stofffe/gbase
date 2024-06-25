// use crate::{filesystem, render, Context};
//
// use super::VertexUV;
//
// pub struct TextureRenderer {
//     pipeline: wgpu::RenderPipeline,
//     vertex_buffer: render::VertexBuffer<VertexUV>,
//     texture: render::Texture,
//     bindgroup: wgpu::BindGroup,
// }
//
// impl TextureRenderer {
//     pub async fn new(ctx: &Context) -> Self {
//         let shader_src = filesystem::load_string(ctx, "texture_renderer.wgsl")
//             .await
//             .unwrap();
//         let shader = render::ShaderBuilder::new().source(shader_src).build(ctx);
//         let vertex_buffer = render::VertexBufferBuilder::new(QUAD_VERTICES).build(ctx);
//         let (texture, sampler) = Self::texture(ctx);
//         let (bindgroup_layout, bindgroup) = Self::bind_group(ctx, &texture, &sampler);
//         let pipeline = render::RenderPipelineBuilder::new(&shader)
//             .targets(&[render::RenderPipelineBuilder::default_target(ctx)])
//             .buffers(&[vertex_buffer.desc()])
//             .bind_groups(&[&bindgroup_layout])
//             .build(ctx);
//
//         Self {
//             vertex_buffer,
//             pipeline,
//             bindgroup,
//             texture,
//         }
//     }
//
//     fn texture(ctx: &Context) -> (render::Texture, render::Sampler) {
//         let window_size = render::window(ctx).inner_size();
//         let sampler = render::SamplerBuilder::new().build(ctx);
//         let texture = render::TextureBuilder::new()
//             .usage(wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST)
//             .build(ctx, window_size.width, window_size.height);
//         (texture, sampler)
//     }
//
//     fn bind_group(
//         ctx: &Context,
//         texture: &render::Texture,
//         sampler: &render::Sampler,
//     ) -> (wgpu::BindGroupLayout, wgpu::BindGroup) {
//         let (bindgroup_layout, bindgroup) = render::BindGroupCombinedBuilder::new()
//             .entries(&[
//                 render::BindGroupCombinedEntry::new(texture.resource())
//                     .ty(texture.binding_type())
//                     .visibility(wgpu::ShaderStages::FRAGMENT),
//                 render::BindGroupCombinedEntry::new(sampler.resource())
//                     .ty(sampler.binding_filtering())
//                     .visibility(wgpu::ShaderStages::FRAGMENT),
//             ])
//             .build(ctx);
//         (bindgroup_layout, bindgroup)
//     }
//
//     pub fn resize(&mut self, ctx: &Context) {
//         let (texture, sampler) = Self::texture(ctx);
//         let (_, bindgroup) = Self::bind_group(ctx, &texture, &sampler);
//         self.texture = texture;
//         self.bindgroup = bindgroup;
//     }
//
//     pub fn render(
//         &self,
//         _ctx: &Context,
//         screen_view: &wgpu::TextureView,
//         encoder: &mut wgpu::CommandEncoder,
//         texture: &wgpu::Texture,
//     ) {
//         // let queue = render::queue(ctx);
//         // let mut encoder = render::EncoderBuilder::new().build(ctx);
//         // Copy input texture to bind group texture
//         encoder.copy_texture_to_texture(
//             wgpu::ImageCopyTextureBase {
//                 texture,
//                 mip_level: 0,
//                 origin: wgpu::Origin3d::ZERO,
//                 aspect: wgpu::TextureAspect::All,
//             },
//             wgpu::ImageCopyTextureBase {
//                 texture: self.texture.texture(),
//                 mip_level: 0,
//                 origin: wgpu::Origin3d::ZERO,
//                 aspect: wgpu::TextureAspect::All,
//             },
//             texture.size(),
//         );
//
//         // Render
//
//         let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
//             label: None,
//             color_attachments: &[Some(wgpu::RenderPassColorAttachment {
//                 view: screen_view,
//                 ops: wgpu::Operations {
//                     load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
//                     store: wgpu::StoreOp::Store,
//                 },
//                 resolve_target: None,
//             })],
//             depth_stencil_attachment: None,
//             timestamp_writes: None,
//             occlusion_query_set: None,
//         });
//
//         render_pass.set_pipeline(&self.pipeline);
//         render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
//         render_pass.set_bind_group(0, &self.bindgroup, &[]);
//         render_pass.draw(0..self.vertex_buffer.len(), 0..1);
//
//         // drop(render_pass);
//         // queue.submit(Some(encoder.finish()));
//     }
// }
//
// #[rustfmt::skip]
// const QUAD_VERTICES: &[render::VertexUV] = &[
//     render::VertexUV { position: [-1.0, -1.0, 0.0], uv: [0.0, 1.0] }, // bottom left
//     render::VertexUV { position: [ 1.0,  1.0, 0.0], uv: [1.0, 0.0] }, // top right
//     render::VertexUV { position: [-1.0,  1.0, 0.0], uv: [0.0, 0.0] }, // top left
//
//     render::VertexUV { position: [-1.0, -1.0, 0.0], uv: [0.0, 1.0] }, // bottom left
//     render::VertexUV { position: [ 1.0, -1.0, 0.0], uv: [1.0, 1.0] }, // bottom right
//     render::VertexUV { position: [ 1.0,  1.0, 0.0], uv: [1.0, 0.0] }, // top right
// ];
