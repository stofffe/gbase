use super::VertexUV;
use crate::{render, Context};

pub struct DepthBuffer {
    pub view: wgpu::TextureView,
}

impl DepthBuffer {
    pub fn new(device: &wgpu::Device, surface_conf: &wgpu::SurfaceConfiguration) -> Self {
        let texture = Self::create_texture(device, surface_conf);
        let view = Self::create_view(&texture);

        Self { view }
    }

    pub fn resize(&mut self, device: &wgpu::Device, surface_conf: &wgpu::SurfaceConfiguration) {
        let texture = Self::create_texture(device, surface_conf);
        self.view = Self::create_view(&texture);
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
    pub fn depth_stencil_attachment(&self) -> wgpu::RenderPassDepthStencilAttachment {
        wgpu::RenderPassDepthStencilAttachment {
            view: &self.view,
            depth_ops: Some(wgpu::Operations {
                load: wgpu::LoadOp::Clear(1.0),
                store: wgpu::StoreOp::Store,
            }),
            stencil_ops: None,
        }
    }

    fn create_texture(
        device: &wgpu::Device,
        surface_conf: &wgpu::SurfaceConfiguration,
    ) -> wgpu::Texture {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some("depth texture"),
            size: wgpu::Extent3d {
                width: surface_conf.width,
                height: surface_conf.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[wgpu::TextureFormat::Depth32Float],
        })
    }

    fn create_view(texture: &wgpu::Texture) -> wgpu::TextureView {
        texture.create_view(&wgpu::TextureViewDescriptor {
            label: Some("depth texture view"),
            ..Default::default()
        })
    }
}

pub struct DepthBufferRenderer {
    sampler: wgpu::Sampler,
    buffer: render::VertexBuffer<VertexUV>,
    bind_group: wgpu::BindGroup,
    bind_group_layout: wgpu::BindGroupLayout,
    pipeline: wgpu::RenderPipeline,
}

impl DepthBufferRenderer {
    pub fn resize(&mut self, device: &wgpu::Device, depth_buffer: &DepthBuffer) {
        self.bind_group = Self::create_bind_group(
            &device,
            &depth_buffer.view,
            &self.sampler,
            &self.bind_group_layout,
        );
    }
    pub fn render(&mut self, encoder: &mut wgpu::CommandEncoder, screen_view: &wgpu::TextureView) {
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
        render_pass.set_vertex_buffer(0, self.buffer.buffer().slice(..));
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.draw(0..self.buffer.len(), 0..1);
    }

    pub fn new(ctx: &Context, depth_buffer: &DepthBuffer) -> Self {
        let device = render::device(ctx);
        let surface_config = render::surface_config(ctx);

        let sampler = Self::create_sampler(&device);
        let bind_group_layout = Self::create_bind_group_layout(&device);
        let bind_group =
            Self::create_bind_group(&device, &depth_buffer.view, &sampler, &bind_group_layout);

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(include_str!("../../../assets/texture.wgsl").into()),
        });
        let buffer = render::VertexBuffer::new(&device, FULLSCREEN_VERTICES);

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("render pipeline layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("render pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[buffer.desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_config.format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleStrip,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        Self {
            sampler,
            bind_group,
            bind_group_layout,
            buffer,
            pipeline,
        }
    }
    fn create_sampler(device: &wgpu::Device) -> wgpu::Sampler {
        device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("depth texture sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            compare: None,
            lod_min_clamp: 0.0,
            lod_max_clamp: 100.0,
            anisotropy_clamp: 1,
            border_color: None,
        })
    }

    fn create_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("depth buffer bind group layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: false },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
                    count: None,
                },
            ],
        })
    }

    fn create_bind_group(
        device: &wgpu::Device,
        view: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
        bind_group_layout: &wgpu::BindGroupLayout,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("depth buffer bind group"),
            layout: bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        })
    }
}

#[rustfmt::skip]
const FULLSCREEN_VERTICES: &[VertexUV] = &[
    VertexUV { position: [-1.0, -1.0, 0.0], uv: [0.0, 1.0] }, // bottom left
    VertexUV { position: [ 1.0,  1.0, 0.0], uv: [1.0, 0.0] }, // top right
    VertexUV { position: [-1.0,  1.0, 0.0], uv: [0.0, 0.0] }, // top left

    VertexUV { position: [-1.0, -1.0, 0.0], uv: [0.0, 1.0] }, // bottom left
    VertexUV { position: [ 1.0, -1.0, 0.0], uv: [1.0, 1.0] }, // bottom right
    VertexUV { position: [ 1.0,  1.0, 0.0], uv: [1.0, 0.0] }, // top right
];

