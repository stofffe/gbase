use gbase::{
    glam::{f32, Vec4},
    render, Context,
};

pub trait UILayoutTextMeasurer {
    fn measure_text(&self, text: &str, max_width: f32) -> (f32, f32);
}

use crate::ui_renderer::UIElementInstace;

const ROOT_ELEMENT: usize = 0;

pub struct UILayouter {
    elems: Vec<UIElement>,
    element_stack: Vec<usize>,
}

impl UILayouter {
    pub fn new() -> Self {
        let mut layouter = Self {
            elems: Vec::new(),
            element_stack: Vec::new(),
        };
        layouter.reset();
        layouter
    }

    pub fn reset(&mut self) {
        self.elems = vec![UIElement::new()];
        self.element_stack = vec![ROOT_ELEMENT];
    }

    pub fn add_element(&mut self, element: UIElement, children: fn(&mut Self)) {
        self.open_element(element);
        children(self);
        self.close_element();
    }

    fn open_element(&mut self, mut element: UIElement) -> usize {
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

    fn close_element(&mut self) {
        self.element_stack
            .pop()
            .expect("element stack should never be empty when closing an element");
    }

    pub fn layout_elements_fullscreen(
        &mut self,
        ctx: &Context,
        text_measurer: &impl UILayoutTextMeasurer,
    ) -> Vec<UIElementInstace> {
        let screen_size = render::surface_size(ctx);
        self.layout_elements(
            screen_size.width as f32,
            screen_size.height as f32,
            text_measurer,
        )
    }
    pub fn layout_elements(
        &mut self,
        root_width: f32,
        root_height: f32,
        text_measurer: &impl UILayoutTextMeasurer,
    ) -> Vec<UIElementInstace> {
        self.elems[ROOT_ELEMENT].width = root_width;
        self.elems[ROOT_ELEMENT].height = root_height;

        //
        // intrinsic sizes
        //

        // TODO: clamp values?
        for elem in (1..self.elems.len()).rev() {
            let element = &self.elems[elem];

            // text size
            let (mut width, mut height) = match element.content {
                Content::Container => (0.0, 0.0),
                Content::Text(ref text_info) => {
                    text_measurer.measure_text(&text_info.text, element.max_width)
                }
            };

            // overwrite if fixed
            if let Sizing::Fixed(fixed_width) = element.sizing_x {
                width = fixed_width;
            }
            if let Sizing::Fixed(fixed_height) = element.sizing_y {
                height = fixed_height;
            }

            // add padding
            width += element.padding.horizontal();
            height += element.padding.vertical();

            self.elems[elem].width = width;
            self.elems[elem].height = height;
        }

        //
        // fit x
        //
        // iterate backward to visit children before parents
        for elem in (1..self.elems.len()).rev() {
            let element = self.elems[elem].clone();
            if !matches!(element.sizing_x, Sizing::Fit) {
                continue;
            }

            // calculate childrens combined size
            let child_count = element.children.len().saturating_sub(1);
            let total_child_gap = child_count as f32 * element.child_gap;

            let mut children_width = 0.0;
            match element.layout_direction {
                LayoutDirection::LeftToRight => {
                    // gap
                    children_width += total_child_gap;

                    // accumulate children sizes
                    for &child_index in element.children.iter() {
                        let child = &self.elems[child_index];
                        children_width += child.width;
                    }
                }
                LayoutDirection::TopToBottom => {
                    // get max width of children
                    for &child_index in element.children.iter() {
                        let child = &self.elems[child_index];
                        children_width = children_width.max(child.width);
                    }
                }
            }

            self.elems[elem].width += children_width;
        }

        //
        // grow x
        //

        for elem in 0..self.elems.len() {
            // fix sizing
            let element = self.elems[elem].clone();

            // resolve percent
            for &child in element.children.iter() {
                if let Sizing::Percent(p) = self.elems[child].sizing_x {
                    self.elems[child].width = element.width * p;
                }
            }

            // extract growable
            let mut growable = Vec::new();
            for &child in element.children.iter() {
                if let Sizing::Grow = self.elems[child].sizing_x {
                    growable.push(child);
                }
            }

            if growable.is_empty() {
                continue;
            }

            match element.layout_direction {
                LayoutDirection::LeftToRight => {
                    // calculate remaining width
                    let mut remaining_width = element.width;
                    remaining_width -= element.padding.horizontal();
                    remaining_width -=
                        (element.children.len().saturating_sub(1)) as f32 * element.child_gap;
                    for &child in element.children.iter() {
                        remaining_width -= self.elems[child].width;
                    }

                    // distribute remaining width
                    while remaining_width > 0.0 {
                        let mut smallest = f32::MAX;
                        let mut second_smallest = f32::MAX;
                        let mut width_to_add = remaining_width;

                        for &child in growable.iter() {
                            let child_width = self.elems[child].width;
                            if child_width < smallest {
                                second_smallest = smallest;
                                smallest = child_width;
                            }
                            if child_width > smallest && child_width < second_smallest {
                                second_smallest = child_width;
                                width_to_add = second_smallest - remaining_width;
                            }
                        }

                        width_to_add = width_to_add.min(remaining_width / growable.len() as f32);

                        for &child in growable.iter() {
                            let child_width = self.elems[child].width;
                            if child_width == smallest {
                                self.elems[child].width += width_to_add;
                                remaining_width -= width_to_add;
                            }
                        }
                    }
                }
                LayoutDirection::TopToBottom => {
                    let remaining_width = element.width - element.padding.horizontal();
                    for &child in growable.iter() {
                        self.elems[child].width = remaining_width;
                    }
                }
            }
        }

        //
        // fit y
        //
        // iterate backward to visit children before parents
        for elem in (1..self.elems.len()).rev() {
            let element = self.elems[elem].clone();
            if !matches!(element.sizing_y, Sizing::Fit) {
                continue;
            }

            // calculate childrens combined size
            let child_count = element.children.len().saturating_sub(1);
            let total_child_gap = child_count as f32 * element.child_gap;

            let mut children_height = 0.0;
            match element.layout_direction {
                LayoutDirection::TopToBottom => {
                    // gap
                    children_height += total_child_gap;

                    // accumulate children sizes
                    for &child_index in element.children.iter() {
                        let child = &self.elems[child_index];
                        children_height += child.height;
                    }
                }
                LayoutDirection::LeftToRight => {
                    // get max height of children
                    for &child_index in element.children.iter() {
                        let child = &self.elems[child_index];
                        children_height = children_height.max(child.height);
                    }
                }
            }

            self.elems[elem].height += children_height;
        }

        //
        // grow y
        //

        for elem in 0..self.elems.len() {
            let element = self.elems[elem].clone();

            // resolve percent
            for &child in element.children.iter() {
                if let Sizing::Percent(p) = self.elems[child].sizing_y {
                    self.elems[child].height = element.height * p;
                }
            }

            // extract growable
            let mut growable = Vec::new();
            for &child in element.children.iter() {
                if matches!(self.elems[child].sizing_y, Sizing::Grow) {
                    growable.push(child);
                }
            }
            if growable.is_empty() {
                continue;
            }

            match element.layout_direction {
                LayoutDirection::TopToBottom => {
                    // calculate remaining width
                    let mut remaining_height = element.height;
                    remaining_height -= element.padding.vertical();
                    remaining_height -=
                        (element.children.len().saturating_sub(1)) as f32 * element.child_gap;
                    for &child in element.children.iter() {
                        remaining_height -= self.elems[child].height;
                    }

                    // distribute remaining height
                    while remaining_height > 0.0 {
                        let mut smallest = f32::MAX;
                        let mut second_smallest = f32::MAX;
                        let mut height_to_add = remaining_height;

                        for &child in growable.iter() {
                            let child_height = self.elems[child].height;
                            if child_height < smallest {
                                second_smallest = smallest;
                                smallest = child_height;
                            }
                            if child_height > smallest && child_height < second_smallest {
                                second_smallest = child_height;
                                height_to_add = second_smallest - remaining_height;
                            }
                        }

                        height_to_add = height_to_add.min(remaining_height / growable.len() as f32);

                        for &child in growable.iter() {
                            let child_height = self.elems[child].height;
                            if child_height == smallest {
                                self.elems[child].height += height_to_add;
                                remaining_height -= height_to_add;
                            }
                        }
                    }
                }
                LayoutDirection::LeftToRight => {
                    let remaining_height = element.height - element.padding.vertical();
                    for &child in growable.iter() {
                        self.elems[child].height = remaining_height;
                    }
                }
            }
        }

        //
        // positioning
        //
        for elem in 1..self.elems.len() {
            let element = &self.elems[elem];

            let x = element.x + element.padding.left;
            let y = element.y + element.padding.top;

            let child_gap = element.child_gap;
            let mut offset = 0.0;

            match element.layout_direction {
                LayoutDirection::LeftToRight => {
                    for child in element.children.clone().into_iter() {
                        self.elems[child].x = x + offset;
                        self.elems[child].y = y;
                        offset += self.elems[child].width + child_gap;
                    }
                }
                LayoutDirection::TopToBottom => {
                    for child in element.children.clone().into_iter() {
                        self.elems[child].x = x;
                        self.elems[child].y = y + offset;
                        offset += self.elems[child].height + child_gap;
                    }
                }
            }
        }

        // convert
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
}

#[derive(Debug, Clone)]
pub enum Content {
    Container,
    Text(TextInfo),
}

#[derive(Debug, Clone)]
pub struct TextInfo {
    text: String,

    text_color: Vec4,
    font_size: f32,
}

#[derive(Debug, Clone)]
pub enum Sizing {
    Fixed(f32),
    Fit,
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

#[derive(Debug, Clone)]
pub struct UIElement {
    // set fields
    sizing_x: Sizing,
    sizing_y: Sizing,

    padding: Padding,

    background_color: Vec4,

    layout_direction: LayoutDirection,

    child_gap: f32,

    content: Content,

    max_width: f32,
    min_width: f32,

    // calculated fields
    x: f32,
    y: f32,
    width: f32,
    height: f32,

    parent: usize,
    children: Vec<usize>,
}

impl UIElement {
    pub fn new() -> Self {
        Self {
            sizing_x: Sizing::Fit,
            sizing_y: Sizing::Fit,
            padding: Padding {
                top: 0.0,
                bottom: 0.0,
                left: 0.0,
                right: 0.0,
            },
            layout_direction: LayoutDirection::LeftToRight,
            background_color: Vec4::ZERO,
            child_gap: 0.0,
            content: Content::Container,

            max_width: 0.0,
            min_width: 0.0,

            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
            parent: ROOT_ELEMENT,
            children: Vec::new(),
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
