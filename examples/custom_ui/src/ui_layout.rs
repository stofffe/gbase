use gbase::glam::{vec2, Vec2, Vec4};

use crate::ui_renderer::UIElementInstace;

const ROOT_ELEMENT: usize = 0;

pub struct UILayouter {
    // screen_size: Vec2,
    // elements: Vec<UIElement>,
}

impl UILayouter {
    pub fn new() -> Self {
        Self {
            // screen_size,
            // elements: vec![UIElement::root(screen_size.x, screen_size.y)],
        }
    }

    // pub fn add_element(&mut self, mut element: UIElement) {
    //     element.parent = ROOT_ELEMENT;
    //     self.elements.push(element);
    //
    //     let element_index = self.elements.len() - 1;
    //     self.elements[ROOT_ELEMENT].children.push(element_index);
    // }
    //
    // pub fn clear_elements(&mut self) {
    //     self.elements = vec![UIElement::root(self.screen_size.x, self.screen_size.y)];
    // }
    //
    // pub fn layout_elements(&mut self) {}
    // pub fn convert_elements(&self) -> Vec<UIElementInstace> {
    //     let mut instances = Vec::new();
    //     for element in self.elements.iter().skip(1) {
    //         instances.push(UIElementInstace {
    //             position: element.pos.to_array(),
    //             size: element.dimensions.to_array(),
    //             color: element.background_color.to_array(),
    //         });
    //     }
    //     instances
    // }

    pub fn layout_elements(
        &mut self,
        screen_size: Vec2,
        elements: Vec<UIElement>,
    ) -> Vec<UIElementInstace> {
        let root = UIElement {
            pos: Vec2::ZERO,
            dimensions: screen_size,
            background_color: Vec4::ZERO,
            parent: ROOT_ELEMENT,
            children: Vec::new(),
        };

        // layout

        // convert

        let mut instances = Vec::new();

        for element in elements.iter() {
            instances.push(UIElementInstace {
                position: element.pos.to_array(),
                size: element.dimensions.to_array(),
                color: element.background_color.to_array(),
            });
        }

        instances
    }
}

#[derive(Debug)]
pub struct UIElement {
    pos: Vec2,
    dimensions: Vec2,
    background_color: Vec4,

    parent: usize,
    children: Vec<usize>,
}

impl UIElement {
    pub fn new() -> Self {
        Self {
            pos: Vec2::ZERO,
            dimensions: Vec2::ZERO,
            background_color: Vec4::ZERO,
            parent: ROOT_ELEMENT,
            children: Vec::new(),
        }
    }
    // pub fn root(width: f32, height: f32) -> Self {
    //     Self {
    //         pos: Vec2::ZERO,
    //         dimensions: vec2(width, height),
    //         background_color: Vec4::ZERO,
    //         parent: 0,
    //         children: Vec::new(),
    //     }
    // }

    //
    // Attributes
    //

    pub fn pos(mut self, pos: Vec2) -> Self {
        self.pos = pos;
        self
    }
    pub fn dimensions(mut self, dimensions: Vec2) -> Self {
        self.dimensions = dimensions;
        self
    }
    pub fn background_color(mut self, background_color: Vec4) -> Self {
        self.background_color = background_color;
        self
    }

    //
    // Layout and interaction
    //

    pub fn draw_with_children(mut self, children: fn(&mut Self)) -> Self {
        children(&mut self);

        self.draw();

        self
    }

    pub fn draw(&mut self) {}
}
