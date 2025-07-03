use std::{
    ops::{Deref, DerefMut},
    sync::Arc,
};

use crate::{render, Context};

//
// Command encoder
//

pub struct EncoderBuilder<'a> {
    label: Option<&'a str>,
}

impl EncoderBuilder<'_> {
    pub fn new() -> Self {
        Self { label: None }
    }

    pub fn build(self, ctx: &Context) -> wgpu::CommandEncoder {
        let device = render::device(ctx);
        device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: self.label })
    }

    pub fn build_new(self, ctx: &Context) -> Encoder {
        let device = render::device(ctx);
        let encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: self.label });
        Encoder { encoder }
    }
}

pub struct Encoder {
    encoder: wgpu::CommandEncoder,
}

impl Encoder {
    pub fn submit(self, ctx: &Context) {
        let queue = render::queue(ctx);
        queue.submit([self.encoder.finish()]);
    }
}

impl Deref for Encoder {
    type Target = wgpu::CommandEncoder;
    fn deref(&self) -> &Self::Target {
        &self.encoder
    }
}

impl DerefMut for Encoder {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.encoder
    }
}

//
// Render pass
//

#[derive(Debug, Clone)]
pub struct RenderPassColorAttachment<'a> {
    view: &'a wgpu::TextureView,
    resolve_target: Option<&'a wgpu::TextureView>,
    ops: wgpu::Operations<wgpu::Color>,
}

impl<'a> RenderPassColorAttachment<'a> {
    pub fn new(view: &'a wgpu::TextureView) -> Self {
        Self {
            view,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: wgpu::StoreOp::Store,
            },
        }
    }

    pub fn resolve_target(mut self, value: &'a wgpu::TextureView) -> Self {
        self.resolve_target = Some(value);
        self
    }
    pub fn load(mut self) -> Self {
        self.ops.load = wgpu::LoadOp::Load;
        self
    }
    pub fn clear(mut self, value: wgpu::Color) -> Self {
        self.ops.load = wgpu::LoadOp::Clear(value);
        self
    }
    pub fn store(mut self, value: wgpu::StoreOp) -> Self {
        self.ops.store = value;
        self
    }
}

impl<'a> From<RenderPassColorAttachment<'a>> for wgpu::RenderPassColorAttachment<'a> {
    fn from(val: RenderPassColorAttachment<'a>) -> Self {
        wgpu::RenderPassColorAttachment {
            view: val.view,
            resolve_target: val.resolve_target,
            ops: val.ops,
        }
    }
}

// TODO very sketchy rn
pub struct RenderPassBuilder<'a> {
    label: Option<&'a str>,
    color_attachments: &'a [Option<RenderPassColorAttachment<'a>>],
    depth_stencil_attachment: Option<wgpu::RenderPassDepthStencilAttachment<'a>>,
    timestamp_writes: Option<wgpu::RenderPassTimestampWrites<'a>>,
    occlusion_query_set: Option<&'a wgpu::QuerySet>,
}

impl<'a> RenderPassBuilder<'a> {
    pub fn new() -> Self {
        Self {
            label: None,
            color_attachments: &[],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        }
    }

    /// Build render pass
    pub fn build(self, encoder: &'a mut wgpu::CommandEncoder) -> wgpu::RenderPass<'a> {
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: self.label,
            color_attachments: &self
                .color_attachments
                .iter()
                .map(|att| att.clone().map(RenderPassColorAttachment::into))
                .collect::<Vec<_>>(),
            depth_stencil_attachment: self.depth_stencil_attachment,
            timestamp_writes: self.timestamp_writes,
            occlusion_query_set: self.occlusion_query_set,
        })
    }

    /// Build render pass and immediately run ```run_func```
    pub fn build_run(
        self,
        encoder: &'a mut wgpu::CommandEncoder,
        run_func: impl FnOnce(wgpu::RenderPass<'a>),
    ) {
        let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: self.label,
            color_attachments: &self
                .color_attachments
                .iter()
                .map(|att| att.clone().map(RenderPassColorAttachment::into))
                .collect::<Vec<_>>(),
            depth_stencil_attachment: self.depth_stencil_attachment,
            timestamp_writes: self.timestamp_writes,
            occlusion_query_set: self.occlusion_query_set,
        });
        (run_func)(render_pass);
    }

    /// Builds render pass and immediately run ```run-func```
    ///
    /// Creates and submits a new encoder
    pub fn build_run_submit(self, ctx: &Context, run_func: impl FnOnce(wgpu::RenderPass<'_>)) {
        let mut encoder = render::EncoderBuilder::new().build(ctx);
        let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: self.label,
            color_attachments: &self
                .color_attachments
                .iter()
                .map(|att| att.clone().map(RenderPassColorAttachment::into))
                .collect::<Vec<_>>(),
            depth_stencil_attachment: self.depth_stencil_attachment,
            timestamp_writes: self.timestamp_writes,
            occlusion_query_set: self.occlusion_query_set,
        });
        (run_func)(render_pass);
        render::queue(ctx).submit(Some(encoder.finish()));
    }
}

