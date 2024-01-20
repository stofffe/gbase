use crate::filesystem;
use crate::{render, Context};
use glam::{uvec2, vec2, Vec4, vec4};
use glam::{UVec2, Vec2};
use std::collections::HashMap;
use std::path::Path;

use super::VertexTrait;

pub struct TextureAtlas {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    sampler: wgpu::Sampler,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
}

impl TextureAtlas {
    fn write_texture(&mut self, ctx: &Context, origin: UVec2, dimensions: UVec2, bytes: &[u8]) {
        let queue = render::queue(ctx);

        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: origin.x,
                    y: origin.y,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            bytes,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(dimensions.x),
                rows_per_image: Some(dimensions.y),
            },
            wgpu::Extent3d {
                width: dimensions.x,
                height: dimensions.y,
                depth_or_array_layers: 1,
            },
        );
    }
    fn new(ctx: &Context, size: UVec2) -> Self {
        let device = render::device(ctx);

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: size.x,
                height: size.y,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: None,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        Self {
            texture,
            view,
            sampler,
            bind_group_layout,
            bind_group,
        }
    }
}

struct FontAtlas {
    texture_atlas: TextureAtlas,
    info: HashMap<char, LetterInfo>,
    line_height: f32,
}

impl FontAtlas {
    async fn new(ctx: &Context, font_path: &str, supported_chars: &str) -> Self {
        // texture
        let font_bytes = filesystem::load_bytes(ctx, Path::new(font_path))
            .await
            .unwrap();
        let font = fontdue::Font::from_bytes(font_bytes, fontdue::FontSettings::default()).unwrap();

        let chars = supported_chars
            .chars()
            .map(|letter| {
                let (metrics, bitmap) = font.rasterize(letter, 64.0);
                (metrics, bitmap, letter)
            })
            .collect::<Vec<_>>();
        // chars.sort_by(|a, b| a.0.height.partial_cmp(&b.0.height).unwrap());
        let texture_dim = uvec2(1024, 1024);
        let max_height = chars
            .iter()
            .map(|(metrics, _, _)| metrics.height)
            .max()
            .unwrap() as u32;
        let line_height = max_height as f32 / texture_dim.y as f32;

        let mut texture_atlas = TextureAtlas::new(ctx, texture_dim);
        let mut offset = UVec2::ZERO;

        let padding = uvec2(10, 10);

        let mut info = HashMap::<char, LetterInfo>::new();

        for (metrics, bitmap, letter) in chars {
            let dimensions = uvec2(metrics.width as u32, metrics.height as u32);

            if dimensions.x + padding.x > texture_dim.x - offset.x {
                offset.y += max_height + padding.x;
                offset.x = 0;
            }

            info.insert(
                letter,
                LetterInfo {
                    atlas_offset: offset.as_vec2() / texture_dim.as_vec2(),
                    atlas_dimensions: dimensions.as_vec2() / texture_dim.as_vec2(),
                    local_offset: vec2(metrics.xmin as f32, metrics.ymin as f32)
                        / texture_dim.as_vec2(),
                    advance: vec2(metrics.advance_width, metrics.advance_height)
                        / texture_dim.as_vec2(),
                },
            );

            // println!("{:?}", metrics);
            texture_atlas.write_texture(ctx, offset, dimensions, &bitmap);
            offset.x += dimensions.x + padding.x;
        }

        Self {
            texture_atlas,
            info,
            line_height,
        }
    }
}

impl FontAtlas {
    pub fn get_info(&self, letter: char) -> &LetterInfo {
        match self.info.get(&letter) {
            Some(info) => info,
            None => panic!("trying to get unsupported letter \"{}\"", letter), // TODO default
        }
    }
    pub fn texture(&self) -> &wgpu::Texture {
        &self.texture_atlas.texture
    }
    pub fn view(&self) -> &wgpu::TextureView {
        &self.texture_atlas.view
    }
    pub fn sampler(&self) -> &wgpu::Sampler {
        &self.texture_atlas.sampler
    }
    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.texture_atlas.bind_group_layout
    }
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.texture_atlas.bind_group
    }
}

#[derive(Debug, Clone)]
struct LetterInfo {
    atlas_offset: Vec2,
    atlas_dimensions: Vec2,
    local_offset: Vec2,
    advance: Vec2,
}

pub struct GUIRenderer {
    batch: render::BatchBuffer<VertexUI>,
    pipeline: render::RenderPipeline,
    font_atlas: FontAtlas,
}

impl GUIRenderer {
    pub async fn new(ctx: &Context, vertices_batch_size: u32, indices_batch_size: u32) -> Self {
        let surface_config = render::surface_config(ctx);
        let batch = render::BatchBufferBuilder::new()
            .vertices_size(vertices_batch_size)
            .indices_size(indices_batch_size)
            .build(ctx);

        let font_atlas = FontAtlas::new(
            ctx,
            "font.ttf",
            "abcdefghijklmnopqrstuvxyzwABCDEFGHIJKLMNOPQRSTUVXYZW0123456789.,_-+*/ ",
        )
        .await;
        // println!("A info {:?}", letter_info.get(&'a'));

        let shader = render::ShaderBuilder::new("ui.wgsl")
            .buffers(vec![batch.vertices_desc()])
            .targets(vec![Some(wgpu::ColorTargetState {
                format: surface_config.format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })])
            .bind_group_layouts(vec![font_atlas.bind_group_layout()])
            .build(ctx)
            .await;
        let pipeline = render::RenderPipelineBuilder::new(&shader).build(ctx);
        Self {
            batch,
            pipeline,
            font_atlas,
        }
    }

