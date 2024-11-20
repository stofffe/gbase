use crate::collision::Quad;
use crate::render::{ArcBindGroup, ArcRenderPipeline};
use crate::{collision, filesystem, input, render, Context};
use glam::{uvec2, vec2, vec4, Vec4};
use glam::{UVec2, Vec2};
use render::VertexTrait;
use std::collections::HashMap;
use winit::event::MouseButton;

struct FontAtlas {
    texture_atlas: render::TextureAtlas,
    info: HashMap<char, LetterInfo>,

    #[allow(dead_code)]
    line_height: f32,
}

const FONT_RASTER_SIZE: f32 = 256.0;
const FONT_ATLAS_SIZE: UVec2 = uvec2(4096, 4096);
const FONT_ATLAS_PADDING: UVec2 = uvec2(10, 10);
pub const DEFAULT_SUPPORTED_CHARS: &str =
    "abcdefghijklmnopqrstuvxyzwABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789.,_-+*/ ()[]{}:";
pub const DEFAULT_SUPPORTED_CHARS_SE: &str =
    "abcdefghijklmnopqrstuvwxyzwåäöABCDEFGHIJKLMNOPQRSTUVWXYZÅÄÖ0123456789.,_-+*/ ()[]{}:";

pub const RED: Vec4 = vec4(1.0, 0.0, 0.0, 1.0);
pub const GREEN: Vec4 = vec4(0.0, 1.0, 0.0, 1.0);
pub const BLUE: Vec4 = vec4(0.0, 0.0, 1.0, 1.0);
pub const BLACK: Vec4 = vec4(0.0, 0.0, 0.0, 1.0);
pub const WHITE: Vec4 = vec4(1.0, 1.0, 1.0, 1.0);
pub const GRAY: Vec4 = vec4(0.5, 0.5, 0.5, 1.0);

