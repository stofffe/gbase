use std::collections::VecDeque;

use gbase::{
    glam::{f32, vec2, Vec2, Vec4},
    tracing,
};

use crate::ui_renderer::UIElementInstace;

const ROOT_ELEMENT: usize = 0;

pub struct UILayouter {
    screen_size: Vec2,
    elems: Vec<UIElement>,

    element_stack: Vec<usize>,
}

impl UILayouter {
    pub fn new(screen_size: Vec2) -> Self {
        let mut layouter = Self {
            screen_size,
            elems: Vec::new(),
            element_stack: Vec::new(),
        };
        layouter.reset();
        layouter
    }

    pub fn reset(&mut self) {
        self.elems = vec![UIElement::root(self.screen_size.x, self.screen_size.y)];
        self.element_stack = vec![ROOT_ELEMENT];
    }

    pub fn add_element(&mut self, element: UIElement, children: fn(&mut Self)) {
        let index = self.open_element(element);
        children(self);
        self.close_element(index);
    }

    pub fn open_element(&mut self, mut element: UIElement) -> usize {
        // point to parent
        let parent = *self
            .element_stack
            .last()
            .expect("element stack can not be empty");
        element.parent = parent;

        // add element to global state
        self.elems.push(element);

        // add element to parents children
        let element_index = self.elems.len() - 1;
        self.elems[parent].children.push(element_index);

        self.element_stack.push(element_index);

        element_index
    }

    // NOTE: intrinsic size pass
    //
    // TODO: percent should set dimensions to 0?
    // clamp values?
    pub fn close_element(&mut self, elem: usize) {
        // padding
        let padding = self.elems[elem].padding.clone();
        let padding_left_right = padding.left + padding.right;
        let padding_top_bottom = padding.bottom + padding.top;
        self.elems[elem].width = padding_left_right;
        self.elems[elem].height = padding_top_bottom;

        // huh
        match self.elems[elem].sizing_x {
            Sizing::Fixed(width) => self.elems[elem].width += width,
            Sizing::Fit => {}
        }
        match self.elems[elem].sizing_y {
            Sizing::Fixed(height) => self.elems[elem].height += height,
            Sizing::Fit => {}
        }

        // gap size
        let child_count = self.elems[elem].children.len().saturating_sub(1);
        let total_child_gap = child_count as f32 * self.elems[elem].child_gap;

        match self.elems[elem].child_direction {
            LayoutDirection::LeftToRight => {
                // gap
                self.elems[elem].width += total_child_gap;

                // accumulate children sizes
                for child in self.elems[elem].children.clone().into_iter() {
                    self.elems[elem].width += self.elems[child].width;
                    self.elems[elem].height = f32::max(
                        self.elems[elem].height,
                        self.elems[child].height + padding_top_bottom,
                    );
                }
            }
            LayoutDirection::TopToBottom => {
                // gap
                self.elems[elem].height += total_child_gap;

                // accumulate children sizes
                for child in self.elems[elem].children.clone().into_iter() {
                    self.elems[elem].height += self.elems[child].height;
                    self.elems[elem].width = f32::max(
                        self.elems[elem].width,
                        self.elems[child].width + padding_left_right,
                    );
                }
            }
        }

        self.element_stack.pop();
    }

