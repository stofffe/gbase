use crate::ui_font::TextMeasurer;
use gbase::{
    glam::{vec4, Vec4},
    render, Context,
};

pub struct UIElementId(usize);

#[derive(Debug)]
pub struct TextSizeResult {
    pub preferred_width: f32,
    pub preferred_height: f32,
    pub min_width: f32,
    pub min_height: f32,
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
        self.elems[ROOT_ELEMENT].preferred_width = root_width;
        self.elems[ROOT_ELEMENT].preferred_height = root_height;

        //
        // intrinsic/fit x
        //

        // TODO: clamp values?
        for elem in (1..self.elems.len()).rev() {
            let element = &self.elems[elem];

            let mut preferred_width = 0.0f32;
            let mut min_width = 0.0f32;

            // fixed sizes already set?

            // text size
            if !element.text_info.text.is_empty() {
                let text_info = &element.text_info;

                let layout_result = text_measurer.calculate_preferred_text_size(
                    &text_info.text,
                    text_info.font_size,
                    false,
                );
                preferred_width += layout_result.preferred_width;
                min_width = layout_result.min_width;
            }

            // children sizes
            let mut children_preferred_width = 0.0;
            let mut children_min_width = 0.0;
            match element.layout_direction {
                LayoutDirection::LeftToRight => {
                    // accumulate children sizes
                    for &child_index in element.children.iter() {
                        let child = &self.elems[child_index];
                        children_preferred_width += child.preferred_width;
                        children_min_width += child.min_width;
                    }
                }
                LayoutDirection::TopToBottom => {
                    // get max width of children
                    for &child_index in element.children.iter() {
                        let child = &self.elems[child_index];
                        children_preferred_width =
                            children_preferred_width.max(child.preferred_width);
                        children_min_width = children_min_width.max(child.min_width);
                    }
                }
            }
            preferred_width += children_preferred_width;
            min_width += children_min_width;

            // gap
            match element.layout_direction {
                LayoutDirection::LeftToRight => {
                    let child_count = element.children.len().saturating_sub(1);
                    let total_child_gap = child_count as f32 * element.child_gap;
                    preferred_width += total_child_gap;
                    min_width += total_child_gap;
                }
                LayoutDirection::TopToBottom => {
                    // dont do anything since we are in the x pass
                }
            }

            // padding
            preferred_width += element.padding.horizontal();
            min_width += element.padding.horizontal();

            // TODO: early continue and do this instead?
            // fixed size overwrite
            if let Sizing::Fixed(fixed_width) = element.sizing_x {
                preferred_width = fixed_width;
                min_width = fixed_width;
            }

            self.elems[elem].preferred_width = preferred_width;
            self.elems[elem].min_width = min_width;
        }

        //
        // percent/grow/shrink x
        //

