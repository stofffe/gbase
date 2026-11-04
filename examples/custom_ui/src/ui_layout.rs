use core::f32;

use crate::ui_font::TextMeasurer;
use gbase::{
    glam::{vec4, Vec4},
    render, Context,
};

pub struct UIElementId(usize);

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

    pub fn elements(&self) -> &[UIElement] {
        &self.elems
    }

    pub fn get_element(&self, handle: &UIElementId) -> &UIElement {
        &self.elems[handle.0]
    }

    pub fn add_element<F: FnOnce(&mut UILayouter) -> R, R>(
        &mut self,
        element: UIElement,
        children: F,
    ) -> (UIElementId, R) {
        let element_index = self.open_element(element);
        let children = children(self);
        self.close_element();
        (element_index, children)
    }

    /// Add element to stack and set parent as current top of stack
    fn open_element(&mut self, mut element: UIElement) -> UIElementId {
        // point to parent
        let parent = *self
            .element_stack
            .last()
            .expect("element stack can not be empty");
        element.parent = parent;

        // add element to global state
        self.elems.push(element);

        // add element to parents child list
        let element_index = self.elems.len() - 1;
        self.elems[parent].children.push(element_index);

        self.element_stack.push(element_index);

        UIElementId(element_index)
    }

    fn close_element(&mut self) {
        self.element_stack
            .pop()
            .expect("element stack should never be empty when closing an element");
    }

    /// Layout elements with the screen dimensions as the canvas size
    pub fn layout_elements_fullscreen(
        &mut self,
        ctx: &mut Context,
        text_measurer: &mut impl TextMeasurer,
    ) {
        let screen_size = render::surface_size(ctx);
        self.layout_elements(
            text_measurer,
            screen_size.width as f32,
            screen_size.height as f32,
        )
    }

    pub fn layout_elements(
        &mut self,
        text_measurer: &mut impl TextMeasurer,
        root_width: f32,
        root_height: f32,
    ) {
        self.elems[ROOT_ELEMENT].size[0] = root_width;
        self.elems[ROOT_ELEMENT].size[1] = root_height;

        self.text_width_pass(text_measurer);

        self.intrinsic_pass(Axis::X);

        self.fit_pass(Axis::X);

        self.percent_grow_shrink_pass(Axis::X);

        self.text_height_pass(text_measurer);

        self.intrinsic_pass(Axis::Y);

        self.fit_pass(Axis::Y);

        self.percent_grow_shrink_pass(Axis::Y);

        self.position_pass();
    }

    // TODO: only allow text size OR children size using some enum

    // Sets the intrinsic width by measuring the text width
    //
    /// Iteration order does not matter so do simplest DFS
    fn text_width_pass(&mut self, text_measurer: &mut impl TextMeasurer) {
        for elem in 1..self.elems.len() {
            let element = &self.elems[elem];

            if !element.text_info.text.is_empty() {
                let text_info = &element.text_info;

                let layout_result = text_measurer.calculate_preferred_text_size(
                    &text_info.text,
                    text_info.font_size,
                    false,
                );
                self.elems[elem].min[0] = layout_result.min_width;
                self.elems[elem].max[0] = layout_result.max_width;
            }
        }
    }

    // Sets the intrinsic height by measuring the text height by wrapping based
    // on the width set in previous passes
    //
    /// Iteration order does not matter so do simplest DFS
    fn text_height_pass(&mut self, text_measurer: &mut impl TextMeasurer) {
        for elem in 1..self.elems.len() {
            let element = &self.elems[elem];

            let element_text = &element.text_info.text;
            if !element_text.is_empty() {
                let layout_result = text_measurer.layout_text(
                    &element.text_info.text,
                    element.text_info.font_size,
                    element.size[0],
                );

                // // TODO: dont know if its need here
                self.elems[elem].min[1] = layout_result.height;
                self.elems[elem].max[1] = layout_result.height;
                self.elems[elem].text_layout = layout_result;
            }
        }
    }

    /// Sets the min/max sizes the elements wants (ignores any constraints)
    ///
    /// Iterates reverse DFS so children sizes are processed before processing the element itself
    fn intrinsic_pass(&mut self, axis: Axis) {
        let i = match axis {
            Axis::X => 0,
            Axis::Y => 1,
        };

        // reverse DFS so children are ready when processing element
        for elem in (1..self.elems.len()).rev() {
            let element = &self.elems[elem];

            // fixed overrides
            if let Sizing::Fixed(fixed_size) = element.sizing[i] {
                self.elems[elem].min[i] = fixed_size;
                self.elems[elem].max[i] = fixed_size;
                self.elems[elem].size[i] = fixed_size;
                continue;
            }

            let mut min_size = element.min[i];
            let mut max_size = element.max[i];

            let layout_dir_matches_axis = match axis {
                Axis::X => matches!(element.layout_direction, LayoutDirection::LeftToRight),
                Axis::Y => matches!(element.layout_direction, LayoutDirection::TopToBottom),
            };

            if layout_dir_matches_axis {
                // accumulate children sizes
                for &child_index in element.children.iter() {
                    let child = &self.elems[child_index];
                    min_size += child.min[i];
                    max_size += child.max[i];
                }
                // add gap
                let child_count = element.children.len().saturating_sub(1);
                let total_child_gap = child_count as f32 * element.child_gap;
                min_size += total_child_gap;
                max_size += total_child_gap;
            } else {
                // get max size of children
                for &child_index in element.children.iter() {
                    let child = &self.elems[child_index];
                    // TODO: unsure about this one
                    min_size = min_size.max(child.min[i]);
                    max_size = max_size.max(child.max[i]);
                }
            }

            // padding
            match axis {
                Axis::X => {
                    min_size += element.padding.horizontal();
                    max_size += element.padding.horizontal();
                }
                Axis::Y => {
                    min_size += element.padding.vertical();
                    max_size += element.padding.vertical();
                }
            };

            // set min and max, but actual size is not set
            self.elems[elem].min[i] = min_size;
            self.elems[elem].max[i] = max_size;
        }
    }

    /// Sets the sizing of elements with Sizing mode FIT
    ///
    /// Iteration order does not matter so do simplest DFS
    fn fit_pass(&mut self, axis: Axis) {
        let i = match axis {
            Axis::X => 0,
            Axis::Y => 1,
        };

        for elem in 1..self.elems.len() {
            let element = &self.elems[elem];

            if let Sizing::Fit = element.sizing[i] {
                self.elems[elem].size[i] = element.max[i];
            }
        }
    }

    /// Sets percent sizing, then grows or shrinks elements as needed
    /// Only Sizing::Grow are allowed to grow
    /// All elements with width > min_width are allowed to shrink
    ///
    /// Iterates DFS so parent is always processed before child
    fn percent_grow_shrink_pass(&mut self, axis: Axis) {
        let i = match axis {
            Axis::X => 0,
            Axis::Y => 1,
        };

        for elem in 0..self.elems.len() {
            let element = self.elems[elem].clone();

            let layout_dir_matches_axis = match axis {
                Axis::X => matches!(element.layout_direction, LayoutDirection::LeftToRight),
                Axis::Y => matches!(element.layout_direction, LayoutDirection::TopToBottom),
            };

            // percent on all children
            for &child in element.children.iter() {
                if let Sizing::Percent(p) = self.elems[child].sizing[i] {
                    let width = element.size[i] * p;

                    self.elems[child].size[i] = width;
                }
            }

            // calculate remaining width (used for grow/shrink)
            let mut remaining_width = element.size[i];
            remaining_width -= element.padding.horizontal();
            if layout_dir_matches_axis {
                remaining_width -=
                    (element.children.len().saturating_sub(1)) as f32 * element.child_gap;
                for &child in element.children.iter() {
                    remaining_width -= self.elems[child].size[i];
                }
            }

            // growable
            let mut growable = Vec::new();
            for &child in element.children.iter() {
                if let Sizing::Grow = self.elems[child].sizing[i] {
                    growable.push(child)
                }
            }
            if layout_dir_matches_axis {
                // distribute remaining width
                while remaining_width > 0.0 && !growable.is_empty() {
                    let mut smallest = f32::MAX;
                    let mut second_smallest = f32::MAX;
                    let mut smallest_count = 0;

                    for &child in growable.iter() {
                        let child_width = self.elems[child].size[i];
                        if child_width < smallest {
                            second_smallest = smallest;
                            smallest = child_width;
                            smallest_count = 1;
                        } else if child_width == smallest {
                            smallest_count += 1;
                        } else if child_width < second_smallest {
                            second_smallest = child_width;
                        }
                    }

                    // TODO: if max width is reached, pop from growable list

                    // if there are multiple with the smallest width and there is not enough
                    // to add to all of them, then distribute the remaining width
                    let width_to_make_equal = second_smallest - smallest;
                    let available_width_after_sharing = remaining_width / smallest_count as f32;
                    let width_to_add = width_to_make_equal.min(available_width_after_sharing);
                    for &child in growable.iter() {
                        let child_width = self.elems[child].size[i];
                        if child_width == smallest {
                            self.elems[child].size[0] += width_to_add;
                            remaining_width -= width_to_add;
                        }
                    }
                }
            } else {
                // fill up all available space
                for &child_index in growable.iter() {
                    self.elems[child_index].size[i] += element.size[i];
                }
            }

            // shrinkable
            let mut shrinkable = Vec::new();
            for &child in element.children.iter() {
                match self.elems[child].sizing[i] {
                    Sizing::Grow => shrinkable.push(child),
                    Sizing::Fit => shrinkable.push(child),
                    _ => (), // TODO: eventually add all that have min_width < width
                }
            }
            if layout_dir_matches_axis {
                // shrink to fit overflowing width
                while remaining_width < 0.0 && !shrinkable.is_empty() {
                    let mut largest = f32::MIN;
                    let mut second_largest = f32::MIN;
                    let mut largest_count = 0;

                    for &child in shrinkable.iter() {
                        let child_width = self.elems[child].size[i];
                        if child_width > largest {
                            second_largest = largest;
                            largest = child_width;
                            largest_count = 1;
                        } else if child_width == largest {
                            largest_count += 1;
                        } else if child_width > second_largest {
                            second_largest = child_width;
                        }
                    }

                    // if there are multiple with the largest width and there is not enough
                    // to add to all of them, then distribute the remaining width
                    let width_to_make_equal = second_largest - largest; // negative
                    let available_width_after_sharing = remaining_width / largest_count as f32; // negative
                    let width_to_add = width_to_make_equal.max(available_width_after_sharing); // negative
                    shrinkable.retain(|&child_index| {
                        let child = &self.elems[child_index];
                        let initial_width = child.size[i];

                        let mut at_min_width = false;
                        if child.size[i] == largest {
                            let mut new_width = child.size[i] + width_to_add; // basically a subtraciton
                            if new_width <= child.min[i] {
                                new_width = child.min[i];
                                at_min_width = true;
                            }

                            let removed_width = new_width - initial_width;
                            remaining_width -= removed_width;

                            self.elems[child_index].size[i] = new_width;
                        }

                        !at_min_width
                    });
                }
            } else {
                for &child_index in shrinkable.iter() {
                    let child = &mut self.elems[child_index];

                    if remaining_width < 0.0 {
                        let new_width = child.size[i] + remaining_width;
                        if new_width < child.min[i] {
                            self.elems[child_index].size[i] = child.min[i];
                        } else {
                            self.elems[child_index].size[i] = new_width;
                        }
                    }
                }
            }
        }
    }

    /// Positions elements relative to their parent element on both x and y axis
    ///
    /// Iterates DFS so parent is always processed before child
    fn position_pass(&mut self) {
        for elem in 1..self.elems.len() {
            let element = &self.elems[elem];

            let x = element.pos[0] + element.padding.left;
            let y = element.pos[1] + element.padding.top;

            let child_gap = element.child_gap;
            let mut offset = 0.0;

            match element.layout_direction {
                LayoutDirection::LeftToRight => {
                    for child in element.children.clone().into_iter() {
                        self.elems[child].pos[0] = x + offset;
                        self.elems[child].pos[1] = y;
                        offset += self.elems[child].size[0] + child_gap;
                    }
                }
                LayoutDirection::TopToBottom => {
                    for child in element.children.clone().into_iter() {
                        self.elems[child].pos[0] = x;
                        self.elems[child].pos[1] = y + offset;
                        offset += self.elems[child].size[1] + child_gap;
                    }
                }
            }
        }
    }
}