impl FontAtlas {
    fn new(ctx: &mut Context, font_bytes: &[u8], supported_chars: &str) -> Self {
        // texture
        let font = fontdue::Font::from_bytes(font_bytes, fontdue::FontSettings::default()).unwrap();

        let chars = supported_chars
            .chars()
            .map(|letter| {
                let (metrics, bitmap) = font.rasterize(letter, FONT_RASTER_SIZE);
                (metrics, bitmap, letter)
            })
            .collect::<Vec<_>>();
        // chars.sort_by(|a, b| a.0.height.partial_cmp(&b.0.height).unwrap());
        let texture_dim = FONT_ATLAS_SIZE;
        let max_height = chars
            .iter()
            .map(|(metrics, _, _)| metrics.height)
            .max()
            .unwrap() as u32;
        let line_height = max_height as f32 / FONT_RASTER_SIZE;

        let texture =
            render::TextureBuilder::new(render::TextureSource::Empty(texture_dim.x, texture_dim.y))
                .format(wgpu::TextureFormat::R8Unorm)
                .build(ctx);
        let mut texture_atlas = render::TextureAtlasBuilder::new().build(texture);

        let mut offset = UVec2::ZERO;
        let padding = FONT_ATLAS_PADDING;

        let mut info = HashMap::<char, LetterInfo>::new();

        for (metrics, bitmap, letter) in chars {
            let dimensions = uvec2(metrics.width as u32, metrics.height as u32);

            // wrap
            if dimensions.x + padding.x > texture_dim.x - offset.x {
                offset.y += max_height + padding.x;
                offset.x = 0;
            }

            #[rustfmt::skip]
            info.insert(
                letter,
                LetterInfo {
                    // uv
                    atlas_offset: offset.as_vec2() / texture_dim.as_vec2(),
                    atlas_dimensions: dimensions.as_vec2() / texture_dim.as_vec2(),
                    size: vec2(metrics.width as f32, metrics.height as f32) / max_height as f32,
                    local_offset: vec2(metrics.xmin as f32, metrics.ymin as f32) / max_height as f32,
                    advance: vec2(metrics.advance_width, metrics.advance_height) / max_height as f32,
                },
            );

            // println!("{:?}", dimensions);
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
}

#[derive(Debug, Clone)]
struct LetterInfo {
    atlas_offset: Vec2,
    atlas_dimensions: Vec2,
    size: Vec2,
    local_offset: Vec2,
    advance: Vec2,
}

pub struct GUIRenderer {
    dynamic_vertices: Vec<VertexUI>,
    dynamic_indices: Vec<u32>,
    vertices: render::VertexBuffer<VertexUI>,
    indices: render::IndexBuffer,

    pipeline: ArcRenderPipeline,
    font_atlas: FontAtlas,
    font_atlas_bindgroup: ArcBindGroup,
}

impl GUIRenderer {
    pub async fn new(
        ctx: &mut Context,
        output_format: wgpu::TextureFormat,
        vertices_batch_size: u32,
        indices_batch_size: u32,
        font_bytes: &[u8],
        supported_chars: &str,
    ) -> Self {
        let dynamic_vertices = Vec::with_capacity(vertices_batch_size as usize);
        let dynamic_indices = Vec::with_capacity(indices_batch_size as usize);
        let vertices = render::VertexBufferBuilder::new(render::VertexBufferSource::Empty(
            vertices_batch_size as u64,
        ))
        .build(ctx);
        let indices = render::IndexBufferBuilder::new(render::IndexBufferSource::Empty(
            indices_batch_size as u64,
        ))
        .build(ctx);

        let sampler = render::SamplerBuilder::new().build(ctx);
        let font_atlas = FontAtlas::new(ctx, font_bytes, supported_chars);

        let shader_str = filesystem::load_string(ctx, "shaders/ui.wgsl")
            .await
            .unwrap();
        let shader = render::ShaderBuilder::new(shader_str).build(ctx);

        let bindgroup_layout = render::BindGroupLayoutBuilder::new()
            .entries(vec![
                // texture atlas
                render::BindGroupLayoutEntry::new()
                    .texture_float_filterable()
                    .fragment(),
                // sampler
                render::BindGroupLayoutEntry::new()
                    .sampler_filtering()
                    .fragment(),
            ])
            .build(ctx);
        let bindgroup = render::BindGroupBuilder::new(bindgroup_layout.clone())
            .entries(vec![
                // texture atlas
                render::BindGroupEntry::Texture(font_atlas.texture_atlas.texture().view()),
                // sampler
                render::BindGroupEntry::Sampler(sampler),
            ])
            .build(ctx);

        let pipeline_layout = render::PipelineLayoutBuilder::new()
            .bind_groups(vec![bindgroup_layout])
            .build(ctx);
        let pipeline = render::RenderPipelineBuilder::new(shader, pipeline_layout)
            .buffers(vec![vertices.desc()])
            .targets(vec![Some(wgpu::ColorTargetState {
                format: output_format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })])
            .build(ctx);

        Self {
            dynamic_vertices,
            dynamic_indices,
            vertices,
            indices,
            pipeline,
            font_atlas,
            font_atlas_bindgroup: bindgroup,
        }
    }

    // TODO use existing render pass instead?
    pub fn render(&mut self, ctx: &Context, screen_view: &wgpu::TextureView) {
        // Update buffers with current frames data

        self.vertices.write(ctx, &self.dynamic_vertices);
        self.indices.write(ctx, &self.dynamic_indices);

        // Render batch
        let queue = render::queue(ctx);
        let mut encoder = render::create_encoder(ctx, None);

        render::RenderPassBuilder::new()
            .color_attachments(&[Some(wgpu::RenderPassColorAttachment {
                view: screen_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })])
            .build_run(&mut encoder, |mut render_pass| {
                render_pass.set_pipeline(&self.pipeline);
                render_pass.set_vertex_buffer(0, self.vertices.slice(..));
                render_pass.set_index_buffer(self.indices.slice(..), self.indices.format());
                render_pass.set_bind_group(0, &self.font_atlas_bindgroup, &[]);
                render_pass.draw_indexed(0..self.indices.len(), 0, 0..1);
            });

        queue.submit(Some(encoder.finish()));

        // Clear for next frame
        self.dynamic_vertices.clear();
        self.dynamic_indices.clear();
    }

    pub fn button(&mut self, ctx: &Context, quad: Quad, color: Vec4) -> bool {
        self.quad(quad, color);

        let mouse_clicked = input::mouse_button_just_pressed(ctx, MouseButton::Left);
        let hovering = collision::point_quad_collision(input::mouse_pos_unorm(ctx), quad);

        mouse_clicked && hovering
    }
    pub fn button_text(
        &mut self,
        ctx: &Context,
        text: &str,
        wrap: bool,
        line_height: f32,
        quad: Quad,
        color: Vec4,
    ) -> bool {
        self.quad(quad, color);
        self.text(text, quad, line_height, vec4(1.0, 1.0, 1.0, 1.0), wrap);

        let mouse_clicked = input::mouse_button_just_pressed(ctx, MouseButton::Left);
        let hovering = collision::point_quad_collision(input::mouse_pos_unorm(ctx), quad);

        mouse_clicked && hovering
    }

    #[rustfmt::skip]
    pub fn quad(&mut self, quad: Quad, color: Vec4) {
        let (x, y) = (quad.origin.x ,quad.origin.y);
        let (sx, sy) = (quad.dimension.x, quad.dimension.y);
        let color = color.to_array();
        let uv = [0.0,0.0];
        let ty = VERTEX_TYPE_SHAPE;

        let offset = self.dynamic_vertices.len() as u32;
        self.dynamic_vertices.push(VertexUI { position: [-1.0 + x * 2.0,            1.0 - y * 2.0,            0.0], color, ty, uv }); // tl
        self.dynamic_vertices.push(VertexUI { position: [-1.0 + x * 2.0 + sx * 2.0, 1.0 - y * 2.0,            0.0], color, ty, uv }); // tr
        self.dynamic_vertices.push(VertexUI { position: [-1.0 + x * 2.0,            1.0 - y * 2.0 - sy * 2.0, 0.0], color, ty, uv }); // bl
        self.dynamic_vertices.push(VertexUI { position: [-1.0 + x * 2.0 + sx * 2.0, 1.0 - y * 2.0 - sy * 2.0, 0.0], color, ty, uv }); // br
        self.dynamic_indices.push(offset); // tl
        self.dynamic_indices.push(offset + 1); // bl
        self.dynamic_indices.push(offset + 2); // tr
        self.dynamic_indices.push(offset + 2); // tr
        self.dynamic_indices.push(offset + 1); // bl
        self.dynamic_indices.push(offset + 3); // br
    }

    // TODO scaling a bit weird
    // currently size.y does nothing
    /// pos \[0,1\]
    /// scale \[0,1\]
    pub fn text(&mut self, text: &str, quad: Quad, line_height: f32, color: Vec4, wrap: bool) {
        let mut global_offset = vec2(0.0, 0.0); // [0,1]
        for letter in text.chars() {
            let info = self.font_atlas.get_info(letter);
            let size = info.size * line_height;
            let loc_offset = info.local_offset * line_height;
            let adv = info.advance * line_height;

            // word wrapping
            if wrap && (global_offset.x + size.x) > quad.dimension.x {
                global_offset.x = 0.0;
                global_offset.y += line_height;
            }

            let offset = quad.origin
                + global_offset
                + vec2(loc_offset.x, -loc_offset.y)
                + vec2(0.0, line_height - size.y);
            self.letter(offset, line_height, letter, color);
            global_offset.x += adv.x;
        }
    }

    #[rustfmt::skip]
    /// pos \[0,1\]
    /// line height \[0,1\]
    pub fn letter(&mut self, pos: Vec2, line_height: f32, letter: char, color: Vec4) {
        let info = self.font_atlas.get_info(letter);

        let atlas_offset = info.atlas_offset;
        let atlas_dim = info.atlas_dimensions;

        let scaled_dim = info.size * line_height;

        let (x, y) = (pos.x, pos.y);
        let (sx, sy)= (scaled_dim.x, scaled_dim.y);
        let (tox, toy) = (atlas_offset.x, atlas_offset.y);
        let (tdx, tdy) =(atlas_dim.x, atlas_dim.y);
        let color = color.to_array();
        let ty = VERTEX_TYPE_TEXT;

        let vertex_offset = self.dynamic_vertices.len() as u32;
        self.dynamic_vertices.push(VertexUI { position: [-1.0 + x * 2.0,            1.0 - y * 2.0,            0.0], ty, color, uv: [tox,       toy] }); // tl
        self.dynamic_vertices.push(VertexUI { position: [-1.0 + x * 2.0 + sx * 2.0, 1.0 - y * 2.0,            0.0], ty, color, uv: [tox + tdx, toy] }); // tr
        self.dynamic_vertices.push(VertexUI { position: [-1.0 + x * 2.0,            1.0 - y * 2.0 - sy * 2.0, 0.0], ty, color, uv: [tox,       toy + tdy] }); // bl
        self.dynamic_vertices.push(VertexUI { position: [-1.0 + x * 2.0 + sx * 2.0, 1.0 - y * 2.0 - sy * 2.0, 0.0], ty, color, uv: [tox + tdx, toy + tdy] }); // br
        self.dynamic_indices.push(vertex_offset); // tl
        self.dynamic_indices.push(vertex_offset + 1); // bl
        self.dynamic_indices.push(vertex_offset + 2); // tr
        self.dynamic_indices.push(vertex_offset + 2); // tr
        self.dynamic_indices.push(vertex_offset + 1); // bl
        self.dynamic_indices.push(vertex_offset + 3); // br
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
