#![allow(clippy::collapsible_if)]

use super::{GUIRenderer, BLACK};
use crate::{
    collision::{self, Quad},
    input,
    render::{self},
    Context,
};
use glam::{vec2, Vec2, Vec4};

#[derive(Debug, Clone, Copy)]
pub enum SizeKind {
    Null,
    Pixels(f32),
    PercentOfParent(f32),
    Grow,
    ChildrenSum, // Only works when all children use Pixels
}

#[derive(Debug, Clone, Copy)]
pub enum Direction {
    Row,
    Column,
}

impl Direction {
    pub fn main_axis(&self) -> usize {
        match self {
            Direction::Row => 0,
            Direction::Column => 1,
        }
    }
    pub fn cross_axis(&self) -> usize {
        match self {
            Direction::Row => 1,
            Direction::Column => 0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Alignment {
    Start,
    Center,
    End,
}

//
// Internal
//
#[derive(Debug, Clone)]
pub struct Widget {
    // data
    pub(crate) label: String,
    pub(crate) parent: usize,

    pub(crate) width: SizeKind,
    pub(crate) height: SizeKind,

    pub(crate) direction: Direction,
    pub(crate) padding: Vec2,
    pub(crate) margin: Vec2,

    pub(crate) gap: f32,

    pub(crate) main_axis_alignment: Alignment,
    pub(crate) cross_axis_alignment: Alignment,

    pub(crate) color: Vec4,
    pub(crate) text: String,
    pub(crate) text_color: Vec4,
    pub(crate) text_height: f32,
    pub(crate) text_wrap: bool,

    // computed
    pub(crate) computed_pos: Vec2,
    pub(crate) computed_size: Vec2,

    // state
    pub(crate) children: Vec<usize>,
}

impl Widget {
    // create
    pub fn new() -> Self {
        Self {
            label: String::new(),
            parent: root_index(),

            width: SizeKind::Pixels(100.0),
            height: SizeKind::Pixels(100.0),

            direction: Direction::Column,
            padding: Vec2::ZERO,
            margin: Vec2::ZERO,
            gap: 0.0,

            main_axis_alignment: Alignment::Start,
            cross_axis_alignment: Alignment::Start,

            color: Vec4::ZERO,

            text: String::new(),
            text_color: BLACK,
            text_height: 100.0,
            text_wrap: false,

            computed_pos: Vec2::ZERO,
            computed_size: Vec2::ZERO,

            children: Vec::new(),
        }
    }

    pub fn slider(
        &mut self,
        ctx: &Context,
        renderer: &mut GUIRenderer,
        min: f32,
        max: f32,
        value: &mut f32,
    ) -> SliderResult {
        let id = self.label.clone();

        let widget_last_frame = renderer
            .widgets_last
            .iter()
            .find(|w| w.label == id)
            .cloned();

        if let Some(lw) = widget_last_frame {
            let mut bounds = Quad::new(lw.computed_pos, lw.computed_size);
            bounds.pos += lw.margin;
            bounds.size -= lw.margin * 2.0;
            let mouse_pos = input::mouse_pos(ctx);
            let mouse_down = input::mouse_button_just_pressed(ctx, input::MouseButton::Left);
            let inside = collision::point_quad_collision(mouse_pos, bounds);

            if inside {
                renderer.set_hot_this_frame(id.clone());
                if renderer.check_hot(&id) && mouse_down {
                    renderer.set_active(id.clone());
                }
            }

            if renderer.check_active(&id) {
                let p = ((mouse_pos.x - bounds.left()) / bounds.size.x).clamp(0.0, 1.0);

                *value = (1.0 - p) * min + p * max;
            }
        };

        let slider_pos = ((*value - min) / (max - min)).clamp(0.0, 1.0);
        let index = renderer.create_widget(self.clone());

        SliderResult { index, slider_pos }
    }

    pub fn button(&mut self, ctx: &Context, renderer: &mut GUIRenderer) -> ButtonResult {
        let id = self.label.clone();

        let widget_last_frame = renderer
            .widgets_last
            .iter()
            .find(|w| w.label == id)
            .cloned();

        let mut clicked = false;
        if let Some(lw) = widget_last_frame {
            let mut bounds = Quad::new(lw.computed_pos, lw.computed_size);
            bounds.pos += lw.margin;
            bounds.size -= lw.margin * 2.0;
            let mouse_pos = input::mouse_pos(ctx);
            let mouse_up = input::mouse_button_released(ctx, input::MouseButton::Left);
            let mouse_down = input::mouse_button_just_pressed(ctx, input::MouseButton::Left);
            let inside = collision::point_quad_collision(mouse_pos, bounds);

            if inside {
                renderer.set_hot_this_frame(id.clone());
                if renderer.check_hot(&id) {
                    if renderer.check_active(&id) && mouse_up {
                        clicked = true;
                    } else if mouse_down {
                        renderer.set_active(id.clone());
                    }
                }
            }
        };
        let index = renderer.create_widget(self.clone());

        ButtonResult { index, clicked }
    }

    // public api
    pub fn layout(&self, renderer: &mut GUIRenderer) -> LayoutResult {
        let index = renderer.create_widget(self.clone());
        LayoutResult { index }
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
    pub fn parent(mut self, value: impl Parent) -> Self {
        self.parent = value.index();
        self
    }
    /// set sizing rules for main axis
    pub fn width(mut self, value: SizeKind) -> Self {
        self.width = value;
        self
    }
    /// set sizing rules for cross axis
    pub fn height(mut self, value: SizeKind) -> Self {
        self.height = value;
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
    /// set child alignment on main axis
    pub fn main_axis_alignment(mut self, value: Alignment) -> Self {
        self.main_axis_alignment = value;
        self
    }
    /// set child alignment on cross axis
    pub fn cross_axis_alignment(mut self, value: Alignment) -> Self {
        self.cross_axis_alignment = value;
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
}

//
// Result
//

pub trait Parent {
    fn index(&self) -> usize;
}

#[derive(Debug, Clone, Copy)]
pub struct ButtonResult {
    pub index: usize,
    pub clicked: bool,
}

impl Parent for ButtonResult {
    fn index(&self) -> usize {
        self.index
    }
}

#[derive(Debug, Clone, Copy)]
pub struct LayoutResult {
    pub index: usize,
}

impl Parent for LayoutResult {
    fn index(&self) -> usize {
        self.index
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SliderResult {
    pub index: usize,
    pub slider_pos: f32,
}

impl Parent for SliderResult {
    fn index(&self) -> usize {
        self.index
    }
}

//
// Constants
//

pub fn root_index() -> usize {
    0
}
pub fn root_widget(ctx: &Context) -> Widget {
    let screen_size = render::surface_size(ctx);
    Widget {
        label: String::from("ROOT"),
        parent: root_index(),

        width: SizeKind::Null,
        height: SizeKind::Null,

        direction: Direction::Column,
        padding: Vec2::ZERO,
        margin: Vec2::ZERO,
        gap: 0.0,

        main_axis_alignment: Alignment::Start,
        cross_axis_alignment: Alignment::Start,

        color: Vec4::ZERO,

        text: String::new(),
        text_color: Vec4::ZERO,
        text_height: 0.0,
        text_wrap: false,

        computed_pos: vec2(0.0, 0.0),
        computed_size: vec2(screen_size.width as f32, screen_size.height as f32),

        children: Vec::new(),
    }
}
