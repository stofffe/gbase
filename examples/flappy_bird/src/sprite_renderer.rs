use gbase::glam::{Vec2, Vec4};
use gbase::winit::dpi::PhysicalSize;
use gbase::{
    filesystem,
    glam::{vec4, Vec4Swizzles},
    render::{self, VertexTrait},
    wgpu, Context,
};
use gbase_utils::Transform2D;

pub struct SpriteRenderer {
    vertices: Vec<VertexSprite>,
    indices: Vec<u32>,

    vertex_buffer: render::VertexBuffer<VertexSprite>,
    index_buffer: render::IndexBuffer,

    bindgroup_layout: render::ArcBindGroupLayout,
    pipeline: render::ArcRenderPipeline,
    // TODO: pass from user code?
    sampler: render::ArcSampler,

    stencil_buffer: render::FrameBuffer,
    stencil_bindgroup_layout: render::ArcBindGroupLayout,
    stencil_pipeline: render::ArcRenderPipeline,
}

impl SpriteRenderer {
    pub fn new(ctx: &mut Context, max_sprites: u64, output_format: wgpu::TextureFormat) -> Self {
        let vertices = Vec::new();
        let indices = Vec::new();

        let vertex_buffer =
            render::VertexBufferBuilder::new(render::VertexBufferSource::Empty(max_sprites * 4))
                .build(ctx);
        let index_buffer =
            render::IndexBufferBuilder::new(render::IndexBufferSource::Empty(max_sprites * 6))
                .build(ctx);

        let shader = render::ShaderBuilder::new(
            filesystem::load_s!("shaders/sprite_renderer.wgsl")
                .expect("could not load sprite renderer shader"),
        )
        .build(ctx);

        let bindgroup_layout = render::BindGroupLayoutBuilder::new()
            .entries(vec![
                // camera
                render::BindGroupLayoutEntry::new().uniform().vertex(),
                // atlas
                render::BindGroupLayoutEntry::new()
                    .texture_float_filterable()
                    .fragment(),
                // sampler
                render::BindGroupLayoutEntry::new()
                    .sampler_filtering()
                    .fragment(),
            ])
            .build(ctx);

        let pipeline_layout = render::PipelineLayoutBuilder::new()
            .bind_groups(vec![bindgroup_layout.clone()])
            .build(ctx);
        let stencil_buffer = render::FrameBufferBuilder::new()
            .screen_size(ctx)
            .format(wgpu::TextureFormat::Depth24PlusStencil8)
            .usage(wgpu::TextureUsages::RENDER_ATTACHMENT)
            .build(ctx);

        let pipeline = render::RenderPipelineBuilder::new(shader, pipeline_layout)
            .label("sprite renderer")
            .buffers(vec![vertex_buffer.desc()])
            .depth_stencil(wgpu::DepthStencilState {
                format: stencil_buffer.format(),
                depth_write_enabled: false,
                depth_compare: wgpu::CompareFunction::Always,
                stencil: wgpu::StencilState {
                    front: wgpu::StencilFaceState {
                        compare: wgpu::CompareFunction::Equal,
                        fail_op: wgpu::StencilOperation::Keep,
                        pass_op: wgpu::StencilOperation::Keep,
                        depth_fail_op: wgpu::StencilOperation::Keep,
                    },
                    back: wgpu::StencilFaceState {
                        compare: wgpu::CompareFunction::Equal,
                        fail_op: wgpu::StencilOperation::Keep,
                        pass_op: wgpu::StencilOperation::Keep,
                        depth_fail_op: wgpu::StencilOperation::Keep,
                    },
                    read_mask: 0xFF,
                    write_mask: 0x00,
                },
                bias: wgpu::DepthBiasState::default(),
            })
            .single_target(
                render::ColorTargetState::new()
                    .format(output_format)
                    .blend(wgpu::BlendState::ALPHA_BLENDING),
            )
            .build(ctx);

        let sampler = render::SamplerBuilder::new().build(ctx);

        // Stencil

        let stencil_shader =
            render::ShaderBuilder::new(include_str!("../assets/shaders/stencil.wgsl")).build(ctx);

        let stencil_bindgroup_layout = render::BindGroupLayoutBuilder::new()
            .entries(vec![render::BindGroupLayoutEntry::new().uniform().vertex()])
            .build(ctx);

        let stencil_pipeline_layout = render::PipelineLayoutBuilder::new()
            .bind_groups(vec![stencil_bindgroup_layout.clone()])
            .build(ctx);
        let stencil_pipeline =
            render::RenderPipelineBuilder::new(stencil_shader, stencil_pipeline_layout)
                .buffers(vec![VertexSprite::desc()])
                .depth_stencil(wgpu::DepthStencilState {
                    format: stencil_buffer.format(),
                    depth_write_enabled: false,
                    depth_compare: wgpu::CompareFunction::Always,
                    stencil: wgpu::StencilState {
                        front: wgpu::StencilFaceState {
                            compare: wgpu::CompareFunction::Always,
                            fail_op: wgpu::StencilOperation::Replace,
                            pass_op: wgpu::StencilOperation::Replace,
                            depth_fail_op: wgpu::StencilOperation::Replace,
                        },
                        back: wgpu::StencilFaceState {
                            compare: wgpu::CompareFunction::Always,
                            fail_op: wgpu::StencilOperation::Replace,
                            pass_op: wgpu::StencilOperation::Replace,
                            depth_fail_op: wgpu::StencilOperation::Replace,
                        },
                        read_mask: 0x00,
                        write_mask: 0xFF,
                    },
                    bias: wgpu::DepthBiasState::default(),
                })
                .build(ctx);

        Self {
            vertices,
            indices,
            vertex_buffer,
            index_buffer,
            bindgroup_layout,
            pipeline,
            sampler,

            stencil_buffer,
            stencil_bindgroup_layout,
            stencil_pipeline,
        }
    }

