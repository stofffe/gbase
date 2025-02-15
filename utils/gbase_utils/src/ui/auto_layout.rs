use super::{Alignment, Direction, GUIRenderer, SizeKind};

impl GUIRenderer {
    // PRE
    // Pixels
    // Text size
    fn auto_layout_fixed(&mut self, index: usize) {
        // Pixels
        let this = self.get_widget_mut(index);
        if let SizeKind::Pixels(px) = this.width {
            this.computed_size[0] = px;
        }
        if let SizeKind::Pixels(px) = this.height {
            this.computed_size[1] = px;
        }

        // children
        for i in 0..self.widgets[index].children.len() {
            self.auto_layout_fixed(self.widgets[index].children[i]);
        }
    }

    // PRE
    // PercentOfParent
    fn auto_layout_percent(&mut self, index: usize) {
        let parent = self.get_widget_parent(index);
        let parent_inner_size = parent.computed_size_margin_padding();
        let this = self.get_widget_mut(index);

        if let SizeKind::PercentOfParent(p) = this.width {
            this.computed_size[0] = parent_inner_size[0] * p;
        }
        if let SizeKind::PercentOfParent(p) = this.height {
            this.computed_size[1] = parent_inner_size[1] * p;
        }

        // children
        for i in 0..this.children.len() {
            self.auto_layout_percent(self.widgets[index].children[i]);
        }
    }

    fn auto_layout_text(&mut self, index: usize) {
        // Text size
        let parent_width = self.get_widget_parent(index).computed_size_margin_padding()[0];
        let this = self.get_widget_mut(index);
        let font_size = this.font_size;
        let width = this.width;
        let height = this.height;
        let text_wrap = this.text_wrap;

        if let SizeKind::TextSize = width {
            let (text_size, _) = self.font_atlas.text_size(
                &self.get_widget(index).text,
                font_size,
                if text_wrap { Some(parent_width) } else { None },
            );

            self.get_widget_mut(index).computed_size[0] = text_size[0];
        }
        if let SizeKind::TextSize = height {
            let (text_size, _) = self.font_atlas.text_size(
                &self.get_widget(index).text,
                font_size,
                if text_wrap { Some(parent_width) } else { None },
            );

            self.get_widget_mut(index).computed_size[1] = text_size[1];
        }

        // children
        for i in 0..self.widgets[index].children.len() {
            self.auto_layout_text(self.widgets[index].children[i]);
        }
    }

    // POST
    // ChildrenSum
    fn auto_layout_children(&mut self, index: usize) {
        // children
        for i in 0..self.widgets[index].children.len() {
            self.auto_layout_children(self.widgets[index].children[i]);
        }

        let this = self.get_widget(index);
        if let SizeKind::ChildrenSum = this.width {
            let size = match this.direction {
                // sum
                Direction::Row => {
                    let children_space = self.get_children_size(index, 0);
                    let padding_space = this.padding[0] * 2.0;
                    let margin_space = this.margin[0] * 2.0;
                    children_space + padding_space + margin_space
                }
                // max
                Direction::Column => {
                    let children_max = self.get_children_max(index, 0);
                    let padding_space = this.padding[0] * 2.0;
                    let margin_space = this.margin[0] * 2.0;
                    children_max + padding_space + margin_space
                }
            };
            self.get_widget_mut(index).computed_size[0] = size;
        }

        let this = self.get_widget(index);
        if let SizeKind::ChildrenSum = this.height {
            let size = match this.direction {
                // sum
                Direction::Column => {
                    let children_space = self.get_children_size(index, 1);
                    let padding_space = this.padding[1] * 2.0;
                    let margin_space = this.margin[1] * 2.0;
                    children_space + padding_space + margin_space
                }
                // max
                Direction::Row => {
                    let children_max = self.get_children_max(index, 1);
                    let padding_space = this.padding[1] * 2.0;
                    let margin_space = this.margin[1] * 2.0;
                    children_max + padding_space + margin_space
                }
            };
            self.get_widget_mut(index).computed_size[1] = size;
        }
    }

    // PRE
    // Grow
    fn auto_layout_grow(&mut self, index: usize) {
        let parent = self.get_widget_parent(index);
        let available_space = parent.computed_size_margin_padding();
        let parent_direction = parent.direction;

        let this = self.get_widget(index);
        if let SizeKind::Grow = this.width {
            let size = match parent_direction {
                Direction::Column => available_space[0],
                Direction::Row => {
                    let neighbours_size = self.get_children_size(this.parent, 0);
                    available_space[0] - neighbours_size
                }
            };
            self.get_widget_mut(index).computed_size[0] = size;
        }

        let this = self.get_widget(index);
        if let SizeKind::Grow = this.height {
            let size = match parent_direction {
                Direction::Row => available_space[1],
                Direction::Column => {
                    let neighbours_size = self.get_children_size(this.parent, 1);
                    available_space[1] - neighbours_size
                }
            };
            self.get_widget_mut(index).computed_size[1] = size;
        }

        // children
        for i in 0..self.widgets[index].children.len() {
            self.auto_layout_grow(self.widgets[index].children[i]);
        }
    }

    // PRE
    fn auto_layout_violations(&mut self, index: usize) {
        // let parent_index = self.w_now[index].parent;

        // SOLVE VIOLATIONS

        // children
        for i in 0..self.widgets[index].children.len() {
            self.auto_layout_violations(self.widgets[index].children[i]);
        }
    }

    // PRE
    // Relative pos
    fn auto_layout_final(&mut self, index: usize) {
        let this = self.get_widget(index);
        let cross_axis_alignment = this.cross_axis_alignment;
        let inner_pos = this.computed_pos_maring_padding();
        let inner_size = this.computed_size_margin_padding();
        let main_axis = this.direction.main_axis();
        let cross_axis = this.direction.cross_axis();
        let children_size = self.get_children_size(index, main_axis);

        let mut main_offset = match this.main_axis_alignment {
            Alignment::Start => 0.0,
            Alignment::Center => inner_size[main_axis] / 2.0 - children_size / 2.0,
            Alignment::End => inner_size[main_axis] - children_size,
        };

        // main axis
        for i in 0..this.children.len() {
            let child_index = self.get_widget(index).children[i];
            let child = self.get_widget_mut(child_index);
            let child_size = child.computed_size;

            // cross axis alignment
            let cross_offset = match cross_axis_alignment {
                Alignment::Start => 0.0,
                Alignment::Center => inner_size[cross_axis] / 2.0 - child_size[cross_axis] / 2.0,
                Alignment::End => inner_size[cross_axis] - child_size[cross_axis],
            };

            let mut pos = inner_pos;
            pos[main_axis] += main_offset;
            pos[cross_axis] += cross_offset;
            child.computed_pos = pos;

            main_offset += child_size[main_axis];
            main_offset += self.get_widget(index).gap;
        }

        // children
        for i in 0..self.widgets[index].children.len() {
            self.auto_layout_final(self.widgets[index].children[i]);
        }
    }

    // Algorithm
    // https://www.rfleury.com/p/ui-part-1-the-interaction-medium?s=w
    // https://www.rfleury.com/p/ui-part-2-build-it-every-frame-immediate
    // https://www.rfleury.com/p/ui-part-3-the-widget-building-language
    pub(crate) fn auto_layout(&mut self, index: usize) {
        self.auto_layout_fixed(index);
        self.auto_layout_percent(index);
        self.auto_layout_text(index);
        self.auto_layout_children(index);
        self.auto_layout_grow(index);
        self.auto_layout_violations(index);
        self.auto_layout_final(index);
        // dbg!(&self.w_now);
    }
}