enum Axis {
    X,
    Y,
}

#[derive(Debug, Clone)]
pub enum Content {
    Container,
    Text(TextInfo),
}

#[derive(Debug, Clone)]
pub struct TextInfo {
    pub text: String,

    pub text_color: Vec4,
    pub font_size: u32,
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

#[derive(Debug, Clone)]
pub struct UIElement {
    // set fields
    sizing: [Sizing; 2],

    padding: Padding,

    child_gap: f32,

    pub background_color: Vec4,

    layout_direction: LayoutDirection,

    pub text_info: TextInfo,

    // gets overwritten when text is used
    min_override: [Option<f32>; 2],
    max_override: [Option<f32>; 2],

    // calculated fields
    min: [f32; 2],
    max: [f32; 2],

    pub size: [f32; 2],
    pub pos: [f32; 2],

    pub text_layout: TextLayoutResult,

    pub parent: usize,
    pub children: Vec<usize>,
}

impl UIElement {
    pub fn new() -> Self {
        Self {
            sizing: [Sizing::Fit, Sizing::Fit],
            padding: Padding {
                top: 0.0,
                bottom: 0.0,
                left: 0.0,
                right: 0.0,
            },
            layout_direction: LayoutDirection::LeftToRight,
            background_color: vec4(0.0, 0.0, 0.0, 1.0),
            child_gap: 0.0,

            text_info: TextInfo {
                text: String::new(),
                text_color: vec4(0.0, 0.0, 0.0, 1.0),
                font_size: 12,
            },

            min_override: [None, None],
            max_override: [None, None],

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
        self.text_info.text = text.into();
        self
    }
    pub fn font_size(mut self, font_size: u32) -> Self {
        self.text_info.font_size = font_size;
        self
    }
    pub fn text_color(mut self, text_color: Vec4) -> Self {
        self.text_info.text_color = text_color;
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

#[cfg(test)]
mod test {
    use crate::{
        ui_font::TextMeasurer,
        ui_layout::{
            LayoutDirection::LeftToRight,
            Sizing::{Fit, Fixed, Grow},
            UIElement, UILayouter,
        },
    };

    struct TestTextMeasurer {}
    impl TextMeasurer for TestTextMeasurer {
        fn calculate_preferred_text_size(
            &mut self,
            text: &str,
            font_size: u32,
            wrap_on_newline: bool,
        ) -> super::TextSizeResult {
            todo!()
        }

        fn layout_text(
            &mut self,
            text: &str,
            font_size: u32,
            max_width: f32,
        ) -> super::TextLayoutResult {
            todo!()
        }
    }

    #[test]
    fn multiple_top_level() {
        let mut layouter = UILayouter::new();
        let mut measurer = TestTextMeasurer {};

        let elem1 = UIElement::new().sizing_x(Fixed(300.0)).draw(&mut layouter);
        let elem2 = UIElement::new().sizing_x(Fixed(200.0)).draw(&mut layouter);

        layouter.layout_elements(&mut measurer, 1000.0, 1000.0);

        assert_eq!(layouter.get_element(&elem1).size[0], 300.0);
        assert_eq!(layouter.get_element(&elem2).size[0], 200.0);
    }

    //
    // Fixed
    //

    #[test]
    fn fixed() {
        let mut layouter = UILayouter::new();
        let mut measurer = TestTextMeasurer {};

        let elem1 = UIElement::new()
            .sizing_x(Fixed(200.0))
            .sizing_y(Fixed(100.0))
            .draw(&mut layouter);

        layouter.layout_elements(&mut measurer, 500.0, 500.0);

        assert_eq!(layouter.get_element(&elem1).size[0], 200.0);
        assert_eq!(layouter.get_element(&elem1).size[1], 100.0);
    }

    #[test]
    fn fixed_overflow_stays_fixed() {
        let mut layouter = UILayouter::new();
        let mut measurer = TestTextMeasurer {};

        let elem1 = UIElement::new()
            .sizing_x(Fixed(400.0))
            .sizing_y(Fixed(200.0))
            .draw(&mut layouter);

        layouter.layout_elements(&mut measurer, 300.0, 300.0);

        assert_eq!(layouter.get_element(&elem1).size[0], 400.0);
        assert_eq!(layouter.get_element(&elem1).size[1], 200.0);
    }

    //
    // Fit
    //

    #[test]
    fn fit_with_fixed_inside() {
        let mut layouter = UILayouter::new();
        let mut measurer = TestTextMeasurer {};

        let (elem1, _) = UIElement::new()
            .layout_direction(LeftToRight)
            .sizing_x(Fit)
            .sizing_y(Fit)
            .draw_with_children(&mut layouter, |layouter| {
                UIElement::new()
                    .sizing_x(Fixed(200.0))
                    .sizing_y(Fixed(250.0))
                    .draw(layouter);
            });

        layouter.layout_elements(&mut measurer, 1000.0, 1000.0);

        assert_eq!(layouter.get_element(&elem1).size[0], 200.0);
        assert_eq!(layouter.get_element(&elem1).size[1], 250.0);
    }

    #[test]
    fn fit_with_no_children() {
        let mut layouter = UILayouter::new();
        let mut measurer = TestTextMeasurer {};

        let elem1 = UIElement::new()
            .sizing_x(Fit)
            .sizing_y(Fit)
            .draw(&mut layouter);

        layouter.layout_elements(&mut measurer, 1000.0, 1000.0);

        assert_eq!(layouter.get_element(&elem1).size[0], 0.0);
        assert_eq!(layouter.get_element(&elem1).size[1], 0.0);
    }

    #[test]
    fn fit_with_grow_inside() {
        let mut layouter = UILayouter::new();
        let mut measurer = TestTextMeasurer {};

        let (elem1, _) = UIElement::new()
            .sizing_x(Fit)
            .sizing_y(Fit)
            .draw_with_children(&mut layouter, |layouter| {
                UIElement::new()
                    .sizing_x(Grow)
                    .sizing_y(Grow)
                    .draw(layouter);
            });

        layouter.layout_elements(&mut measurer, 1000.0, 1000.0);

        assert_eq!(layouter.get_element(&elem1).size[0], 0.0);
        assert_eq!(layouter.get_element(&elem1).size[1], 0.0);
    }

    #[test]
    fn fit_with_fit_with_fixed_inside() {
        let mut layouter = UILayouter::new();
        let mut measurer = TestTextMeasurer {};

        let (elem1, _) = UIElement::new()
            .sizing_x(Fit)
            .sizing_y(Fit)
            .draw_with_children(&mut layouter, |layouter| {
                UIElement::new()
                    .sizing_x(Fit)
                    .draw_with_children(layouter, |layouter| {
                        UIElement::new()
                            .sizing_x(Fixed(100.0))
                            .sizing_y(Fixed(200.0))
                            .draw(layouter);
                    });
            });

        layouter.layout_elements(&mut measurer, 1000.0, 1000.0);

        assert_eq!(layouter.get_element(&elem1).size[0], 100.0);
        assert_eq!(layouter.get_element(&elem1).size[1], 200.0);
    }

    //
    // Grow
    //

    #[test]
    fn grow_at_root() {
        let mut layouter = UILayouter::new();
        let mut measurer = TestTextMeasurer {};

        let elem1 = UIElement::new()
            .sizing_x(Grow)
            .sizing_y(Grow)
            .draw(&mut layouter);

        layouter.layout_elements(&mut measurer, 1000.0, 1000.0);

        assert_eq!(layouter.get_element(&elem1).size[0], 1000.0);
        assert_eq!(layouter.get_element(&elem1).size[1], 1000.0);
    }

    #[test]
    fn grow_inside_fixed() {
        let mut layouter = UILayouter::new();
        let mut measurer = TestTextMeasurer {};

        let (_, elem2) = UIElement::new()
            .sizing_x(Fixed(200.0))
            .sizing_y(Fixed(100.0))
            .draw_with_children(&mut layouter, |layouter| {
                UIElement::new()
                    .sizing_x(Grow)
                    .sizing_y(Grow)
                    .draw(layouter)
            });

        layouter.layout_elements(&mut measurer, 1000.0, 1000.0);

        assert_eq!(layouter.get_element(&elem2).size[0], 200.0);
        assert_eq!(layouter.get_element(&elem2).size[1], 100.0);
    }

    #[test]
    fn grow_distribution_main_axis() {
        let mut layouter = UILayouter::new();
        let mut measurer = TestTextMeasurer {};

        let (_, (elem2, elem3)) = UIElement::new()
            .layout_direction(LeftToRight)
            .sizing_x(Fixed(400.0))
            .draw_with_children(&mut layouter, |layouter| {
                let elem2 = UIElement::new().sizing_x(Grow).draw(layouter);
                let elem3 = UIElement::new().sizing_x(Grow).draw(layouter);
                (elem2, elem3)
            });

        layouter.layout_elements(&mut measurer, 1000.0, 1000.0);

        assert_eq!(layouter.get_element(&elem2).size[0], 200.0);
        assert_eq!(layouter.get_element(&elem3).size[0], 200.0);
    }

    #[test]
    fn grow_distribution_cross_axis() {
        let mut layouter = UILayouter::new();
        let mut measurer = TestTextMeasurer {};

        let (_, (elem2, elem3)) = UIElement::new()
            .layout_direction(LeftToRight)
            .sizing_y(Fixed(300.0))
            .draw_with_children(&mut layouter, |layouter| {
                let elem2 = UIElement::new().sizing_y(Grow).draw(layouter);
                let elem3 = UIElement::new().sizing_y(Grow).draw(layouter);
                (elem2, elem3)
            });

        layouter.layout_elements(&mut measurer, 1000.0, 1000.0);

        assert_eq!(layouter.get_element(&elem2).size[1], 300.0);
        assert_eq!(layouter.get_element(&elem3).size[1], 300.0);
    }

    //
    // Shrink
    //
}