    // TODO use existing render pass instead?
    pub fn render(&mut self, ctx: &Context, screen_view: &wgpu::TextureView) {
        // Update buffers with current frames data
        self.batch.upload_buffers(ctx);

        // Render batch
        let queue = render::queue(ctx);
        let mut encoder = render::create_encoder(ctx, None);
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: screen_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        render_pass.set_pipeline(self.pipeline.pipeline());
        render_pass.set_vertex_buffer(0, self.batch.vertices_slice(..));
        render_pass.set_index_buffer(self.batch.indices_slice(..), self.batch.indices_format());
        render_pass.set_bind_group(0, self.font_atlas.bind_group(), &[]);
        render_pass.draw_indexed(0..self.batch.indices_len(), 0, 0..1);

        drop(render_pass);
        queue.submit(Some(encoder.finish()));

        // Clear for next frame
        self.batch.clear();
    }

    #[rustfmt::skip]
    pub fn draw_quad(&mut self, pos: Vec2, size: Vec2, color: Vec4) {
        let size = size * 2.0;

        let offset = self.batch.vertices_len();
        let color = color.to_array();
        let (x, y) = (pos.x ,pos.y);
        let (sx, sy) = (size.x, size.y);
        
        self.batch.add_vertex(VertexUI { position: [-1.0 + x, 1.0 - y , 0.0], color, uv: [0.0, 0.0], ty: VERTEX_TYPE_SHAPE }); // tl
        self.batch.add_vertex(VertexUI { position: [-1.0 + x + sx, 1.0 - y, 0.0], color, uv: [0.0, 0.0], ty: VERTEX_TYPE_SHAPE }); // tr
        self.batch.add_vertex(VertexUI { position: [-1.0 + x, 1.0 - y - sy, 0.0], color, uv: [0.0, 0.0], ty: VERTEX_TYPE_SHAPE }); // bl
        self.batch.add_vertex(VertexUI { position: [-1.0 + x + sx, 1.0 - y - sy, 0.0], color, uv: [0.0, 0.0], ty: VERTEX_TYPE_SHAPE }); // br
        self.batch.add_index(offset); // tl 
        self.batch.add_index(offset + 1); // bl 
        self.batch.add_index(offset + 2); // tr
        self.batch.add_index(offset + 2); // tr 
        self.batch.add_index(offset + 1); // bl 
        self.batch.add_index(offset + 3); // br
    }

    // currently size.y does nothing
    pub fn draw_text(&mut self, pos: Vec2, size: Vec2, font_scale: f32, color: Vec4, text: &str) {
        let size = size * 2.0;

        let mut global_offset = vec2(0.0, 0.0);
        for letter in text.chars() {
            let info = self.font_atlas.get_info(letter);
            let atlas_dim = info.atlas_dimensions;

            if (global_offset.x + atlas_dim.x) * font_scale > size.x {
                global_offset.x = 0.0;
                global_offset.y += self.font_atlas.line_height;
            }

            let local_offset = global_offset + vec2(info.local_offset.x, -info.local_offset.y) + vec2(0.0, self.font_atlas.line_height - atlas_dim.y);
            let pos = pos + local_offset * font_scale;
            global_offset.x += info.advance.x;
            self.draw_letter(pos, font_scale, letter, color);
        }
    }

    #[rustfmt::skip]
    pub fn draw_letter(&mut self, pos: Vec2, scale: f32, letter: char, color: Vec4) {
        let info = self.font_atlas.get_info(letter);

        let texture_offset = info.atlas_offset;
        let texture_dim = info.atlas_dimensions;
        let scaled_dim = texture_dim * scale;

        let (x, y) = (pos.x, pos.y);
        let (sx, sy)= (scaled_dim.x, scaled_dim.y);
        let (tox, toy) = (texture_offset.x, texture_offset.y);
        let (tdx, tdy) =(texture_dim.x, texture_dim.y);
        let color = color.to_array();

        let vertex_offset = self.batch.vertices_len();
        self.batch.add_vertex(VertexUI { position: [-1.0 + x, 1.0 - y, 0.0],          ty: VERTEX_TYPE_TEXT, color, uv: [tox, toy] }); // tl
        self.batch.add_vertex(VertexUI { position: [-1.0 + x + sx, 1.0 - y, 0.0],     ty: VERTEX_TYPE_TEXT, color, uv: [tox + tdx, toy] }); // tr
        self.batch.add_vertex(VertexUI { position: [-1.0 + x, 1.0 - y - sy, 0.0],     ty: VERTEX_TYPE_TEXT, color, uv: [tox, toy + tdy] }); // bl
        self.batch.add_vertex(VertexUI { position: [-1.0 + x + sx, 1.0 - y -sy, 0.0], ty: VERTEX_TYPE_TEXT, color, uv: [tox + tdx, toy + tdy] }); // br
        self.batch.add_index(vertex_offset); // tl 
        self.batch.add_index(vertex_offset + 1); // bl 
        self.batch.add_index(vertex_offset + 2); // tr
        self.batch.add_index(vertex_offset + 2); // tr 
        self.batch.add_index(vertex_offset + 1); // bl 
        self.batch.add_index(vertex_offset + 3); // br
    }
}

const VERTEX_TYPE_SHAPE: u32 = 0;
const VERTEX_TYPE_TEXT: u32 = 1;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct VertexUI {
    pub position: [f32; 3],
    pub ty: u32, // 0 shape, 1 text
    pub color: [f32; 4],
    pub uv: [f32; 2],
}

impl VertexUI {
    const ATTRIBUTES: &'static [wgpu::VertexAttribute] = &wgpu::vertex_attr_array![
        0=>Float32x3,   // pos
        1=>Uint32,      // ty
        2=>Float32x4,   // color
        3=>Float32x2,   // uv
    ];
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Self>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: Self::ATTRIBUTES,
        }
    }
}

impl VertexTrait for VertexUI {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        Self::desc()
    }
}
