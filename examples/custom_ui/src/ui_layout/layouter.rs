use gbase::{render, Context};

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

        self.intrinsic_pass(Axis::X, text_measurer);

        self.fixed_fit_pass(Axis::X);

        self.percent_grow_shrink_pass(Axis::X);

        self.intrinsic_pass(Axis::Y, text_measurer);

        self.fixed_fit_pass(Axis::Y);

        self.percent_grow_shrink_pass(Axis::Y);

        self.position_pass();
    }

    // Sets the intrinsic size, the size the content naturally wants regardless of sizing mode
    //
    /// Children need to be ready before parent so do REVERSE DFS
    fn intrinsic_pass(&mut self, axis: Axis, text_measurer: &mut impl TextMeasurer) {
        let i = match axis {
            Axis::X => 0,
            Axis::Y => 1,
        };

        for elem in (1..self.elems.len()).rev() {
            let element = &self.elems[elem];

            // 1. Calculate intrinsic sizing
            let (intrinsic_content_min, intrinsic_content_max) = match element.content_type {
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

                            let intrinsic_min = layout_result.min_width;
                            let intrinsic_max = layout_result.max_width;

                            (intrinsic_min, intrinsic_max)
                        }
                        Axis::Y => {
                            let content_width = element.content_size(Axis::X);

                            let layout_result = text_measurer.layout_text(
                                &text_info.text,
                                text_info.font_size,
                                content_width,
                            );

                            let intrinsic_min = layout_result.height;
                            let intrinsic_max = layout_result.height;

                            self.elems[elem].text_layout = layout_result;

                            (intrinsic_min, intrinsic_max)
                        }
                    }
                }
                ContentType::Children => {
                    let element = &self.elems[elem];
                    let mut intrinsic_min = 0.0;
                    let mut intrinsic_max = 0.0;

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
                            intrinsic_min += child.min[i];
                            intrinsic_max += child.max[i];
                        }
                        // add gap
                        let child_count = element.children.len().saturating_sub(1);
                        let total_child_gap = child_count as f32 * element.child_gap;
                        intrinsic_min += total_child_gap;
                        intrinsic_max += total_child_gap;
                    } else {
                        // get max size of children
                        for &child_index in element.children.iter() {
                            let child = &self.elems[child_index];
                            // TODO: unsure about this one
                            intrinsic_min = intrinsic_min.max(child.min[i]);
                            intrinsic_max = intrinsic_max.max(child.max[i]);
                        }
                    }

                    (intrinsic_min, intrinsic_max)
                }
            };

            // padding applies regardless of content type
            let element = &self.elems[elem];

            let intrinsic_min = intrinsic_content_min + element.padding.symmetric_sum(axis);
            let intrinsic_max = intrinsic_content_max + element.padding.symmetric_sum(axis);

            let override_min = self.elems[elem].min_override[i];
            let override_max = self.elems[elem].max_override[i];

            let min = intrinsic_min.max(override_min);
            let max = intrinsic_max.min(override_max);

            debug_assert!(min <= max);

            // set min and max, but actual size is not set
            // constrain by explicit min/max, if they are not set the computed sizes will always win
            self.elems[elem].min[i] = min;
            self.elems[elem].max[i] = max;

            // fixed size is the only sizing mode which defines a size independent of children/parent elements
            // so a special case can be applied which treats it like an intrinsic
            // note that it ignores the intrinsic min/max but respects the overriden min/max
            if let Sizing::Fixed(fixed_size) = self.elems[elem].sizing[i] {
                self.elems[elem].min[i] = fixed_size.max(override_min);
                self.elems[elem].max[i] = fixed_size.min(override_max);
            }
        }
    }

    /// Sets the fixed and fit sizing based on intrinsic sizes
    ///
    /// Iterates DFS so parent is always processed before child
    fn fixed_fit_pass(&mut self, axis: Axis) {
        let i = match axis {
            Axis::X => 0,
            Axis::Y => 1,
        };

        for elem in 1..self.elems.len() {
            // Sizing mode based sizing
            let min = self.elems[elem].min[i];
            let max = self.elems[elem].max[i];

            match self.elems[elem].sizing[i] {
                Sizing::Fixed(fixed_size) => self.elems[elem].size[i] = fixed_size.clamp(min, max),
                Sizing::Fit => self.elems[elem].size[i] = max, // fit assumed to be max-content
                _ => (),
            }
        }
    }

    /// Sets percent sizing, then grows or shrinks elements as needed
    /// Only Sizing::Grow are allowed to grow
    /// All elements with size > min_size are allowed to shrink
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

            let content_size = element.content_size(axis);

            // percent on all children
            for &child in element.children.iter() {
                if let Sizing::Percent(p) = self.elems[child].sizing[i] {
                    let child_size = content_size * p;

                    let child_override_min = self.elems[child].min_override[i];
                    let child_override_max = self.elems[child].max_override[i];
                    self.elems[child].size[i] =
                        child_size.clamp(child_override_min, child_override_max);
                }
            }

            // calculate remaining size (used for grow/shrink)
            let mut remaining_size = content_size;
            let layout_dir_matches_axis = match axis {
                Axis::X => matches!(element.layout_direction, LayoutDirection::LeftToRight),
                Axis::Y => matches!(element.layout_direction, LayoutDirection::TopToBottom),
            };
            if layout_dir_matches_axis {
                // remove gaps
                remaining_size -=
                    (element.children.len().saturating_sub(1)) as f32 * element.child_gap;
                // remove child sizes
                for &child in element.children.iter() {
                    remaining_size -= self.elems[child].size[i];
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
                // distribute remaining size
                while remaining_size > 0.0 && !growable.is_empty() {
                    let mut smallest = f32::MAX;
                    let mut second_smallest = f32::MAX;
                    let mut smallest_count = 0;

                    for &child in growable.iter() {
                        let child_size = self.elems[child].size[i];
                        if child_size < smallest {
                            second_smallest = smallest;
                            smallest = child_size;
                            smallest_count = 1;
                        } else if child_size == smallest {
                            smallest_count += 1;
                        } else if child_size < second_smallest {
                            second_smallest = child_size;
                        }
                    }

                    // TODO: if max size is reached, pop from growable list

                    // if there are multiple with the smallest size and there is not enough
                    // to add to all of them, then distribute the remaining size
                    let size_to_make_equal = second_smallest - smallest;
                    let available_size_after_sharing = remaining_size / smallest_count as f32;
                    let size_to_add = size_to_make_equal.min(available_size_after_sharing);
                    for &child in growable.iter() {
                        let child_size = self.elems[child].size[i];
                        if child_size == smallest {
                            self.elems[child].size[i] += size_to_add;
                            remaining_size -= size_to_add;
                        }
                    }
                }
            } else {
                // fill up all available space
                for &child_index in growable.iter() {
                    let content_size = element.content_size(axis);
                    self.elems[child_index].size[i] = content_size;
                }
            }

            // shrink children
            let mut shrinkable = Vec::new();
            for &child in element.children.iter() {
                match self.elems[child].sizing[i] {
                    Sizing::Grow => shrinkable.push(child), // TODO: is this needed, shouldnt be able to grow if there is too little space
                    Sizing::Fit => shrinkable.push(child),
                    _ => (), // TODO: eventually add all that have min_size < size
                }
            }
            if layout_dir_matches_axis {
                // shrink to fit overflowing size
                while remaining_size < 0.0 && !shrinkable.is_empty() {
                    let mut largest = f32::MIN;
                    let mut second_largest = f32::MIN;
                    let mut largest_count = 0;

                    for &child in shrinkable.iter() {
                        let child_size = self.elems[child].size[i];
                        if child_size > largest {
                            second_largest = largest;
                            largest = child_size;
                            largest_count = 1;
                        } else if child_size == largest {
                            largest_count += 1;
                        } else if child_size > second_largest {
                            second_largest = child_size;
                        }
                    }

                    // if there are multiple with the largest size and there is not enough
                    // to add to all of them, then distribute the remaining size
                    let size_to_make_equal = second_largest - largest; // negative
                    let available_size_after_sharing = remaining_size / largest_count as f32; // negative
                    let size_to_add = size_to_make_equal.max(available_size_after_sharing); // negative
                    shrinkable.retain(|&child_index| {
                        let child = &self.elems[child_index];
                        let initial_size = child.size[i];

                        let mut at_min_size = false;
                        if child.size[i] == largest {
                            let mut new_size = child.size[i] + size_to_add; // basically a subtraciton
                            if new_size <= child.min[i] {
                                new_size = child.min[i];
                                at_min_size = true;
                            }

                            let removed_size = new_size - initial_size;
                            remaining_size -= removed_size;

                            self.elems[child_index].size[i] = new_size;
                        }

                        !at_min_size
                    });
                }
            } else {
                for &child_index in shrinkable.iter() {
                    let child = &mut self.elems[child_index];

                    if remaining_size < 0.0 {
                        let new_size = child.size[i] + remaining_size;
                        if new_size < child.min[i] {
                            self.elems[child_index].size[i] = child.min[i];
                        } else {
                            self.elems[child_index].size[i] = new_size;
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
        // TODO: could this by using parent pointer instead?
        for elem in 1..self.elems.len() {
            let element = &self.elems[elem];

            let x = element.pos[0] + element.padding.left;
            let y = element.pos[1] + element.padding.top;

            let child_gap = element.child_gap;
            let mut offset = 0.0;

            match element.layout_direction {
                LayoutDirection::LeftToRight => {
                    for child in element.children.clone().into_iter() {
                        let padding = self.elems[child].padding.clone();
                        self.elems[child].pos[0] = x + offset;
                        self.elems[child].pos[1] = y;
                        self.elems[child].content_pos[0] = x + offset + padding.left;
                        self.elems[child].content_pos[1] = y + padding.top;
                        offset += self.elems[child].size[0] + child_gap;
                    }
                }
                LayoutDirection::TopToBottom => {
                    for child in element.children.clone().into_iter() {
                        let padding = self.elems[child].padding.clone();
                        self.elems[child].pos[0] = x;
                        self.elems[child].pos[1] = y + offset;
                        self.elems[child].content_pos[0] = x + padding.left;
                        self.elems[child].content_pos[1] = y + offset + padding.top;
                        offset += self.elems[child].size[1] + child_gap;
                    }
                }
            }
        }
    }
}
