use crate::{render, Context};
use glam::{uvec2, vec2, Vec4};
use glam::{UVec2, Vec2};
use image::error::ParameterErrorKind;
use std::collections::HashMap;
use super::VertexTrait;

struct FontAtlas {
    texture_atlas: render::TextureAtlas,
    info: HashMap<char, LetterInfo>,
    line_height: f32,
}

impl FontAtlas {
    fn new(ctx: &Context, font_bytes: &[u8], supported_chars: &str) -> Self {
        // texture
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

        let mut texture_atlas = render::TextureAtlas::new(ctx, texture_dim);
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
                    local_offset: vec2(metrics.xmin as f32, metrics.ymin as f32) / texture_dim.as_vec2(),
                    advance: vec2(metrics.advance_width, metrics.advance_height) / texture_dim.as_vec2(),
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
        self.texture_atlas.texture()
    }
    pub fn view(&self) -> &wgpu::TextureView {
        self.texture_atlas.view()
    }
    pub fn sampler(&self) -> &wgpu::Sampler {
        self.texture_atlas.sampler()
    }
    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        self.texture_atlas.bind_group_layout()
    }
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        self.texture_atlas.bind_group()
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
    vertices: render::DynamicVertexBuffer<VertexUI>,
    indices: render::DynamicIndexBuffer,

    pipeline: render::RenderPipeline,
    font_atlas: FontAtlas,
}

pub const DEFAULT_SUPPORTED_CHARS: &str = "abcdefghijklmnopqrstuvxyzwABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789.,_-+*/ ";
pub const DEFAULT_SUPPORTED_CHARS_SE: &str = "abcdefghijklmnopqrstuvwxyzwåäöABCDEFGHIJKLMNOPQRSTUVWXYZÅÄÖ0123456789.,_-+*/ ";

impl GUIRenderer {
    pub async fn new(ctx: &Context, vertices_batch_size: u32, indices_batch_size: u32, font_bytes: &[u8], supported_chars: &str) -> Self {
        let surface_config = render::surface_config(ctx);

        let vertices = render::DynamicVertexBufferBuilder::new().capacity(vertices_batch_size as usize).build(ctx);
        let indices = render::DynamicIndexBufferBuilder::new().capacity(indices_batch_size as usize).build(ctx);

        let font_atlas = FontAtlas::new(
            ctx,
            font_bytes,
            supported_chars,
        ) ;
        // println!("A info {:?}", letter_info.get(&'a'));

        let shader = render::ShaderBuilder::new("ui.wgsl")
            .buffers(vec![vertices.desc()])
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
            vertices,
            indices,
            pipeline,
            font_atlas,
        }
    }

    // TODO use existing render pass instead?
    pub fn render(&mut self, ctx: &Context, screen_view: &wgpu::TextureView) {
        // Update buffers with current frames data
        self.vertices.update_buffer(ctx);
        self.indices.update_buffer(ctx);

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
        render_pass.set_vertex_buffer(0, self.vertices.slice(..));
        render_pass.set_index_buffer(self.indices.slice(..), self.indices.format());
        render_pass.set_bind_group(0, self.font_atlas.bind_group(), &[]);
        render_pass.draw_indexed(0..self.indices.len(), 0, 0..1);

        drop(render_pass);
        queue.submit(Some(encoder.finish()));

        // Clear for next frame
        self.vertices.clear();
        self.indices.clear();
    }

    #[rustfmt::skip]
    pub fn draw_quad(&mut self, pos: Vec2, size: Vec2, color: Vec4) {
        let size = size * 2.0;

        let offset = self.vertices.len();
        let color = color.to_array();
        let (x, y) = (pos.x ,pos.y);
        let (sx, sy) = (size.x, size.y);
        
        self.vertices.add(VertexUI { position: [-1.0 + x, 1.0 - y , 0.0], color, uv: [0.0, 0.0], ty: VERTEX_TYPE_SHAPE }); // tl
        self.vertices.add(VertexUI { position: [-1.0 + x + sx, 1.0 - y, 0.0], color, uv: [0.0, 0.0], ty: VERTEX_TYPE_SHAPE }); // tr
        self.vertices.add(VertexUI { position: [-1.0 + x, 1.0 - y - sy, 0.0], color, uv: [0.0, 0.0], ty: VERTEX_TYPE_SHAPE }); // bl
        self.vertices.add(VertexUI { position: [-1.0 + x + sx, 1.0 - y - sy, 0.0], color, uv: [0.0, 0.0], ty: VERTEX_TYPE_SHAPE }); // br
        self.indices.add(offset); // tl 
        self.indices.add(offset + 1); // bl 
        self.indices.add(offset + 2); // tr
        self.indices.add(offset + 2); // tr 
        self.indices.add(offset + 1); // bl 
        self.indices.add(offset + 3); // br
    }

    // TODO scaling a bit weird
    // currently size.y does nothing
    /// pos \[0,1\]
    /// scale \[0,1\] 
    pub fn draw_text(&mut self, pos: Vec2, size: Vec2, font_scale: f32, color: Vec4, text: &str) {
        let mut global_offset = vec2(0.0, 0.0);
        for letter in text.chars() {
            let info = self.font_atlas.get_info(letter);
            let atlas_dim = info.atlas_dimensions;

            // Check wrap
            if (global_offset.x + atlas_dim.x) * font_scale > size.x * 2.0 {
                global_offset.x = 0.0;
                global_offset.y += self.font_atlas.line_height;
            }

            let local_offset = global_offset
                + vec2(info.local_offset.x, -info.local_offset.y)
                + vec2(0.0, self.font_atlas.line_height - atlas_dim.y);
            println!("local offset {local_offset})");
            let pos = pos + local_offset * font_scale / 2.0; // div by two 
            global_offset.x += info.advance.x;
            self.draw_letter(pos, font_scale, letter, color);
        }
    }

    #[rustfmt::skip]
    /// pos \[0,1\]
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

        let vertex_offset = self.vertices.len();
        self.vertices.add(VertexUI { position: [-1.0 + x * 2.0,      1.0 - y * 2.0,      0.0], ty: VERTEX_TYPE_TEXT, color, uv: [tox,       toy] }); // tl
        self.vertices.add(VertexUI { position: [-1.0 + x * 2.0 + sx, 1.0 - y * 2.0,      0.0], ty: VERTEX_TYPE_TEXT, color, uv: [tox + tdx, toy] }); // tr
        self.vertices.add(VertexUI { position: [-1.0 + x * 2.0,      1.0 - y * 2.0 - sy, 0.0], ty: VERTEX_TYPE_TEXT, color, uv: [tox,       toy + tdy] }); // bl
        self.vertices.add(VertexUI { position: [-1.0 + x * 2.0 + sx, 1.0 - y * 2.0 -sy,  0.0], ty: VERTEX_TYPE_TEXT, color, uv: [tox + tdx, toy + tdy] }); // br
        self.indices.add(vertex_offset); // tl 
        self.indices.add(vertex_offset + 1); // bl 
        self.indices.add(vertex_offset + 2); // tr
        self.indices.add(vertex_offset + 2); // tr 
        self.indices.add(vertex_offset + 1); // bl 
        self.indices.add(vertex_offset + 3); // br
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
