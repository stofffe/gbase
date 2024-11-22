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

// TODO very sketchy rn
pub struct RenderPassBuilder<'a> {
    label: Option<&'a str>,
    color_attachments: &'a [Option<wgpu::RenderPassColorAttachment<'a>>],
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
            color_attachments: self.color_attachments,
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
            color_attachments: self.color_attachments,
            depth_stencil_attachment: self.depth_stencil_attachment,
            timestamp_writes: self.timestamp_writes,
            occlusion_query_set: self.occlusion_query_set,
        });
        (run_func)(render_pass);
    }

    /// Builds render pass and immediately run ```run-func```
    ///
    /// Creates and submits a new encoder
    pub fn build_run_new_encoder(self, ctx: &Context, run_func: impl FnOnce(wgpu::RenderPass<'_>)) {
        let mut encoder = render::EncoderBuilder::new().build(ctx);
        let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: self.label,
            color_attachments: self.color_attachments,
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
    pub fn color_attachments(
        mut self,
        value: &'a [Option<wgpu::RenderPassColorAttachment<'a>>],
    ) -> Self {
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
    pub fn timestamp_writes(mut self, value: wgpu::RenderPassTimestampWrites<'a>) -> Self {
        self.timestamp_writes = Some(value);
        self
    }
    pub fn occlusion_query_set(mut self, value: &'a wgpu::QuerySet) -> Self {
        self.occlusion_query_set = Some(value);
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
    pub fn build_run_no_encoder(self, ctx: &Context, run_func: impl FnOnce(wgpu::ComputePass<'_>)) {
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
    pub fn timestamp_writes(mut self, value: wgpu::ComputePassTimestampWrites<'a>) -> Self {
        self.timestamp_writes = Some(value);
        self
    }
}
