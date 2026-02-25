use gbase::{
    glam::{f32, vec2, Vec2, Vec4},
    tracing,
    wgpu::naga::proc::Layouter,
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
        self.open_element(element);
        children(self);
        self.close_element();
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
    pub fn close_element(&mut self) {
        // TODO: could element stack be used instead of index?
        // self.element_stack.pop();
        // let element = &self.elems[elem_index];
        let element_index = self
            .element_stack
            .pop()
            .expect("element stack should never be empty when closing an element");
        let element = &self.elems[element_index];

        // calculate childrens combined size
        let mut children_width = 0.0;
        let mut children_height = 0.0;

        let child_count = element.children.len().saturating_sub(1);
        let total_child_gap = child_count as f32 * element.child_gap;
        match element.layout_direction {
            LayoutDirection::LeftToRight => {
                // gap
                children_width += total_child_gap;

                // accumulate children sizes
                for child_index in element.children.clone().into_iter() {
                    let child = &self.elems[child_index];
                    children_width += child.width;
                    children_height = f32::max(children_height, child.height);
                }
            }
            LayoutDirection::TopToBottom => {
                // gap
                children_height += total_child_gap;

                // accumulate children sizes
                for child_index in element.children.clone().into_iter() {
                    let child = &self.elems[child_index];
                    children_height += child.height;
                    children_width = f32::max(children_width, child.width);
                }
            }
        }

        // calculate fixed size
        let width = match element.sizing_x {
            Sizing::Fixed(fixed_width) => fixed_width,
            Sizing::Fit => children_width,
        };
        let height = match element.sizing_y {
            Sizing::Fixed(fixed_height) => fixed_height,
            Sizing::Fit => children_height,
        };

        // add padding
        let padding = element.padding.clone();
        let padding_left_right = padding.left + padding.right;
        let padding_top_bottom = padding.bottom + padding.top;

        let final_width = width + padding_left_right;
        let final_height = height + padding_top_bottom;
        self.elems[element_index].width = final_width;
        self.elems[element_index].height = final_height;
    }

    pub fn layout_elements(&mut self) -> Vec<UIElementInstace> {
        // NOTE: non fixed sizing pass

        // NOTE: positioning pass
        let mut stack = Vec::new();
        stack.push(ROOT_ELEMENT);

        // DFS from root
        while let Some(elem) = stack.pop() {
            let x = self.elems[elem].x + self.elems[elem].padding.left;
            let y = self.elems[elem].y + self.elems[elem].padding.top;

            let child_gap = self.elems[elem].child_gap;
            let mut offset = 0.0;

            match self.elems[elem].layout_direction {
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
    }
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
    layout_direction: LayoutDirection,

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
            layout_direction: LayoutDirection::LeftToRight,
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
        Self {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,

            sizing_x: Sizing::Fixed(width),
            sizing_y: Sizing::Fixed(height),
            parent: ROOT_ELEMENT,

            background_color: Vec4::ZERO,
            layout_direction: LayoutDirection::LeftToRight,
            padding: Padding {
                top: 0.0,
                bottom: 0.0,
                left: 0.0,
                right: 0.0,
            },
            children: Vec::new(),
            child_gap: 0.0,
        }
    }

    //
    // Attributes
    //

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

    //
    // Helpers
    //

    pub fn draw_with_children(self, layouter: &mut UILayouter, children: fn(&mut UILayouter)) {
        layouter.add_element(self, children);
    }
    pub fn draw(self, layouter: &mut UILayouter) {
        layouter.add_element(self, |_| {});
    }
}