    /// renders everything to the internal stencil buffer
    pub fn render_stencil(
        &mut self,
        ctx: &mut Context,
        camera: &render::UniformBuffer<gbase_utils::CameraUniform>,
        stencil_reference: u32,
    ) {
        // update buffers
        self.vertex_buffer.write(ctx, &self.vertices);
        self.index_buffer.write(ctx, &self.indices);

        // create bindgroup
        let stencil_bindgroup =
            render::BindGroupBuilder::new(self.stencil_bindgroup_layout.clone())
                .entries(vec![
                    // camera
                    render::BindGroupEntry::Buffer(camera.buffer()),
                ])
                .build(ctx);

        render::RenderPassBuilder::new()
            .depth_stencil_attachment(wgpu::RenderPassDepthStencilAttachment {
                view: self.stencil_buffer.view_ref(),
                depth_ops: None,
                stencil_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(0),
                    store: wgpu::StoreOp::Store,
                }),
            })
            .build_run_submit(ctx, |mut pass| {
                pass.set_stencil_reference(stencil_reference);
                pass.set_pipeline(&self.stencil_pipeline);
                pass.set_bind_group(0, stencil_bindgroup.as_ref(), &[]);
                pass.set_vertex_buffer(0, self.vertex_buffer.slice(..)); // TODO: use len of vertices?
                pass.set_index_buffer(self.index_buffer.slice(..), self.index_buffer.format());

                pass.draw_indexed(0..self.indices.len() as u32, 0, 0..1);
            });

        self.vertices.clear();
        self.indices.clear();
    }

    pub fn render(
        &mut self,
        ctx: &mut Context,
        output_view: &wgpu::TextureView,
        camera: &render::UniformBuffer<gbase_utils::CameraUniform>,
        atlas: &render::TextureWithView,
        stencil_reference: u32,
    ) {
        // update buffers
        self.vertex_buffer.write(ctx, &self.vertices);
        self.index_buffer.write(ctx, &self.indices);

        // create bindgroup
        let mut encoder = render::EncoderBuilder::new().build(ctx);
        let bindgroup = render::BindGroupBuilder::new(self.bindgroup_layout.clone())
            .entries(vec![
                // camera
                render::BindGroupEntry::Buffer(camera.buffer()),
                // atlas
                render::BindGroupEntry::Texture(atlas.view()),
                // sampler
                render::BindGroupEntry::Sampler(self.sampler.clone()),
            ])
            .build(ctx);

        render::RenderPassBuilder::new()
            .color_attachments(&[Some(
                render::RenderPassColorAttachment::new(output_view).load(),
            )])
            .depth_stencil_attachment(wgpu::RenderPassDepthStencilAttachment {
                view: self.stencil_buffer.view_ref(),
                depth_ops: None,
                stencil_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Discard,
                }),
            })
            .build_run(&mut encoder, |mut pass| {
                pass.set_stencil_reference(stencil_reference);
                pass.set_pipeline(&self.pipeline);
                pass.set_bind_group(0, bindgroup.as_ref(), &[]);
                pass.set_vertex_buffer(0, self.vertex_buffer.slice(..)); // TODO: use len of vertices?
                pass.set_index_buffer(self.index_buffer.slice(..), self.index_buffer.format());

                pass.draw_indexed(0..self.indices.len() as u32, 0, 0..1);
            });

        render::queue(ctx).submit(Some(encoder.finish()));

        self.vertices.clear();
        self.indices.clear();
    }

    pub fn resize(&mut self, ctx: &mut Context, new_size: PhysicalSize<u32>) {
        self.stencil_buffer
            .resize(ctx, new_size.width, new_size.height);
    }

    pub fn draw_sprite(&mut self, transform: &Transform2D, atlas_pos: Vec2, atlas_size: Vec2) {
        self.draw_sprite_tint(transform, atlas_pos, atlas_size, gbase_utils::WHITE);
    }

    pub fn draw_sprite_tint(
        &mut self,
        transform: &Transform2D,
        atlas_pos: Vec2,
        atlas_size: Vec2,
        tint: Vec4,
    ) {
        let (ux, uy) = (atlas_pos.x, atlas_pos.y);
        let (uw, uh) = (atlas_size.x, atlas_size.y);
        let color = tint.to_array();

        let t = transform.matrix();
        let offset = self.vertices.len() as u32; // save before pushing verts
        self.vertices.push(VertexSprite {
            position: (t * vec4(-0.5, 0.5, 0.0, 1.0)).xyz().to_array(),
            uv: [ux, uy],
            color,
            uses_texture: 1.0,
        }); // tl
        self.vertices.push(VertexSprite {
            position: (t * vec4(-0.5, -0.5, 0.0, 1.0)).xyz().to_array(),
            uv: [ux, uy + uh],
            color,
            uses_texture: 1.0,
        }); // bl
        self.vertices.push(VertexSprite {
            position: (t * vec4(0.5, 0.5, 0.0, 1.0)).xyz().to_array(),
            uv: [ux + uw, uy],
            color,
            uses_texture: 1.0,
        }); // tr
        self.vertices.push(VertexSprite {
            position: (t * vec4(0.5, -0.5, 0.0, 1.0)).xyz().to_array(),
            uv: [ux + uw, uy + uh],
            color,
            uses_texture: 1.0,
        }); // br
        self.indices.push(offset); // tl
        self.indices.push(offset + 1); // bl
        self.indices.push(offset + 2); // tr
        self.indices.push(offset + 2); // tr
        self.indices.push(offset + 1); // bl
        self.indices.push(offset + 3); // br
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct VertexSprite {
    pub position: [f32; 3],
    pub color: [f32; 4],
    pub uv: [f32; 2],
    pub uses_texture: f32,
}

impl VertexSprite {
    const ATTRIBUTES: &'static [wgpu::VertexAttribute] = &wgpu::vertex_attr_array![
        0=>Float32x3,  // pos
        1=>Float32x4,  // color
        2=>Float32x2,  // uv
        3=>Float32,    // uses texture
    ];
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: Self::ATTRIBUTES,
        }
    }
}

impl VertexTrait for VertexSprite {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        Self::desc()
    }
}
