pub mod builder;
pub mod layouter;

#[cfg(test)]
mod test;

use crate::ui_layout::layouter::UILayouter;
use gbase::glam::{vec4, Vec4};

const ROOT_ELEMENT: usize = 0;

pub struct UIElementId(usize);

#[derive(Debug, Clone)]
pub struct UIElement {
    // general
    content_type: ContentType,
    sizing: [Sizing; 2],
    padding: Padding,
    pub background_color: Vec4,
    min_override: [Option<f32>; 2],
    max_override: [Option<f32>; 2],

    // container
    layout_direction: LayoutDirection,
    child_gap: f32,

    // text
    pub text: String,
    pub text_color: Vec4,
    pub font_size: u32,

    // calculated
    pub text_layout: TextLayoutResult,
    min: [f32; 2],
    max: [f32; 2],
    pub size: [f32; 2],
    pub pos: [f32; 2],
    pub parent: usize,
    pub children: Vec<usize>,
}

impl UIElement {
    pub fn new() -> Self {
        Self {
            // general
            content_type: ContentType::Container,
            sizing: [Sizing::Fit, Sizing::Fit],
            padding: Padding {
                top: 0.0,
                bottom: 0.0,
                left: 0.0,
                right: 0.0,
            },
            background_color: vec4(0.0, 0.0, 0.0, 1.0),
            min_override: [None, None],
            max_override: [None, None],

            // container
            layout_direction: LayoutDirection::LeftToRight,
            child_gap: 0.0,

            // text
            text: String::new(),
            text_color: Vec4::ZERO,
            font_size: 12,

            // calculated
            text_layout: TextLayoutResult {
                width: 0.0,
                height: 0.0,
                glyphs: Vec::new(),
            },
            min: [0.0, 0.0],
            max: [0.0, 0.0],
            size: [0.0, 0.0],
            pos: [0.0, 0.0],
            parent: ROOT_ELEMENT,
            children: Vec::new(),
        }
    }

    //
    // Attributes
    //

    pub fn sizing_x(mut self, sizing: Sizing) -> Self {
        self.sizing[0] = sizing;
        self
    }
    pub fn sizing_y(mut self, sizing: Sizing) -> Self {
        self.sizing[1] = sizing;
        self
    }
    pub fn background_color(mut self, background_color: Vec4) -> Self {
        self.background_color = background_color;
        self
    }
    pub fn layout_direction(mut self, layout_direction: LayoutDirection) -> Self {
        self.layout_direction = layout_direction;
        self
    }
    pub fn padding(mut self, padding: Padding) -> Self {
        self.padding = padding;
        self
    }
    pub fn child_gap(mut self, child_gap: f32) -> Self {
        self.child_gap = child_gap;
        self
    }
    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.text = text.into();
        self.content_type = ContentType::Text;
        self
    }
    pub fn font_size(mut self, font_size: u32) -> Self {
        self.font_size = font_size;
        self
    }
    pub fn text_color(mut self, text_color: Vec4) -> Self {
        self.text_color = text_color;
        self
    }

    //
    // Helpers
    //

    pub fn draw_with_children<F: FnOnce(&mut UILayouter) -> R, R>(
        self,
        layouter: &mut UILayouter,
        children: F,
    ) -> (UIElementId, R) {
        layouter.add_element(self, children)
    }

    pub fn draw(self, layouter: &mut UILayouter) -> UIElementId {
        let (id, _) = layouter.add_element(self, |_| {});
        id
    }
}

enum Axis {
    X,
    Y,
}

#[derive(Debug, Clone)]
pub struct ContainerInfo {
    layout_direction: LayoutDirection,
    child_gap: f32,
}

impl ContainerInfo {
    pub fn new() -> Self {
        Self {
            layout_direction: LayoutDirection::LeftToRight,
            child_gap: 0.0,
        }
    }

    pub fn direction(mut self, direction: LayoutDirection) -> Self {
        self.layout_direction = direction;
        self
    }

    pub fn child_gap(mut self, child_gap: f32) -> Self {
        self.child_gap = child_gap;
        self
    }
}

#[derive(Debug, Clone)]
pub enum ContentType {
    Container,
    Text,
}

#[derive(Debug, Clone)]
pub struct TextInfo {
    pub text: String,
    pub text_color: Vec4,
    pub font_size: u32,
}

impl TextInfo {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            text_color: vec4(0.0, 0.0, 0.0, 1.0),
            font_size: 12,
        }
    }

    pub fn font_size(mut self, font_size: u32) -> Self {
        self.font_size = font_size;
        self
    }
    pub fn text_color(mut self, text_color: Vec4) -> Self {
        self.text_color = text_color;
        self
    }
}

#[derive(Debug, Clone)]
pub enum Sizing {
    Fixed(f32),
    Fit, // acts as fit(max_content) right now
    Percent(f32),
    Grow,
}

#[derive(Debug, Clone)]
pub enum LayoutDirection {
    LeftToRight,
    TopToBottom,
}

#[derive(Debug, Clone)]
pub struct Padding {
    top: f32,
    bottom: f32,
    left: f32,
    right: f32,
}

impl Padding {
    pub fn new(top: f32, right: f32, bottom: f32, left: f32) -> Self {
        Self {
            top,
            bottom,
            left,
            right,
        }
    }
    fn horizontal(&self) -> f32 {
        self.left + self.right
    }
    fn vertical(&self) -> f32 {
        self.bottom + self.top
    }
}

#[derive(Debug)]
pub struct TextSizeResult {
    pub min_width: f32,
    pub min_height: f32,
    pub max_width: f32,
    pub max_height: f32,
}

// TODO: should not be clone
#[derive(Debug, Clone)]
pub struct TextLayoutResult {
    pub width: f32,
    pub height: f32,

    pub glyphs: Vec<Glyph>,
}

// glyph relative to its parent container

#[derive(Debug, Clone)]
pub struct Glyph {
    pub character: char,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}
