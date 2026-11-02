use crate::{
    ui_layout::{Glyph, TextLayoutResult, TextSizeResult},
    ui_renderer::{Font, SdfFontAtlas},
};
use gbase::{
    asset::{AssetCache, AssetHandle},
    rustc_hash::FxHashMap,
};

#[derive(Clone, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct FontId(u64);

pub struct UIFonts {
    fonts: FxHashMap<FontId, AssetHandle<Font>>,
    next_font_id: FontId,

    temp_font_handle: AssetHandle<Font>,
    temp_font_atlas_handle: AssetHandle<SdfFontAtlas>,
}

impl UIFonts {
    pub fn new(
        temp_font_handle: AssetHandle<Font>,
        temp_font_atlas_handle: AssetHandle<SdfFontAtlas>,
    ) -> Self {
        Self {
            fonts: FxHashMap::default(),
            next_font_id: FontId(1),
            temp_font_handle,
            temp_font_atlas_handle,
        }
    }

    pub fn register_font(&mut self, handle: AssetHandle<Font>) -> FontId {
        let font_id = self.next_font_id.clone();
        self.fonts.insert(font_id.clone(), handle);

        self.next_font_id.0 += 1;

        font_id
    }

    pub fn get_font(&self, font_id: &FontId) -> Option<&AssetHandle<Font>> {
        self.fonts.get(font_id)
    }
}

pub struct TextMeasureSettings<'a> {
    text: &'a str,
    font_size: u32,
    font: FontId,
    wrap_on_newline: bool,
}

// TODO: combine into one func with optional max width?
pub trait TextMeasurer {
    fn calculate_preferred_text_size(
        &mut self,
        text: &str,
        font_size: u32,
        wrap_on_newline: bool,
    ) -> TextSizeResult;

    fn layout_text(&mut self, text: &str, font_size: u32, max_width: f32) -> TextLayoutResult;
}

pub struct FontTextMeasurer<'a> {
    pub fonts: &'a UIFonts,
    pub cache: &'a mut AssetCache,
}

impl<'a> TextMeasurer for FontTextMeasurer<'a> {
    fn calculate_preferred_text_size(
        &mut self,
        text: &str,
        font_size: u32,
        wrap_on_newline: bool,
    ) -> TextSizeResult {
        let Ok(font) = self.cache.get_asset(&self.fonts.temp_font_handle).cloned() else {
            // tracing::error!("could not get font");
            return TextSizeResult {
                max_width: 0.0,
                max_height: 0.0,
                min_width: 0.0,
                min_height: 0.0,
            };
        };

        let line_metrics = font
            .font
            .horizontal_line_metrics(font_size as f32)
            .expect("could not get line metrics");
        let line_height = line_metrics.new_line_size;

        let mut x_offset = 0.0f32;
        let mut y_offset = 0.0f32;
        let mut longest_line = 0.0f32;
        let mut shortest_word = f32::MAX;
        // width
        let mut current_word_width = 0.0;
        let mut prev_char = None;
        for letter in text.chars() {
            // let font_metrics = font.font.metrics(letter, font_size as f32);
            let font_metrics = font.font.metrics(letter, font_size as f32);

            // TODO: might be a bit too long due to using advance and not width
            if wrap_on_newline && letter == '\n' {
                longest_line = longest_line.max(x_offset);
                x_offset = 0.0;
                y_offset += line_height;
                continue;
            }

            if letter.is_whitespace() {
                shortest_word = shortest_word.min(current_word_width);
                current_word_width = 0.0;
            }

            if let Some(prev) = prev_char {
                if let Some(kern) = font.font.horizontal_kern(prev, letter, font_size as f32) {
                    x_offset += kern;
                }
            }

            current_word_width += font_metrics.advance_width;
            x_offset += font_metrics.advance_width;

            prev_char = Some(letter);
        }

        let preferred_width = longest_line.max(x_offset);
        let preferred_height = y_offset + line_height;
        let min_width = shortest_word;
        // TODO: needed?
        let min_height = preferred_height;

        TextSizeResult {
            max_width: preferred_width,
            max_height: preferred_height,
            min_width,
            min_height,
        }
    }

