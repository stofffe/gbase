use crate::{render, Context};
use glam::{uvec2, vec2, vec4, Vec4};
use glam::{UVec2, Vec2};
use std::collections::HashMap;

pub struct FontAtlas {
    pub(crate) texture_atlas: render::TextureAtlas,
    pub(crate) letter_info: HashMap<char, LetterInfo>,

    pub(crate) font_info: FontInfo,
}

const FONT_RASTER_SIZE: f32 = 256.0;
const FONT_ATLAS_SIZE: UVec2 = uvec2(4096, 4096);
const FONT_ATLAS_PADDING: UVec2 = uvec2(10, 10);
pub const DEFAULT_SUPPORTED_CHARS: &str =
    " abcdefghijklmnopqrstuvxyzwABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789.,_-+*/=()[]{}:\"'?";
pub const SE_CHARS: &str = "åäöÅÄÖ";

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

        let texture =
            render::TextureBuilder::new(render::TextureSource::Empty(texture_dim.x, texture_dim.y))
                .format(wgpu::TextureFormat::R8Unorm)
                .build(ctx);
        let mut texture_atlas = render::TextureAtlasBuilder::new().build(texture);

        let mut offset = UVec2::ZERO;
        let padding = FONT_ATLAS_PADDING;

        let mut letter_info = HashMap::<char, LetterInfo>::new();

        for (metrics, bitmap, letter) in chars {
            let dimensions = uvec2(metrics.width as u32, metrics.height as u32);

            // wrap
            if dimensions.x + padding.x > texture_dim.x - offset.x {
                offset.y += max_height + padding.x;
                offset.x = 0;
            }

            #[rustfmt::skip]
            letter_info.insert(
                letter,
                LetterInfo {
                    // uv
                    atlas_offset: offset.as_vec2() / texture_dim.as_vec2(),
                    atlas_dimensions: dimensions.as_vec2() / texture_dim.as_vec2(),
                    size_unorm: vec2(metrics.width as f32, metrics.height as f32) / max_height as f32,
                    local_offset: vec2(metrics.xmin as f32, metrics.ymin as f32) / max_height as f32,
                    advance: vec2(metrics.advance_width, metrics.advance_height) / max_height as f32,
                },
            );

            // println!("{:?}", dimensions);
            texture_atlas.write_texture(ctx, offset, dimensions, &bitmap);
            offset.x += dimensions.x + padding.x;
        }

        let font_info = FontInfo {
            // height: max_height as f32,
            // height_unorm: max_height as f32 / FONT_RASTER_SIZE,
            // // TODO: temp
            // padding: FONT_ATLAS_PADDING.x as f32,
            // padding_unorm: FONT_ATLAS_PADDING.x as f32 / FONT_RASTER_SIZE,
        };

        Self {
            texture_atlas,
            letter_info,
            font_info,
        }
    }
}

impl FontAtlas {
    pub fn get_info(&self, letter: char) -> &LetterInfo {
        match self.letter_info.get(&letter) {
            Some(info) => info,
            None => panic!("trying to get unsupported letter \"{}\"", letter), // TODO default
        }
    }
    pub fn text_size(&self, text: &str, font_size: f32, wrap_width: Option<f32>) -> (Vec2, u32) {
        let mut size = Vec2::ZERO;
        let mut lines = 1;
        match wrap_width {
            None => {
                let mut sum = 0.0;
                for c in text.chars() {
                    let advance = self.get_info(c).advance.x * font_size;
                    sum += advance;
                }
                size.x = sum;
                size.y = font_size;
            }
            Some(wrap_width) => {
                let mut sum = 0.0;

                for c in text.chars() {
                    let advance = self.get_info(c).advance.x * font_size;

                    if (sum + advance) > wrap_width {
                        sum = 0.0;
                        lines += 1;
                    }
                    sum += advance;
                }
                size.x = wrap_width;
                size.y = lines as f32 * font_size;
            }
        }

        (size, lines)
    }
}

#[derive(Debug, Clone)]
pub struct LetterInfo {
    pub(crate) atlas_offset: Vec2,
    pub(crate) atlas_dimensions: Vec2,
    pub(crate) size_unorm: Vec2,
    pub(crate) local_offset: Vec2,

    pub(crate) advance: Vec2,
}

#[derive(Debug, Clone)]
pub struct FontInfo {
    // pub(crate) height_unorm: f32,
    // pub(crate) height: f32,
    // pub(crate) padding_unorm: f32,
    // pub(crate) padding: f32,
}
