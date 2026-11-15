use gbase::{render, tracing, Context};

use crate::{
    ui_font::TextMeasurer,
    ui_layout::{
        Axis, ContentType, LayoutDirection, Sizing, TextInfo, UIElement, UIElementId, ROOT_ELEMENT,
    },
};

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

        #[cfg(debug_assertions)]
        for elem in 1..self.elems.len() {
            let element = &self.elems[elem];

            if !matches!(element.content_type, ContentType::Text) {
                assert!(element.text.is_empty());
            }
        }

        self.intrinsic_fixed_fit_pass(Axis::X, text_measurer);

        self.percent_grow_shrink_pass(Axis::X);

        self.intrinsic_fixed_fit_pass(Axis::Y, text_measurer);

        self.percent_grow_shrink_pass(Axis::Y);

        self.position_pass();
    }

    // Sets the intrinsic, fixed and fit size
    //
    /// Children need to be ready before parent so do REVERSE DFS
    fn intrinsic_fixed_fit_pass(&mut self, axis: Axis, text_measurer: &mut impl TextMeasurer) {
        let i = match axis {
            Axis::X => 0,
            Axis::Y => 1,
        };

        // container
        // text
        // image

        for elem in (1..self.elems.len()).rev() {
            let element = &self.elems[elem];

            let (min_size, max_size) = if let Sizing::Fixed(fixed_size) = element.sizing[i] {
                (fixed_size, fixed_size)
            } else {
                let (mut min_size, mut max_size) = match element.content_type {
                    ContentType::Text => {
                        let text_info = TextInfo {
                            text: element.text.clone(),
                            text_color: element.text_color,
                            font_size: element.font_size,
                        };
                        match axis {
                            Axis::X => {
                                let layout_result = text_measurer.calculate_preferred_text_size(
                                    &text_info.text,
                                    text_info.font_size,
                                    false,
                                );

                                let min_size = layout_result.min_width;
                                let max_size = layout_result.max_width;

                                (min_size, max_size)
                            }
                            Axis::Y => {
                                let layout_result = text_measurer.layout_text(
                                    &text_info.text,
                                    text_info.font_size,
                                    element.size[0],
                                );

                                let min_size = layout_result.height;
                                let max_size = layout_result.height;

                                self.elems[elem].text_layout = layout_result;

                                (min_size, max_size)
                            }
                        }
                    }
                    ContentType::Children => {
                        let element = &self.elems[elem];
                        let mut min_size = 0.0;
                        let mut max_size = 0.0;

                        let layout_dir_matches_axis = match axis {
                            Axis::X => {
                                matches!(element.layout_direction, LayoutDirection::LeftToRight)
                            }
                            Axis::Y => {
                                matches!(element.layout_direction, LayoutDirection::TopToBottom)
                            }
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

                        (min_size, max_size)
                    }
                    ContentType::None => (0.0, 0.0),
                };

                // padding applies regardless of sizing mode
                let element = &self.elems[elem];
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
                (min_size, max_size)
            };

            // set min and max, but actual size is not set
            // constrain by explicit min/max, if they are not set the computed sizes will always win
            let override_min = self.elems[elem].min_override[i];
            let override_max = self.elems[elem].max_override[i];
            self.elems[elem].min[i] = min_size.max(override_min);
            self.elems[elem].max[i] = max_size.min(override_max);

            // Sizing mode based sizing
            let min = self.elems[elem].min[i];
            let max = self.elems[elem].max[i];
            match self.elems[elem].sizing[i] {
                Sizing::Fixed(fixed_size) => self.elems[elem].size[i] = fixed_size.clamp(min, max),
                Sizing::Fit => self.elems[elem].size[i] = max_size.clamp(min, max),
                _ => (),
            }
        }
    }

    /// Sets percent sizing, then grows or shrinks elements as needed
    /// Only Sizing::Grow are allowed to grow
    /// All elements with width > min_width are allowed to shrink
    ///
    /// Iterates DFS so parent is always processed before child
    /// When iterating an element will loop through its children and grow those with Grow sizing
    /// The percent does however need to be applied first so the grow knows how much space is left
    /// but percent can not be its own pass since it depends on parents with Grow
    fn percent_grow_shrink_pass(&mut self, axis: Axis) {
        let i = match axis {
            Axis::X => 0,
            Axis::Y => 1,
        };

        for elem in 0..self.elems.len() {
            let element = self.elems[elem].clone();
            // percent on all children
            for &child in element.children.iter() {
                if let Sizing::Percent(p) = self.elems[child].sizing[i] {
                    // TODO: this includes padding, probably should subtract it
                    let width = element.size[i] * p;

                    let child_override_min = self.elems[child].min_override[i];
                    let child_override_max = self.elems[child].max_override[i];
                    self.elems[child].size[i] = width.clamp(child_override_min, child_override_max);
                }
            }

            // calculate remaining width (used for grow/shrink)
            let mut remaining_width = element.size[i];

            let padding = match axis {
                Axis::X => element.padding.horizontal(),
                Axis::Y => element.padding.vertical(),
            };
            remaining_width -= padding;

            let layout_dir_matches_axis = match axis {
                Axis::X => matches!(element.layout_direction, LayoutDirection::LeftToRight),
                Axis::Y => matches!(element.layout_direction, LayoutDirection::TopToBottom),
            };
            if layout_dir_matches_axis {
                remaining_width -=
                    (element.children.len().saturating_sub(1)) as f32 * element.child_gap;
                for &child in element.children.iter() {
                    remaining_width -= self.elems[child].size[i];
                }
            }

            // grow children
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
                            self.elems[child].size[i] += width_to_add;
                            remaining_width -= width_to_add;
                        }
                    }
                }
            } else {
                // fill up all available space
                for &child_index in growable.iter() {
                    // TODO: max size check
                    self.elems[child_index].size[i] = element.size[i];
                }
            }

            // shrink children
            let mut shrinkable = Vec::new();
            for &child in element.children.iter() {
                match self.elems[child].sizing[i] {
                    Sizing::Grow => shrinkable.push(child), // TODO: is this needed, shouldnt be able to grow if there is too little space
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
