use super::{GUIRenderer, UiID, BLACK};
use crate::{
    collision::{self, Quad},
    input, Context,
};
use glam::{vec2, Vec2, Vec4};

//
// Internal
//

pub fn root_index() -> usize {
    0
}
pub fn root_widget() -> Widget {
    Widget {
        label: String::from("ROOT"),
        parent: root_index(),

        size_main: SizeKind::Null,
        size_cross: SizeKind::Null,

        direction: Direction::Column,

        color: Vec4::ZERO,

        text: String::new(),
        text_color: Vec4::ZERO,
        text_height: 0.0,
        text_wrap: false,

        pos: vec2(0.0, 0.0),
        size: vec2(1.0, 1.0),

        clickable: false,

        children: Vec::new(),
    }
}

#[derive(Debug, Clone, Copy)]
pub enum SizeKind {
    Null,
    Pixels(f32),
    PercentOfParent(f32),
    Grow,
}

#[derive(Debug, Clone, Copy)]
pub enum Direction {
    Row,
    Column,
}

#[derive(Debug, Clone)]
pub struct Widget {
    // data
    pub(crate) label: String,
    pub(crate) parent: usize,

    pub(crate) size_main: SizeKind,
    pub(crate) size_cross: SizeKind,

    pub(crate) direction: Direction,

    pub(crate) color: Vec4,
    pub(crate) text: String,
    pub(crate) text_color: Vec4,
    pub(crate) text_height: f32,
    pub(crate) text_wrap: bool,

    // computed
    pub(crate) pos: Vec2,
    pub(crate) size: Vec2,

    // flags
    pub(crate) clickable: bool,
    // long press
    // text alignment
    // margin
    // child alignment

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

            color: Vec4::ZERO,

            text: String::new(),
            text_color: BLACK,
            text_height: 0.05,
            text_wrap: false,

            pos: vec2(0.0, 0.0),
            size: vec2(0.0, 0.0),

            clickable: false,

            children: Vec::new(),
        }
    }

    // 1. get widget from id
    // 2. logic using last frame
    // 3. auto layout this frame

    // public api
    pub fn render(mut self, ctx: &Context, renderer: &mut GUIRenderer) -> WidgetResult {
        let id = self.label.clone();

        let widget_prev = renderer.widgets_last.iter().find(|w| w.label == self.label); // TODO use id instead

        //
        // logic
        //

        let mut clicked = false;
        if let Some(widget_prev) = widget_prev {
            if self.clickable {
                let mouse_up = input::mouse_button_released(ctx, input::MouseButton::Left);
                let mouse_down = input::mouse_button_just_pressed(ctx, input::MouseButton::Left);
                let inside = collision::point_quad_collision(
                    input::mouse_pos_unorm(ctx),
                    Quad::new(widget_prev.pos, widget_prev.size),
                );

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
        }

        //
        // inital layout
        //

        // let parent = renderer.get_widget(self.parent);
        // self.pos += parent.pos;

        let index = renderer.create_widget(self);

        WidgetResult { index, clicked }
    }

    // private api
    pub(crate) fn inner_render(&self, renderer: &mut GUIRenderer) {
        let pos = self.pos;
        let size = self.size;
        let color = self.color;

        if self.color != Vec4::ZERO {
            renderer.quad(pos, size, color);
        }

        if !self.text.is_empty() {
            renderer.text(
                &self.text,
                Quad::new(pos, size),
                self.text_height,
                self.text_color,
                self.text_wrap,
            );
        }
    }
}

impl Default for Widget {
    fn default() -> Self {
        Self::new()
    }
}

// builder methods
impl Widget {
    pub fn label(mut self, value: impl Into<String>) -> Self {
        self.label = value.into();
        self
    }
    pub fn parent(mut self, value: WidgetResult) -> Self {
        self.parent = value.index;
        self
    }

    pub fn size_main(mut self, value: SizeKind) -> Self {
        self.size_main = value;
        self
    }
    pub fn size_cross(mut self, value: SizeKind) -> Self {
        self.size_cross = value;
        self
    }
    pub fn direction(mut self, value: Direction) -> Self {
        self.direction = value;
        self
    }

    pub fn color(mut self, value: Vec4) -> Self {
        self.color = value;
        self
    }

    pub fn text(mut self, value: impl Into<String>) -> Self {
        self.text = value.into();
        self
    }
    pub fn text_color(mut self, value: Vec4) -> Self {
        self.text_color = value;
        self
    }
    pub fn text_height(mut self, value: f32) -> Self {
        self.text_height = value;
        self
    }
    pub fn text_wrap(mut self, value: bool) -> Self {
        self.text_wrap = value;
        self
    }

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
