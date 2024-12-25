use super::{GUIRenderer, BLACK};
use crate::{
    collision::{self, Quad},
    input, render, Context,
};
use glam::{vec2, Vec2, Vec4};

#[derive(Debug, Clone, Copy)]
pub enum SizeKind {
    Null,
    Pixels(f32),
    PercentOfParent(f32),
    Grow,
    /// Only works when all children use Pixels
    ChildrenSum,
}

#[derive(Debug, Clone, Copy)]
pub enum Direction {
    Row,
    Column,
}

impl Direction {
    pub fn main_axis(&self) -> usize {
        match self {
            Direction::Row => 0,    // x
            Direction::Column => 1, // y
        }
    }
    pub fn cross_axis(&self) -> usize {
        match self {
            Direction::Row => 1,    // x
            Direction::Column => 0, // y
        }
    }
}

//
// Internal
//

#[derive(Debug, Clone)]
pub struct Widget {
    // data
    pub(crate) label: String,
    pub(crate) parent: usize,

    pub(crate) size_main: SizeKind,
    pub(crate) size_cross: SizeKind,

    pub(crate) direction: Direction,
    pub(crate) padding: Vec2,
    pub(crate) margin: Vec2,

    pub(crate) gap: f32,

    pub(crate) color: Vec4,
    pub(crate) text: String,
    pub(crate) text_color: Vec4,
    pub(crate) text_height: f32,
    pub(crate) text_wrap: bool,

    // computed
    pub(crate) computed_pos: Vec2,
    pub(crate) computed_size: Vec2,

    // flags
    pub(crate) clickable: bool,
    // long press
    // text alignment
    // main/cross axis alignment

    // state
    pub(crate) children: Vec<usize>,
}

impl Widget {
    // create
    pub fn new() -> Self {
        Self {
            label: String::new(),
            parent: root_index(),

            size_main: SizeKind::Pixels(0.2),
            size_cross: SizeKind::Pixels(0.2),

            direction: Direction::Column,
            padding: Vec2::ZERO,
            margin: Vec2::ZERO,
            gap: 0.0,

            color: Vec4::ZERO,

            text: String::new(),
            text_color: BLACK,
            text_height: 100.0,
            text_wrap: false,

            computed_pos: Vec2::ZERO,
            computed_size: Vec2::ZERO,

            clickable: false,

            children: Vec::new(),
        }
    }

    // public api
    pub fn render(self, ctx: &Context, renderer: &mut GUIRenderer) -> WidgetResult {
        let mut result = renderer
            .widgets_last
            .iter()
            .find(|w| w.label == self.label)
            .cloned()
            .map(|w| w.inner_logic(ctx, renderer))
            .unwrap_or_default();

        result.index = renderer.create_widget(self);

        result
    }

    pub(crate) fn inner_logic(&self, ctx: &Context, renderer: &mut GUIRenderer) -> WidgetResult {
        let id = self.label.clone();

        // includes: content, padding, border
        // excludes: margin
        let mut bounds = Quad::new(self.computed_pos, self.computed_size);
        bounds.pos += self.margin;
        bounds.size -= self.margin * 2.0;

        let mut clicked = false;
        if self.clickable {
            let mouse_up = input::mouse_button_released(ctx, input::MouseButton::Left);
            let mouse_down = input::mouse_button_just_pressed(ctx, input::MouseButton::Left);
            let inside = collision::point_quad_collision(input::mouse_pos_unorm(ctx), bounds);

            if inside {
                renderer.set_hot_this_frame(id.clone());
                if renderer.check_hot(&id) {
                    if renderer.check_active(&id) && mouse_up {
                        clicked = true;
                    } else if mouse_down {
                        renderer.set_active(id);
                    }
                }
            }
        }

        WidgetResult { index: 0, clicked }
    }

