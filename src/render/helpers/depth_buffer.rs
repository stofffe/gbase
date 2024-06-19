use crate::{render, Context};

///
/// Depth buffer
///

pub struct DepthBuffer {
    texture: super::Texture,
}

impl DepthBuffer {
    pub fn texture(&self) -> &wgpu::Texture {
        self.texture.texture()
    }
    pub fn view(&self) -> &wgpu::TextureView {
        self.texture.view()
    }
}

impl DepthBuffer {
    pub fn new(ctx: &Context) -> Self {
        let texture = Self::create_texture(ctx);
        Self { texture }
    }

    pub fn resize(&mut self, ctx: &Context) {
        self.texture = Self::create_texture(ctx);
    }

    fn create_texture(ctx: &Context) -> super::Texture {
        let surface_config = render::surface_config(ctx);
        super::TextureBuilder::new()
            .usage(wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING)
            .format(wgpu::TextureFormat::Depth32Float)
            .build(ctx, surface_config.width, surface_config.height)
    }

    // TODO not depend on self?
    pub fn depth_stencil_state() -> wgpu::DepthStencilState {
        wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: wgpu::CompareFunction::Less,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        }
    }

    // TODO not working
    pub fn depth_stencil_attachment_load(&self) -> wgpu::RenderPassDepthStencilAttachment<'_> {
        wgpu::RenderPassDepthStencilAttachment {
            view: self.texture.view(),
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Load,
                store: wgpu::StoreOp::Store,
            }),
            stencil_ops: None,
        }
    }
    pub fn depth_stencil_attachment_clear(&self) -> wgpu::RenderPassDepthStencilAttachment<'_> {
        wgpu::RenderPassDepthStencilAttachment {
            view: self.texture.view(),
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Clear(1.0),
                store: wgpu::StoreOp::Store,
            }),
            stencil_ops: None,
        }
    }
    pub fn binding_type(&self) -> wgpu::BindingType {
        wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: false },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        }
    }

    pub fn resource(&self) -> wgpu::BindingResource<'_> {
        wgpu::BindingResource::TextureView(self.view())
    }
}

///
/// Debug depth buffer renderer
///

pub struct DepthBufferRenderer {
    sampler: super::Sampler,
    vertex_buffer: super::VertexBuffer<super::VertexUV>,
    bind_group: wgpu::BindGroup,
    bind_group_layout: wgpu::BindGroupLayout,
    pipeline: wgpu::RenderPipeline,
}

impl DepthBufferRenderer {
    pub fn resize(&mut self, ctx: &Context, depth_buffer: &DepthBuffer) {
        let (bgl, bg) = Self::create_bindgroups(ctx, &self.sampler, depth_buffer);
        self.bind_group_layout = bgl;
        self.bind_group = bg;
    }

    pub fn render(&mut self, ctx: &Context, screen_view: &wgpu::TextureView) {
        let queue = render::queue(ctx);
        let mut encoder = super::EncoderBuilder::new().build(ctx);
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("render pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: screen_view,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
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
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.draw(0..self.vertex_buffer.len(), 0..1);

        drop(render_pass);

        queue.submit(Some(encoder.finish()));
    }

    pub fn new(ctx: &Context, depth_buffer: &DepthBuffer) -> Self {
        let vertex_buffer = super::VertexBufferBuilder::new(FULLSCREEN_VERTICES)
            .usage(wgpu::BufferUsages::VERTEX)
            .build(ctx);

        let sampler = super::SamplerBuilder::new().build(ctx);
        let (bind_group_layout, bind_group) = Self::create_bindgroups(ctx, &sampler, depth_buffer);
        let shader =
            super::ShaderBuilder::new().build(ctx, include_str!("../../../assets/texture.wgsl"));
        let pipeline = super::RenderPipelineBuilder::new(&shader)
            .buffers(&[vertex_buffer.desc()])
            .targets(&[super::RenderPipelineBuilder::default_target(ctx)])
            .bind_groups(&[&bind_group_layout])
            .build(ctx);

        Self {
            sampler,
            bind_group,
            bind_group_layout,
            vertex_buffer,
            pipeline,
        }
    }

    fn create_bindgroups(
        ctx: &Context,
        sampler: &super::Sampler,
        depth_buffer: &DepthBuffer,
    ) -> (wgpu::BindGroupLayout, wgpu::BindGroup) {
        super::BindGroupCombinedBuilder::new()
            .entries(&[
                super::BindGroupCombinedEntry::new(depth_buffer.resource())
                    .visibility(wgpu::ShaderStages::VERTEX_FRAGMENT | wgpu::ShaderStages::COMPUTE)
                    .ty(depth_buffer.binding_type()),
                super::BindGroupCombinedEntry::new(sampler.resource())
                    .visibility(wgpu::ShaderStages::VERTEX_FRAGMENT | wgpu::ShaderStages::COMPUTE)
                    .ty(sampler.binding_nonfiltering()),
            ])
            .build(ctx)
    }
}

#[rustfmt::skip]
const FULLSCREEN_VERTICES: &[super::VertexUV] = &[
    super::VertexUV { position: [-1.0, -1.0, 0.0], uv: [0.0, 1.0] }, // bottom left
    super::VertexUV { position: [ 1.0,  1.0, 0.0], uv: [1.0, 0.0] }, // top right
    super::VertexUV { position: [-1.0,  1.0, 0.0], uv: [0.0, 0.0] }, // top left

    super::VertexUV { position: [-1.0, -1.0, 0.0], uv: [0.0, 1.0] }, // bottom left
    super::VertexUV { position: [ 1.0, -1.0, 0.0], uv: [1.0, 1.0] }, // bottom right
    super::VertexUV { position: [ 1.0,  1.0, 0.0], uv: [1.0, 0.0] }, // top right
];
