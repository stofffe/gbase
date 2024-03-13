use crate::{render, Context};

pub struct EncoderBuilder<'a> {
    label: Option<&'a str>,
}

impl<'a> EncoderBuilder<'a> {
    pub fn new() -> Self {
        Self { label: None }
    }

    pub fn build(self, ctx: &Context) -> wgpu::CommandEncoder {
        let device = render::device(ctx);
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: self.label })
    }
}

// // TODO very sketchy rn
// pub struct RenderPassBuilder<'a> {
//     encoder: &'a mut wgpu::CommandEncoder,
//     label: Option<&'a str>,
//     color_attachments: &'a [Option<wgpu::RenderPassColorAttachment<'a>>],
//     depth_stencil_attachment: Option<wgpu::RenderPassDepthStencilAttachment<'a>>,
// }
//
// impl<'a> RenderPassBuilder<'a> {
//     pub fn new(encoder: &'a mut wgpu::CommandEncoder) -> Self {
//         Self {
//             encoder,
//             label: None,
//             color_attachments: &[],
//             depth_stencil_attachment: None,
//         }
//     }
// }
// // pub fn build<A, F>(self, state: &A, conf: F)
// // where
// //     F: for<'b> FnOnce(&'b A, wgpu::RenderPass<'b>),
// // {
// //     let render_pass = self.encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
// //         label: self.label,
// //         color_attachments: self.color_attachments,
// //         depth_stencil_attachment: self.depth_stencil_attachment,
// //         timestamp_writes: None,
// //         occlusion_query_set: None,
// //     });
// //     conf(state, render_pass);
// // }
// //
// // pub fn clear_target(
// //     view: &wgpu::TextureView,
// //     color: wgpu::Color,
// // ) -> wgpu::RenderPassColorAttachment<'_> {
// //     wgpu::RenderPassColorAttachment {
// //         view,
// //         ops: wgpu::Operations {
// //             load: wgpu::LoadOp::Clear(color),
// //             store: wgpu::StoreOp::Store,
// //         },
// //         resolve_target: None,
// //     }
// // }
// //
// // pub fn load_target(view: &wgpu::TextureView) -> wgpu::RenderPassColorAttachment<'_> {
// //     wgpu::RenderPassColorAttachment {
// //         view,
// //         ops: wgpu::Operations {
// //             load: wgpu::LoadOp::Load,
// //             store: wgpu::StoreOp::Store,
// //         },
// //         resolve_target: None,
// //     }
// // }
// // }
//
// impl<'a> RenderPassBuilder<'a> {
//     pub fn label(mut self, value: &'a str) -> Self {
//         self.label = Some(value);
//         self
//     }
//     pub fn color_attachments(
//         mut self,
//         value: &'a [Option<wgpu::RenderPassColorAttachment<'a>>],
//     ) -> Self {
//         self.color_attachments = value;
//         self
//     }
//     pub fn depth_stencil_attachment(
//         mut self,
//         value: Option<wgpu::RenderPassDepthStencilAttachment<'a>>,
//     ) -> Self {
//         self.depth_stencil_attachment = value;
//         self
//     }
// }