    fn layout_text(&mut self, text: &str, font_size: u32, max_width: f32) -> TextLayoutResult {
        let Ok(font) = self.cache.get_asset(&self.fonts.temp_font_handle).cloned() else {
            return TextLayoutResult {
                width: 0.0,
                height: 0.0,
                glyphs: Vec::new(),
            };
        };

        let line_metrics = font
            .font
            .horizontal_line_metrics(font_size as f32)
            .expect("could not get line metrics");
        let line_height = line_metrics.new_line_size;

        let text = text.chars().collect::<Vec<_>>();
        let mut glyphs = Vec::new();

        let mut x_offset = 0.0f32;
        let mut y_offset = 0.0f32;
        let mut longest_line_width = 0.0f32;

        let mut current_word_start = 0;
        let mut current_word_len = 0;
        let mut current_word_width = 0.0;

        let mut prev_char = None;
        for text_index in 0..text.len() {
            let letter = text[text_index];
            let font_metrics = font.font.metrics(letter, font_size as f32);

            let is_last_element = text_index == text.len() - 1;
            let is_first_element = current_word_start == 0;
            let is_whitespace = letter == ' ' || letter == '\t' || letter == '\n';

            // add letter if not whitespace
            if !is_whitespace {
                if let Some(prev) = prev_char {
                    if let Some(kern) = font.font.horizontal_kern(prev, letter, font_size as f32) {
                        current_word_width += kern;
                    }
                }
                current_word_width += font_metrics.advance_width;
                current_word_len += 1;
                prev_char = Some(letter);
            }

            let word_finished = is_whitespace || is_last_element;

            let mut wrapped = false;
            // create glyphs for words when needed
            if word_finished {
                if current_word_len > 0 {
                    // wrap check
                    if x_offset + current_word_width > max_width {
                        y_offset += line_height;
                        longest_line_width = longest_line_width.max(x_offset);
                        x_offset = 0.0;
                        wrapped = true;
                    }

                    // TODO: how to handle tab

                    if letter == ' ' && !wrapped && !is_first_element {
                        let glyph_metrics = font.font.metrics(letter, font_size as f32);
                        glyphs.push(Glyph {
                            character: letter,
                            x: x_offset,
                            y: y_offset + line_height
                                - glyph_metrics.height as f32
                                - glyph_metrics.ymin as f32
                                + line_metrics.descent,
                            width: glyph_metrics.width as f32,
                            height: glyph_metrics.height as f32,
                        });
                        x_offset += font_metrics.advance_width;
                    }

                    // create glyphs for all letters in word
                    let mut prev_char = None;
                    for text_index in current_word_start..current_word_start + current_word_len {
                        let text_char = text[text_index];
                        let glyph_metrics = font.font.metrics(text_char, font_size as f32);
                        glyphs.push(Glyph {
                            character: text_char,
                            x: x_offset,
                            y: y_offset + line_height
                                - glyph_metrics.height as f32
                                - glyph_metrics.ymin as f32
                                + line_metrics.descent,
                            width: glyph_metrics.width as f32,
                            height: glyph_metrics.height as f32,
                        });

                        if let Some(prev) = prev_char {
                            if let Some(kern) =
                                font.font.horizontal_kern(prev, text_char, font_size as f32)
                            {
                                x_offset += kern;
                            }
                        }
                        prev_char = Some(text_char);
                        x_offset += glyph_metrics.advance_width;
                    }
                }

                // TODO: should maybe be a setting
                if letter == '\n' {
                    y_offset += line_height;
                    longest_line_width = longest_line_width.max(x_offset);
                    x_offset = 0.0;
                }

                // reset state
                current_word_start = text_index + 1;
                current_word_len = 0;
                current_word_width = 0.0;
            }
        }

        let width = longest_line_width.max(x_offset);
        let height = y_offset + line_height;

        TextLayoutResult {
            width,
            height,
            glyphs,
        }
    }
}
