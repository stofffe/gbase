use std::collections::HashMap;

use crate::ui_layout::UILayoutTextMeasurer;

pub struct UIFont {
    lookup: HashMap<(char, u32), fontdue::Metrics>,
    font: fontdue::Font,
}

impl UIFont {
    pub fn new(bytes: &[u8]) -> Self {
        let font = fontdue::Font::from_bytes(bytes, fontdue::FontSettings::default())
            .expect("could not load font");
        let lookup = HashMap::new();

        Self { lookup, font }
    }

    fn get_letter_info(&mut self, letter: char, font_size: u32) -> fontdue::Metrics {
        if let Some(metrics) = self.lookup.get(&(letter, font_size)) {
            return metrics.clone();
        }

        let (metrics, _) = self.font.rasterize(letter, font_size as f32);
        self.lookup.insert((letter, font_size), metrics.clone());
        metrics
    }
}

impl UILayoutTextMeasurer for UIFont {
    fn measure_text(&mut self, text: &str, font_size: u32) -> (f32, f32) {
        let mut width = 0.0f32;
        let mut height = 0.0f32;

        // width
        let mut prev_char = None;
        for letter in text.chars() {
            let metrics = self.get_letter_info(letter, font_size);

            if let Some(prev) = prev_char {
                if let Some(kern) = self.font.horizontal_kern(prev, letter, font_size as f32) {
                    width += kern;
                }
            }

            width += metrics.advance_width;
            height = height.max(metrics.height as f32);

            prev_char = Some(letter);
        }

        // height
        // if none use the max of all individual chars
        if let Some(line_metrics) = self.font.horizontal_line_metrics(font_size as f32) {
            height = line_metrics.ascent - line_metrics.descent + line_metrics.line_gap;
        }

        (width, height)
    }
}
