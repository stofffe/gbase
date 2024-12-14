use crate::{render, Context};
use glam::{uvec2, vec2, vec4, Vec4};
use glam::{UVec2, Vec2};
use std::collections::HashMap;

pub struct FontAtlas {
    pub(crate) texture_atlas: render::TextureAtlas,
    pub(crate) info: HashMap<char, LetterInfo>,

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
    pub(crate) fn new(ctx: &mut Context, font_bytes: &[u8], supported_chars: &str) -> Self {
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
pub struct LetterInfo {
    pub(crate) atlas_offset: Vec2,
    pub(crate) atlas_dimensions: Vec2,
    pub(crate) size: Vec2,
    pub(crate) local_offset: Vec2,
    pub(crate) advance: Vec2,
}