        for elem in 0..self.elems.len() {
            let element = self.elems[elem].clone();

            // percent
            for &child in element.children.iter() {
                if let Sizing::Percent(p) = self.elems[child].sizing_x {
                    let width = element.preferred_width * p;

                    self.elems[child].preferred_width = width;
                    self.elems[child].min_width = width;
                }
            }

            // calculate remaining width (used for grow/shrink)
            let mut remaining_width = element.preferred_width;
            remaining_width -= element.padding.horizontal();
            if let LayoutDirection::LeftToRight = element.layout_direction {
                remaining_width -=
                    (element.children.len().saturating_sub(1)) as f32 * element.child_gap;
                for &child in element.children.iter() {
                    remaining_width -= self.elems[child].preferred_width;
                }
            }

            // growable
            let mut growable = Vec::new();
            for &child in element.children.iter() {
                if let Sizing::Grow = self.elems[child].sizing_x {
                    growable.push(child);
                }
            }
            match element.layout_direction {
                LayoutDirection::LeftToRight => {
                    // distribute remaining width
                    while remaining_width > 0.0 && !growable.is_empty() {
                        let mut smallest = f32::MAX;
                        let mut second_smallest = f32::MAX;
                        let mut smallest_count = 0;

                        for &child in growable.iter() {
                            let child_width = self.elems[child].preferred_width;
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

                        // if there are multiple with the smallest width and there is not enough
                        // to add to all of them, then distribute the remaining width
                        let width_to_make_equal = second_smallest - smallest;
                        let available_width_after_sharing = remaining_width / smallest_count as f32;
                        let width_to_add = width_to_make_equal.min(available_width_after_sharing);
                        for &child in growable.iter() {
                            let child_width = self.elems[child].preferred_width;
                            if child_width == smallest {
                                self.elems[child].preferred_width += width_to_add;
                                remaining_width -= width_to_add;
                            }
                        }
                    }
                }
                LayoutDirection::TopToBottom => {
                    // fill up all available space
                    for &child_index in growable.iter() {
                        self.elems[child_index].preferred_width = remaining_width;
                    }
                }
            }

            // shrinkable
            let mut shrinkable = Vec::new();
            for &child in element.children.iter() {
                let sizing = &self.elems[child].sizing_x;
                if matches!(sizing, Sizing::Grow | Sizing::Fit) {
                    shrinkable.push(child);
                }
            }
            match element.layout_direction {
                LayoutDirection::LeftToRight => {
                    // shrink to fit overflowing width
                    while remaining_width < 0.0 && !shrinkable.is_empty() {
                        let mut largest = f32::MIN;
                        let mut second_largest = f32::MIN;
                        let mut largest_count = 0;

                        for &child in shrinkable.iter() {
                            let child_width = self.elems[child].preferred_width;
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
                            let initial_width = child.preferred_width;

                            let mut at_min_width = false;
                            if child.preferred_width == largest {
                                let mut new_width = child.preferred_width + width_to_add; // basically a subtraciton
                                if new_width <= child.min_width {
                                    new_width = child.min_width;
                                    at_min_width = true;
                                }

                                let removed_width = new_width - initial_width;
                                remaining_width -= removed_width;

                                self.elems[child_index].preferred_width = new_width;
                            }

                            !at_min_width
                        });
                    }
                }
                LayoutDirection::TopToBottom => {
                    for &child_index in shrinkable.iter() {
                        let child = &self.elems[child_index];

                        let mut new_width = child.preferred_width.min(remaining_width);
                        if new_width < child.min_width {
                            new_width = child.min_width;
                        }

                        self.elems[child_index].preferred_width = new_width;
                    }
                }
            }
        }

        //
        // wrap text
        //

        for elem in (1..self.elems.len()).rev() {
            let element = &self.elems[elem];

            let element_text = &element.text_info.text;
            if !element_text.is_empty() {
                let layout_result = text_measurer.layout_text(
                    &element.text_info.text,
                    element.text_info.font_size,
                    element.preferred_width,
                );

                self.elems[elem].preferred_height += layout_result.height;
                // TODO: dont know if its need here
                self.elems[elem].min_height += layout_result.height;
                self.elems[elem].text_layout = layout_result;
            }
        }

        //
        // intrinsic/fit y
        //

        for elem in (1..self.elems.len()).rev() {
            let element = &self.elems[elem];

            // use heights calculated in text wrapping stage
            let mut preferred_height = element.preferred_height;
            let mut min_height = element.min_height;

            // children sizes
            let mut children_preferred_height = 0.0;
            let mut children_min_height = 0.0;
            match element.layout_direction {
                LayoutDirection::TopToBottom => {
                    // accumulate children sizes
                    for &child_index in element.children.iter() {
                        let child = &self.elems[child_index];
                        children_preferred_height += child.preferred_height;
                        children_min_height += child.min_height;
                    }
                }
                LayoutDirection::LeftToRight => {
                    // get max height of children
                    for &child_index in element.children.iter() {
                        let child = &self.elems[child_index];
                        children_preferred_height =
                            children_preferred_height.max(child.preferred_height);
                        children_min_height = children_min_height.max(child.min_height);
                    }
                }
            }
            preferred_height += children_preferred_height;
            min_height += children_min_height;

            // gap
            match element.layout_direction {
                LayoutDirection::TopToBottom => {
                    let child_count = element.children.len().saturating_sub(1);
                    let total_child_gap = child_count as f32 * element.child_gap;
                    preferred_height += total_child_gap;
                    min_height += total_child_gap;
                }
                LayoutDirection::LeftToRight => {}
            }

            // padding
            preferred_height += element.padding.vertical();
            min_height += element.padding.vertical();

            // fixed size overwrite
            if let Sizing::Fixed(fixed_height) = element.sizing_y {
                preferred_height = fixed_height;
                min_height = fixed_height;
            }

            self.elems[elem].preferred_height = preferred_height;
            self.elems[elem].min_height = min_height;
        }

