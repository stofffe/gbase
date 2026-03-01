use crate::ui_layout::UILayoutTextMeasurer;

pub struct UIFont {}

impl UIFont {
    pub fn new() -> Self {
        Self {}
    }
}

impl UILayoutTextMeasurer for UIFont {
    fn measure_text(&self, text: &str, max_width: f32) -> (f32, f32) {
        todo!()
    }
}
