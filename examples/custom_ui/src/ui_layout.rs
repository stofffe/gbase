use gbase::{
    glam::{f32, vec4, Vec4},
    render, Context,
};

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

pub trait UILayoutTextMeasurer {
    fn calculate_preferred_text_size(
        &mut self,
        text: &str,
        font_size: u32,
        wrap_on_newline: bool,
    ) -> TextSizeResult;
    fn layout_text(&mut self, text: &str, font_size: u32, max_width: f32) -> TextLayoutResult;
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

    pub fn add_element(&mut self, element: UIElement, children: fn(&mut Self)) {
        self.open_element(element);
        children(self);
        self.close_element();
    }

    pub fn elements(&self) -> &[UIElement] {
        &self.elems
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
        text_measurer: &mut impl UILayoutTextMeasurer,
    ) {
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
        text_measurer: &mut impl UILayoutTextMeasurer,
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
                LayoutDirection::TopToBottom => {}
            }

            // padding
            preferred_width += element.padding.horizontal();
            min_width += element.padding.horizontal();

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
                        let mut width_to_add = remaining_width;

                        for &child in growable.iter() {
                            let child_width = self.elems[child].preferred_width;
                            if child_width < smallest {
                                second_smallest = smallest;
                                smallest = child_width;
                            }
                            if child_width > smallest && child_width < second_smallest {
                                second_smallest = child_width;
                                width_to_add = second_smallest - smallest;
                            }
                        }

                        width_to_add = width_to_add.min(remaining_width / growable.len() as f32);

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
                        let mut largest = 0.0;
                        let mut second_largest = 0.0;
                        let mut width_to_add = remaining_width;

                        for &child in shrinkable.iter() {
                            let child_width = self.elems[child].preferred_width;
                            if child_width > largest {
                                second_largest = largest;
                                largest = child_width;
                            }
                            if child_width < largest {
                                second_largest = second_largest.max(child_width);
                                width_to_add = second_largest - largest;
                            }
                        }

                        width_to_add = width_to_add.max(remaining_width / shrinkable.len() as f32);

                        shrinkable.retain(|&child_index| {
                            let child = &self.elems[child_index];
                            let prev_width = child.preferred_width;

                            let mut at_min_width = false;
                            if child.preferred_width == largest {
                                let mut new_width = child.preferred_width + width_to_add;
                                if new_width <= child.min_width {
                                    new_width = child.min_width;
                                    at_min_width = true;
                                }

                                let removed_width = new_width - prev_width;
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
                        let mut height_to_add = remaining_height;

                        for &child in growable.iter() {
                            let child_height = self.elems[child].preferred_height;
                            if child_height < smallest {
                                second_smallest = smallest;
                                smallest = child_height;
                            }
                            if child_height > smallest && child_height < second_smallest {
                                second_smallest = child_height;
                                height_to_add = second_smallest - smallest;
                            }
                        }

                        height_to_add = height_to_add.min(remaining_height / growable.len() as f32);

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
                        let mut largest = 0.0;
                        let mut second_largest = 0.0;
                        let mut height_to_add = remaining_height;

                        for &child in shrinkable.iter() {
                            let child_height = self.elems[child].preferred_height;
                            if child_height > largest {
                                second_largest = largest;
                                largest = child_height;
                            }
                            if child_height < largest {
                                second_largest = second_largest.max(child_height);
                                height_to_add = second_largest - largest;
                            }
                        }

                        height_to_add =
                            height_to_add.max(remaining_height / shrinkable.len() as f32);

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

            // let element = &self.elems[elem];
            // instances.push(UIElementInstace {
            //     position: [element.x, element.y],
            //     size: [element.preferred_width, element.preferred_height],
            //     color: element.background_color.to_array(),
            //     font_atlas_offset: [0.0, 0.0],
            //     font_atlas_size: [0.0, 0.0],
            // });

            // // glyphs
            // if !element.text_info.text.is_empty() {
            //     for glyph in element.text_layout.glyphs.iter() {
            //         let x = element.x + glyph.x;
            //         let y = element.y + glyph.y;
            //         instances.push(UIElementInstace {
            //             position: [x, y],
            //             size: [glyph.width, glyph.height],
            //             color: element.text_info.text_color.to_array(),
            //             font_atlas_offset: [glyph, glyph.y],
            //             font_atlas_size: [glyph.width, glyph.height],
            //         });
            //     }
            // }
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

    pub fn draw_with_children(self, layouter: &mut UILayouter, children: fn(&mut UILayouter)) {
        layouter.add_element(self, children);
    }
    pub fn draw(self, layouter: &mut UILayouter) {
        layouter.add_element(self, |_| {});
    }
}