// pub struct DepthBuffer {
//     pub texture: wgpu::Texture,
//     pub view: wgpu::TextureView,
//     pub sampler: wgpu::Sampler,
//     pub bind_group_layout: wgpu::BindGroupLayout,
//     pub bind_group: wgpu::BindGroup,
// }
//
// impl DepthBuffer {
//     pub fn new(device: &wgpu::Device, surface_conf: &wgpu::SurfaceConfiguration) -> Self {
//         let texture = Self::create_texture(device, surface_conf);
//         let view = Self::create_view(&texture);
//         let sampler = Self::create_sampler(device);
//         let bind_group_layout = Self::create_bind_group_layout(device);
//         let bind_group = Self::create_bind_group(device, &view, &sampler, &bind_group_layout);
//
//         Self {
//             texture,
//             view,
//             sampler,
//             bind_group_layout,
//             bind_group,
//         }
//     }
//
//     pub fn resize_to_window(
//         &mut self,
//         device: &wgpu::Device,
//         surface_conf: &wgpu::SurfaceConfiguration,
//     ) {
//         self.texture = Self::create_texture(device, surface_conf);
//         self.view = Self::create_view(&self.texture);
//         self.bind_group =
//             Self::create_bind_group(device, &self.view, &self.sampler, &self.bind_group_layout);
//     }
//
//     fn create_texture(
//         device: &wgpu::Device,
//         surface_conf: &wgpu::SurfaceConfiguration,
//     ) -> wgpu::Texture {
//         device.create_texture(&wgpu::TextureDescriptor {
//             label: Some("depth texture"),
//             size: wgpu::Extent3d {
//                 width: surface_conf.width,
//                 height: surface_conf.height,
//                 depth_or_array_layers: 1,
//             },
//             mip_level_count: 1,
//             sample_count: 1,
//             dimension: wgpu::TextureDimension::D2,
//             format: wgpu::TextureFormat::Depth32Float,
//             usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
//             view_formats: &[wgpu::TextureFormat::Depth32Float],
//         })
//     }
//
//     fn create_view(texture: &wgpu::Texture) -> wgpu::TextureView {
//         texture.create_view(&wgpu::TextureViewDescriptor {
//             label: Some("depth texture view"),
//             ..Default::default()
//         })
//     }
//
//     fn create_sampler(device: &wgpu::Device) -> wgpu::Sampler {
//         device.create_sampler(&wgpu::SamplerDescriptor {
//             label: Some("depth texture sampler"),
//             address_mode_u: wgpu::AddressMode::ClampToEdge,
//             address_mode_v: wgpu::AddressMode::ClampToEdge,
//             address_mode_w: wgpu::AddressMode::ClampToEdge,
//             mag_filter: wgpu::FilterMode::Nearest,
//             min_filter: wgpu::FilterMode::Nearest,
//             mipmap_filter: wgpu::FilterMode::Nearest,
//             compare: None,
//             lod_min_clamp: 0.0,
//             lod_max_clamp: 100.0,
//             anisotropy_clamp: 1,
//             border_color: None,
//         })
//     }
//
//     fn create_bind_group_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
//         device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
//             label: Some("depth buffer bind group layout"),
//             entries: &[
//                 wgpu::BindGroupLayoutEntry {
//                     binding: 0,
//                     visibility: wgpu::ShaderStages::FRAGMENT,
//                     ty: wgpu::BindingType::Texture {
//                         sample_type: wgpu::TextureSampleType::Float { filterable: false },
//                         view_dimension: wgpu::TextureViewDimension::D2,
//                         multisampled: false,
//                     },
//                     count: None,
//                 },
//                 wgpu::BindGroupLayoutEntry {
//                     binding: 1,
//                     visibility: wgpu::ShaderStages::FRAGMENT,
//                     ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering),
//                     count: None,
//                 },
//             ],
//         })
//     }
//
//     fn create_bind_group(
//         device: &wgpu::Device,
//         view: &wgpu::TextureView,
//         sampler: &wgpu::Sampler,
//         bind_group_layout: &wgpu::BindGroupLayout,
//     ) -> wgpu::BindGroup {
//         device.create_bind_group(&wgpu::BindGroupDescriptor {
//             label: Some("depth buffer bind group"),
//             layout: bind_group_layout,
//             entries: &[
//                 wgpu::BindGroupEntry {
//                     binding: 0,
//                     resource: wgpu::BindingResource::TextureView(view),
//                 },
//                 wgpu::BindGroupEntry {
//                     binding: 1,
//                     resource: wgpu::BindingResource::Sampler(sampler),
//                 },
//             ],
//         })
//     }
// }