    // private api
    pub(crate) fn inner_render(&self, renderer: &mut GUIRenderer) {
        let mut bounds = Quad::new(self.computed_pos, self.computed_size);

        // only cut away margin and not padding
        bounds.pos += self.margin;
        bounds.size -= self.margin * 2.0;

        if self.color != Vec4::ZERO {
            renderer.quad(bounds.pos, bounds.size, self.color);
        }

        if !self.text.is_empty() {
            renderer.text(
                &self.text,
                bounds,
                self.text_height,
                self.text_color,
                self.text_wrap,
            );
        }
    }

    pub(crate) fn computed_inner_pos(&self) -> Vec2 {
        self.computed_pos + self.margin + self.padding
    }
    pub(crate) fn computed_inner_size(&self) -> Vec2 {
        self.computed_size - self.margin * 2.0 - self.padding * 2.0
    }
}

// builder methods
impl Widget {
    /// add label to identify widget for interactions
    pub fn label(mut self, value: impl Into<String>) -> Self {
        self.label = value.into();
        self
    }
    /// set parent widget
    pub fn parent(mut self, value: WidgetResult) -> Self {
        self.parent = value.index;
        self
    }
    /// set sizing rules for main axis
    pub fn size_main(mut self, value: SizeKind) -> Self {
        self.size_main = value;
        self
    }
    /// set sizing rules for cross axis
    pub fn size_cross(mut self, value: SizeKind) -> Self {
        self.size_cross = value;
        self
    }
    /// set layout direction of child elements
    pub fn direction(mut self, value: Direction) -> Self {
        self.direction = value;
        self
    }
    /// set uniform padding
    pub fn padding(mut self, value: f32) -> Self {
        self.padding = vec2(value, value);
        self
    }
    /// set uniform margin
    pub fn margin(mut self, value: f32) -> Self {
        self.margin = vec2(value, value);
        self
    }
    /// set horizontal / vertical padding
    pub fn padding_hv(mut self, value: Vec2) -> Self {
        self.padding = value;
        self
    }
    /// set horizontal / vertical margin
    pub fn margin_hv(mut self, value: Vec2) -> Self {
        self.margin = value;
        self
    }
    /// set gap between child elements on main axis
    pub fn gap(mut self, value: f32) -> Self {
        self.gap = value;
        self
    }
    /// set color of background
    pub fn color(mut self, value: Vec4) -> Self {
        self.color = value;
        self
    }
    /// set text content
    pub fn text(mut self, value: impl Into<String>) -> Self {
        self.text = value.into();
        self
    }
    /// set text color
    pub fn text_color(mut self, value: Vec4) -> Self {
        self.text_color = value;
        self
    }
    /// set font size
    pub fn text_font_size(mut self, value: f32) -> Self {
        self.text_height = value;
        self
    }
    /// enable/disable text wrapping
    pub fn text_wrap(mut self, value: bool) -> Self {
        self.text_wrap = value;
        self
    }
    /// make widget clickable
    pub fn clickable(mut self) -> Self {
        self.clickable = true;
        self
    }
}

//
// Result
//

#[derive(Debug, Clone, Copy)]
pub struct WidgetResult {
    pub index: usize,
    pub clicked: bool,
}

#[allow(clippy::derivable_impls)]
impl Default for WidgetResult {
    fn default() -> Self {
        Self {
            index: 0,
            clicked: false,
        }
    }
}

pub fn root_index() -> usize {
    0
}
pub fn root_widget(ctx: &Context) -> Widget {
    let screen_size = render::surface_size(ctx);
    Widget {
        label: String::from("ROOT"),
        parent: root_index(),

        size_main: SizeKind::Null,
        size_cross: SizeKind::Null,

        direction: Direction::Column,
        padding: Vec2::ZERO,
        margin: Vec2::ZERO,
        gap: 0.0,

        color: Vec4::ZERO,

        text: String::new(),
        text_color: Vec4::ZERO,
        text_height: 0.0,
        text_wrap: false,

        computed_pos: vec2(0.0, 0.0),
        computed_size: vec2(screen_size.width as f32, screen_size.height as f32),

        clickable: false,

        children: Vec::new(),
    }
}
