use gbase::glam::{vec4, Vec4};

use crate::ui_layout::{
    layouter::UILayouter, ContentType, LayoutDirection, Padding, Sizing, UIElement, UIElementId,
};

pub struct UIBuilder {
    sizing: [Sizing; 2],
    padding: Padding,
    pub background_color: Vec4,
    min_override: [Option<f32>; 2],
    max_override: [Option<f32>; 2],
}

impl UIBuilder {
    pub fn new() -> Self {
        Self {
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
        }
    }

    pub fn build(self) -> UIElement {
        let mut element = UIElement::new();
        element.content_type = ContentType::None;
        element.sizing = self.sizing;
        element.padding = self.padding;
        element.background_color = self.background_color;
        element.min_override = self.min_override;
        element.max_override = self.max_override;
        element
    }

    pub fn draw(self, layouter: &mut UILayouter) -> UIElementId {
        let element = self.build();
        element.draw(layouter)
    }

    pub fn container(self) -> UIContainerBuilder {
        UIContainerBuilder {
            ui_builder: self,
            layout_direction: LayoutDirection::LeftToRight,
            child_gap: 0.0,
        }
    }

    pub fn text(self, text: impl Into<String>) -> UITextBuilder {
        UITextBuilder {
            ui_builder: self,
            text: text.into(),
            text_color: vec4(0.0, 0.0, 0.0, 1.0),
            font_size: 12,
        }
    }

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
    pub fn padding(mut self, padding: Padding) -> Self {
        self.padding = padding;
        self
    }
}

pub struct UIContainerBuilder {
    ui_builder: UIBuilder,

    layout_direction: LayoutDirection,
    child_gap: f32,
}

impl UIContainerBuilder {
    pub fn build(self) -> UIElement {
        let mut element = self.ui_builder.build();
        element.content_type = ContentType::Children;
        element.layout_direction = self.layout_direction;
        element.child_gap = self.child_gap;
        element
    }

    pub fn draw_with_children<F: FnOnce(&mut UILayouter) -> R, R>(
        self,
        layouter: &mut UILayouter,
        children: F,
    ) -> (UIElementId, R) {
        let element = self.build();
        element.draw_with_children(layouter, children)
    }

    pub fn draw(self, layouter: &mut UILayouter) -> UIElementId {
        let element = self.build();
        element.draw(layouter)
    }

    pub fn layout_direction(mut self, layout_direction: LayoutDirection) -> Self {
        self.layout_direction = layout_direction;
        self
    }

    pub fn child_gap(mut self, child_gap: f32) -> Self {
        self.child_gap = child_gap;
        self
    }
}

pub struct UITextBuilder {
    ui_builder: UIBuilder,
    text: String,
    text_color: Vec4,
    font_size: u32,
}

impl UITextBuilder {
    pub fn build(self) -> UIElement {
        let mut element = self.ui_builder.build();
        element.content_type = ContentType::Text;
        element.text = self.text;
        element.text_color = self.text_color;
        element.font_size = self.font_size;
        element
    }

    pub fn draw(self, layouter: &mut UILayouter) -> UIElementId {
        let element = self.build();
        element.draw(layouter)
    }

    pub fn text_color(mut self, text_color: Vec4) -> Self {
        self.text_color = text_color;
        self
    }

    pub fn font_size(mut self, font_size: u32) -> Self {
        self.font_size = font_size;
        self
    }
}