impl<'a> RenderPassBuilder<'a> {
    pub fn label(mut self, value: &'a str) -> Self {
        self.label = Some(value);
        self
    }
    pub fn color_attachments(mut self, value: &'a [Option<RenderPassColorAttachment<'a>>]) -> Self {
        self.color_attachments = value;
        self
    }
    pub fn depth_stencil_attachment(
        mut self,
        value: wgpu::RenderPassDepthStencilAttachment<'a>,
    ) -> Self {
        self.depth_stencil_attachment = Some(value);
        self
    }
    // pub fn timestamp_writes(mut self, value: Option<wgpu::RenderPassTimestampWrites<'a>>) -> Self {
    //     self.timestamp_writes = value;
    //     self
    // }
    pub fn occlusion_query_set(mut self, value: &'a wgpu::QuerySet) -> Self {
        self.occlusion_query_set = Some(value);
        self
    }

    // TODO: send label and do this in build instead?
    pub fn trace_gpu(mut self, ctx: &'a mut Context, label: &'static str) -> Self {
        self.timestamp_writes = ctx.render.gpu_profiler.profile_render_pass(label);
        self
    }
}

pub struct ComputePassBuilder<'a> {
    label: Option<&'a str>,
    timestamp_writes: Option<wgpu::ComputePassTimestampWrites<'a>>,
}

impl<'a> ComputePassBuilder<'a> {
    pub fn new() -> Self {
        Self {
            label: None,
            timestamp_writes: None,
        }
    }

    /// Builds compute pass
    pub fn build(self, encoder: &'a mut wgpu::CommandEncoder) -> wgpu::ComputePass<'a> {
        encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: self.label,
            timestamp_writes: self.timestamp_writes,
        })
    }

    /// Builds compute pass and immediately run ```run-func```
    pub fn build_run(
        self,
        encoder: &'a mut wgpu::CommandEncoder,
        run_func: impl FnOnce(wgpu::ComputePass<'_>),
    ) {
        let compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: self.label,
            timestamp_writes: self.timestamp_writes,
        });
        (run_func)(compute_pass)
    }

    /// Builds compute pass and immediately run ```run-func```
    ///
    /// Creates and submits a new encoder
    pub fn build_run_submit(self, ctx: &Context, run_func: impl FnOnce(wgpu::ComputePass<'_>)) {
        let mut encoder = render::EncoderBuilder::new().build(ctx);
        let compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: self.label,
            timestamp_writes: self.timestamp_writes,
        });
        (run_func)(compute_pass);
        render::queue(ctx).submit(Some(encoder.finish()));
    }
}

impl<'a> ComputePassBuilder<'a> {
    pub fn label(mut self, value: &'a str) -> Self {
        self.label = Some(value);
        self
    }
    // pub fn timestamp_writes(mut self, value: Option<wgpu::ComputePassTimestampWrites<'a>>) -> Self {
    //     self.timestamp_writes = value;
    //     self
    // }
    pub fn trace_gpu(mut self, ctx: &'a mut Context, label: &'static str) -> Self {
        self.timestamp_writes = ctx.render.gpu_profiler.profile_compute_pass(label);
        self
    }
}
