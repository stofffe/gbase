use crate::ui_layout::{
    layouter::UILayouter, LayoutDirection, Padding, Sizing, UIElement, UIElementId,
};
use gbase::glam::Vec4;

/// Builder for UIElement that uses types to constrain what fields can be set with two sub builders
/// * UITextBuilder
/// * UIContainerBuilder
pub struct UIBuilder {
    element: UIElement,
}

impl UIBuilder {
    pub fn new() -> Self {
        Self {
            element: UIElement::new(),
        }
    }

    pub fn element(self) -> UIElement {
        self.element
    }

    pub fn draw(self, layouter: &mut UILayouter) -> UIElementId {
        let element = self.element();
        element.draw(layouter)
    }

    // TODO: set content type?
    // maybe no since this should just wrap ui element and it should handle that by itself?

    /// Make this ui element a container element that can have children
    /// Switches the builder to a UIContainerBuilder
    pub fn container(self) -> UIContainerBuilder {
        UIContainerBuilder { builder: self }
    }

    // TODO: set content type?
    // maybe no since this should just wrap ui element and it should handle that by itself?

    /// Make this ui element a text element
    /// Switches the builder to a UITextBuilder
    pub fn text(self, text: impl Into<String>) -> UITextBuilder {
        UITextBuilder {
            ui_element: self.element.with_text(text),
        }
    }

    pub fn sizing_x(mut self, sizing: Sizing) -> Self {
        self.element.set_sizing_x(sizing);
        self
    }
    pub fn sizing_y(mut self, sizing: Sizing) -> Self {
        self.element.set_sizing_y(sizing);
        self
    }
    pub fn background_color(mut self, background_color: Vec4) -> Self {
        self.element.set_background_color(background_color);
        self
    }
    pub fn padding(mut self, padding: Padding) -> Self {
        self.element.set_padding(padding);
        self
    }
}

pub struct UIContainerBuilder {
    builder: UIBuilder,
}

impl UIContainerBuilder {
    pub fn element(self) -> UIElement {
        self.builder.element()
    }

    pub fn draw_with_children<F: FnOnce(&mut UILayouter) -> R, R>(
        self,
        layouter: &mut UILayouter,
        children: F,
    ) -> (UIElementId, R) {
        self.element().draw_with_children(layouter, children)
    }

    pub fn draw(self, layouter: &mut UILayouter) -> UIElementId {
        self.builder.draw(layouter)
    }

    pub fn layout_direction(mut self, layout_direction: LayoutDirection) -> Self {
        self.builder.element.set_layout_direction(layout_direction);
        self
    }

    pub fn child_gap(mut self, child_gap: f32) -> Self {
        self.builder.element.set_child_gap(child_gap);
        self
    }
}

pub struct UITextBuilder {
    ui_element: UIElement,
}

impl UITextBuilder {
    pub fn element(self) -> UIElement {
        self.ui_element
    }

    pub fn draw(self, layouter: &mut UILayouter) -> UIElementId {
        let element = self.element();
        element.draw(layouter)
    }

    pub fn text_color(mut self, text_color: Vec4) -> Self {
        self.ui_element.set_text_color(text_color);
        self
    }

    pub fn font_size(mut self, font_size: u32) -> Self {
        self.ui_element.set_font_size(font_size);
        self
    }
}