        //
        // percent/grow/shrink y
        //

        for elem in 0..self.elems.len() {
            let element = self.elems[elem].clone();

            // percent
            for &child in element.children.iter() {
                if let Sizing::Percent(p) = self.elems[child].sizing_y {
                    let height = element.preferred_height * p;

                    self.elems[child].preferred_height = height;
                    self.elems[child].min_height = height;
                }
            }

            // remaining height (used for grow/shrink)
            let mut remaining_height = element.preferred_height;
            remaining_height -= element.padding.vertical();
            if let LayoutDirection::TopToBottom = element.layout_direction {
                remaining_height -=
                    (element.children.len().saturating_sub(1)) as f32 * element.child_gap;
                for &child in element.children.iter() {
                    remaining_height -= self.elems[child].preferred_height;
                }
            }

            // growable
            let mut growable = Vec::new();
            for &child in element.children.iter() {
                if let Sizing::Grow = self.elems[child].sizing_y {
                    growable.push(child);
                }
            }
            match element.layout_direction {
                LayoutDirection::TopToBottom => {
                    // distribute remaining height
                    while remaining_height > 0.0 && !growable.is_empty() {
                        let mut smallest = f32::MAX;
                        let mut second_smallest = f32::MAX;
                        let mut smallest_count = 0;

                        for &child in growable.iter() {
                            let child_height = self.elems[child].preferred_height;
                            if child_height < smallest {
                                second_smallest = smallest;
                                smallest = child_height;
                                smallest_count = 1;
                            } else if child_height == smallest {
                                smallest_count += 1;
                            } else if child_height < second_smallest {
                                second_smallest = child_height;
                            }
                        }

                        // if there are multiple with the smallest height and there is not enough
                        // to add to all of them, then distribute the remaining height
                        let height_to_make_equal = second_smallest - smallest;
                        let available_height_after_sharing =
                            remaining_height / smallest_count as f32;
                        let height_to_add =
                            height_to_make_equal.min(available_height_after_sharing);

                        for &child in growable.iter() {
                            let child_height = self.elems[child].preferred_height;
                            if child_height == smallest {
                                self.elems[child].preferred_height += height_to_add;
                                remaining_height -= height_to_add;
                            }
                        }
                    }
                }
                LayoutDirection::LeftToRight => {
                    // fill up all available space
                    for &child_index in growable.iter() {
                        self.elems[child_index].preferred_height = remaining_height;
                    }
                }
            }

            // shrinkable
            let mut shrinkable = Vec::new();
            for &child in element.children.iter() {
                if let Sizing::Fit = self.elems[child].sizing_y {
                    shrinkable.push(child);
                }
            }
            match element.layout_direction {
                LayoutDirection::TopToBottom => {
                    // shrink to fit overflowing height
                    while remaining_height < 0.0 && !shrinkable.is_empty() {
                        let mut largest = f32::MIN;
                        let mut second_largest = f32::MIN;
                        let mut largest_count = 0;

                        for &child in shrinkable.iter() {
                            let child_height = self.elems[child].preferred_height;
                            if child_height > largest {
                                second_largest = largest;
                                largest = child_height;
                                largest_count = 1;
                            } else if child_height == largest {
                                largest_count += 1;
                            } else if child_height > second_largest {
                                second_largest = child_height;
                            }
                        }

                        // if there are multiple with the largest height and there is not enough
                        // to add to all of them, then distribute the remaining height
                        let height_to_make_equal = second_largest - largest;
                        let available_height_after_sharing =
                            remaining_height / largest_count as f32;
                        let height_to_add =
                            height_to_make_equal.max(available_height_after_sharing);
                        shrinkable.retain(|&child_index| {
                            let child = &self.elems[child_index];
                            let prev_height = child.preferred_height;

                            let mut at_min_height = false;
                            if child.preferred_height == largest {
                                let mut new_height = child.preferred_height + height_to_add;
                                if new_height <= child.min_height {
                                    new_height = child.min_height;
                                    at_min_height = true;
                                }

                                let removed_height = new_height - prev_height;
                                remaining_height -= removed_height;

                                self.elems[child_index].preferred_height = new_height;
                            }

                            !at_min_height
                        });
                    }
                }
                LayoutDirection::LeftToRight => {
                    for &child_index in shrinkable.iter() {
                        let child = &self.elems[child_index];

                        let mut new_height = child.preferred_height.min(remaining_height);
                        if new_height < child.min_height {
                            new_height = child.min_height;
                        }

                        self.elems[child_index].preferred_height = new_height;
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
                        offset += self.elems[child].preferred_width + child_gap;
                    }
                }
                LayoutDirection::TopToBottom => {
                    for child in element.children.clone().into_iter() {
                        self.elems[child].x = x;
                        self.elems[child].y = y + offset;
                        offset += self.elems[child].preferred_height + child_gap;
                    }
                }
            }
        }
    }
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

    pub background_color: Vec4,

    layout_direction: LayoutDirection,

    pub text_info: TextInfo,

    child_gap: f32,

    min_width: f32, // TODO: already handled by width?
    min_height: f32,

    // calculated fields
    pub x: f32,
    pub y: f32,
    pub preferred_width: f32,
    pub preferred_height: f32,

    pub text_layout: TextLayoutResult,

    pub parent: usize,
    pub children: Vec<usize>,
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
            background_color: vec4(0.0, 0.0, 0.0, 1.0),
            child_gap: 0.0,

            text_info: TextInfo {
                text: String::new(),
                text_color: vec4(0.0, 0.0, 0.0, 1.0),
                font_size: 12,
            },

            min_width: 0.0,
            min_height: 0.0,

            text_layout: TextLayoutResult {
                width: 0.0,
                height: 0.0,
                glyphs: Vec::new(),
            },
            x: 0.0,
            y: 0.0,
            preferred_width: 0.0,
            preferred_height: 0.0,
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

        assert_eq!(layouter.get_element(&elem1).preferred_width, 300.0);
        assert_eq!(layouter.get_element(&elem2).preferred_width, 200.0);
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

        assert_eq!(layouter.get_element(&elem1).preferred_width, 200.0);
        assert_eq!(layouter.get_element(&elem1).preferred_height, 100.0);
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

        assert_eq!(layouter.get_element(&elem1).preferred_width, 400.0);
        assert_eq!(layouter.get_element(&elem1).preferred_height, 200.0);
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

        assert_eq!(layouter.get_element(&elem1).preferred_width, 200.0);
        assert_eq!(layouter.get_element(&elem1).preferred_height, 250.0);
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

        assert_eq!(layouter.get_element(&elem1).preferred_width, 0.0);
        assert_eq!(layouter.get_element(&elem1).preferred_height, 0.0);
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

        assert_eq!(layouter.get_element(&elem1).preferred_width, 0.0);
        assert_eq!(layouter.get_element(&elem1).preferred_height, 0.0);
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

        assert_eq!(layouter.get_element(&elem1).preferred_width, 100.0);
        assert_eq!(layouter.get_element(&elem1).preferred_height, 200.0);
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

        assert_eq!(layouter.get_element(&elem1).preferred_width, 1000.0);
        assert_eq!(layouter.get_element(&elem1).preferred_height, 1000.0);
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

        assert_eq!(layouter.get_element(&elem2).preferred_width, 200.0);
        assert_eq!(layouter.get_element(&elem2).preferred_height, 100.0);
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

        assert_eq!(layouter.get_element(&elem2).preferred_width, 200.0);
        assert_eq!(layouter.get_element(&elem3).preferred_width, 200.0);
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

        assert_eq!(layouter.get_element(&elem2).preferred_height, 300.0);
        assert_eq!(layouter.get_element(&elem3).preferred_height, 300.0);
    }

    //
    // Shrink
    //
}