    pub fn layout_elements(&mut self) -> Vec<UIElementInstace> {
        // NOTE: non fixed sizing pass

        // NOTE: positioning pass
        let mut stack = Vec::new();
        stack.push(ROOT_ELEMENT);

        while let Some(elem) = stack.pop() {
            let x = self.elems[elem].x + self.elems[elem].padding.left;
            let y = self.elems[elem].y + self.elems[elem].padding.top;

            let child_gap = self.elems[elem].child_gap;
            let mut offset = 0.0;

            match self.elems[elem].child_direction {
                LayoutDirection::LeftToRight => {
                    for child in self.elems[elem].children.clone().into_iter() {
                        self.elems[child].x = x + offset;

                        offset += self.elems[child].width + child_gap;

                        tracing::error!("add {}", child);
                        stack.push(child);
                    }
                }
                LayoutDirection::TopToBottom => {
                    for child in self.elems[elem].children.clone().into_iter() {
                        self.elems[child].y = y + offset;
                        offset += self.elems[child].height + child_gap;

                        tracing::error!("add {}", child);
                        stack.push(child);
                    }
                }
            }
        }

        // convert
        let mut instances = Vec::new();
        for elem in self.elems.iter().skip(1) {
            tracing::error!("draw {:?}", elem);
            instances.push(UIElementInstace {
                position: [elem.x, elem.y],
                size: [elem.width, elem.height],
                color: elem.background_color.to_array(),
            });
        }
        instances

        // self.layout_size();
        // self.layout_position();
        // self.convert()
    }

    fn layout_size(&mut self) {}

    fn layout_position(&mut self) {}

    fn convert(&mut self) -> Vec<UIElementInstace> {
        let mut instances = Vec::new();
        for elem in self.elems.iter().skip(1) {
            instances.push(UIElementInstace {
                position: [elem.x, elem.y],
                size: [elem.width, elem.height],
                color: elem.background_color.to_array(),
            });
        }
        instances
    }

    //     pub fn layout_elements(
    //         &mut self,
    //         screen_size: Vec2,
    //         elements: Vec<UIElement>,
    //     ) -> Vec<UIElementInstace> {
    //         let root = UIElement {
    //             pos: Vec2::ZERO,
    //             dimensions: screen_size,
    //             background_color: Vec4::ZERO,
    //             parent: ROOT_ELEMENT,
    //             children: Vec::new(),
    //         };
    //
    //         // layout
    //
    //         // convert
    //
    //         let mut instances = Vec::new();
    //
    //         for element in elements.iter() {
    //             instances.push(UIElementInstace {
    //                 position: element.pos.to_array(),
    //                 size: element.dimensions.to_array(),
    //                 color: element.background_color.to_array(),
    //             });
    //         }
    //
    //         instances
    //     }
}

#[derive(Debug, Clone)]
pub enum Sizing {
    Fixed(f32),
    Fit,
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
    fn horizontal(&self) -> f32 {
        self.left + self.right
    }
    fn vertical(&self) -> f32 {
        self.bottom + self.top
    }
}

#[derive(Debug, Clone)]
pub struct UIElement {
    x: f32,
    y: f32,
    width: f32,
    height: f32,

    padding: Padding,

    sizing_x: Sizing,
    sizing_y: Sizing,

    background_color: Vec4,
    child_direction: LayoutDirection,

    parent: usize,
    children: Vec<usize>,
    child_gap: f32,
}

impl UIElement {
    pub fn new() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
            background_color: Vec4::ZERO,
            sizing_x: Sizing::Fit,
            sizing_y: Sizing::Fit,
            child_direction: LayoutDirection::LeftToRight,
            padding: Padding {
                top: 0.0,
                bottom: 0.0,
                left: 0.0,
                right: 0.0,
            },
            parent: ROOT_ELEMENT,
            children: Vec::new(),
            child_gap: 0.0,
        }
    }
    pub fn root(width: f32, height: f32) -> Self {
        Self::new().dimensions(width, height)
    }

    //
    // Attributes
    //

    pub fn pos(mut self, x: f32, y: f32) -> Self {
        self.x = x;
        self.y = y;
        self
    }
    pub fn dimensions(mut self, width: f32, height: f32) -> Self {
        self.width = width;
        self.height = height;
        self
    }
    pub fn sizing_x(mut self, sizing: Sizing) -> Self {
        self.sizing_x = sizing;
        self
    }
    pub fn sizing_y(mut self, sizing: Sizing) -> Self {
        self.sizing_y = sizing;
        self
    }
    pub fn background_color(mut self, background_color: Vec4) -> Self {
        self.background_color = background_color;
        self
    }
}
