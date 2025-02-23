use super::{GUIRenderer, BLACK};
use gbase::{
    collision::{self, AABB},
    glam::{vec2, vec4, Vec2, Vec4},
    input,
    render::{self},
    Context,
};

#[derive(Debug, Clone, Copy)]
pub enum SizeKind {
    Null,
    Pixels(f32),
    TextSize,
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
    pub(crate) border_radius: Vec4, // (tr, br, tl, bl)

    pub(crate) gap: f32,

    pub(crate) main_axis_alignment: Alignment,
    pub(crate) cross_axis_alignment: Alignment,

    pub(crate) color: Option<Vec4>,
    pub(crate) text: String,
    pub(crate) text_color: Vec4,
    pub(crate) font_size: f32,
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
            border_radius: Vec4::ZERO,

            gap: 0.0,

            main_axis_alignment: Alignment::Start,
            cross_axis_alignment: Alignment::Start,

            color: None,

            text: String::new(),
            text_color: BLACK,
            font_size: 100.0,
            text_wrap: false,

            computed_pos: Vec2::ZERO,
            computed_size: Vec2::ZERO,

            children: Vec::new(),
        }
    }
}

impl Default for Widget {
    fn default() -> Self {
        Self::new()
    }
}

//
// Interaction types
//

impl Widget {
    // public api
    pub fn render(self, renderer: &mut GUIRenderer) -> LayoutResult {
        let index = renderer.insert_widget(self);
        LayoutResult { index }
    }

    pub fn button(self, ctx: &Context, renderer: &mut GUIRenderer) -> ButtonResult {
        debug_assert!(!self.label.is_empty(), "ui button must have a label");

        let id = self.label.clone();
        let mut clicked = false;
        if let Some(last_widget) = renderer.get_widget_cached(&id) {
            let mouse_pos = input::mouse_pos(ctx);
            let mouse_up = input::mouse_button_released(ctx, input::MouseButton::Left);
            let mouse_down = input::mouse_button_just_pressed(ctx, input::MouseButton::Left);
            let inside = collision::point_aabb_collision(
                mouse_pos,
                AABB::from_top_left(
                    last_widget.computed_pos_margin(),
                    last_widget.computed_size_margin(),
                ),
            );

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
        let index = renderer.insert_widget(self);

        ButtonResult { index, clicked }
    }

    pub fn slider(
        self,
        ctx: &Context,
        renderer: &mut GUIRenderer,
        min: f32,
        max: f32,
        value: &mut f32,
    ) -> SliderResult {
        debug_assert!(!self.label.is_empty(), "ui slider must have a label");

        let id = self.label.clone();
        if let Some(last_widget) = renderer.get_widget_cached(&id) {
            let bounds = AABB::from_top_left(
                last_widget.computed_pos + last_widget.margin,
                last_widget.computed_size - last_widget.margin * 2.0,
            );

            let mouse_pos = input::mouse_pos(ctx);
            let mouse_down = input::mouse_button_just_pressed(ctx, input::MouseButton::Left);
            let inside = collision::point_aabb_collision(mouse_pos, bounds);

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

        let index = renderer.insert_widget(self);

        SliderResult {
            index,
            pos: slider_pos,
        }
    }

    pub fn layout(
        self,
        renderer: &mut GUIRenderer,
        children: impl FnOnce(&mut GUIRenderer),
    ) -> LayoutResult {
        let index = renderer.insert_widget(self);

        renderer.push_layout(index);
        children(renderer);
        renderer.pop_layout();

        LayoutResult { index }
    }

    pub fn button_layout(
        self,
        ctx: &Context,
        renderer: &mut GUIRenderer,
        children: impl FnOnce(&mut GUIRenderer, ButtonResult),
    ) -> ButtonResult {
        let result = self.button(ctx, renderer);

        renderer.push_layout(result.index);
        children(renderer, result);
        renderer.pop_layout();

        result
    }

    pub fn slider_layout(
        self,
        ctx: &Context,
        renderer: &mut GUIRenderer,
        min: f32,
        max: f32,
        value: &mut f32,
        children: impl FnOnce(&mut GUIRenderer, SliderResult),
    ) -> SliderResult {
        let result = self.slider(ctx, renderer, min, max, value);

        renderer.push_layout(result.index);
        children(renderer, result);
        renderer.pop_layout();

        result
    }

    /// Top left position
    ///
    /// Including margin and padding
    pub(crate) fn computed_pos_margin_padding(&self) -> Vec2 {
        self.computed_pos + self.margin + self.padding
    }

    /// Size
    ///
    /// Including margin and padding
    pub(crate) fn computed_size_margin_padding(&self) -> Vec2 {
        self.computed_size - self.margin * 2.0 - self.padding * 2.0
    }

    /// Top left position
    ///
    /// Including margin
    pub(crate) fn computed_pos_margin(&self) -> Vec2 {
        self.computed_pos + self.margin
    }

    /// Size
    ///
    /// Including margin
    pub(crate) fn computed_size_margin(&self) -> Vec2 {
        self.computed_size - self.margin * 2.0
    }

    /// Top left position
    ///
    /// Including padding
    pub(crate) fn computed_pos_padding(&self) -> Vec2 {
        self.computed_pos + self.padding
    }

    /// Size
    ///
    /// Including padding
    pub(crate) fn computed_size_padding(&self) -> Vec2 {
        self.computed_size - self.padding * 2.0
    }
}

// Builder methods
impl Widget {
    /// add label to identify widget for interactions
    pub fn label(mut self, value: impl Into<String>) -> Self {
        self.label = value.into();
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
    /// set horizontal / vertical padding
    pub fn padding_hv(mut self, value: Vec2) -> Self {
        self.padding = value;
        self
    }

    /// set uniform margin
    pub fn margin(mut self, value: f32) -> Self {
        self.margin = vec2(value, value);
        self
    }
    /// set horizontal / vertical margin
    pub fn margin_hv(mut self, value: Vec2) -> Self {
        self.margin = value;
        self
    }

    /// set uniform border radius
    pub fn border_radius(mut self, value: f32) -> Self {
        self.border_radius = vec4(value, value, value, value);
        self
    }
    /// set uniform border radius for top corners
    pub fn border_radius_top(mut self, value: f32) -> Self {
        self.border_radius.x = value;
        self.border_radius.y = value;
        self
    }
    /// set uniform border radius for bottom corners
    pub fn border_radius_bottom(mut self, value: f32) -> Self {
        self.border_radius.z = value;
        self.border_radius.w = value;
        self
    }
    /// set uniform border radius for left corners
    pub fn border_radius_left(mut self, value: f32) -> Self {
        self.border_radius.x = value;
        self.border_radius.w = value;
        self
    }
    /// set uniform border radius for right corners
    pub fn border_radius_right(mut self, value: f32) -> Self {
        self.border_radius.y = value;
        self.border_radius.z = value;
        self
    }
    /// set uniform border radius for all corners
    pub fn border_radius_all(mut self, tl: f32, tr: f32, br: f32, bl: f32) -> Self {
        self.border_radius = vec4(tl, tr, br, bl);
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
        self.color = Some(value);
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
        self.font_size = value;
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

#[derive(Debug, Clone, Copy)]
pub struct ButtonResult {
    index: usize,
    pub clicked: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct LayoutResult {
    index: usize,
}

#[derive(Debug, Clone, Copy)]
pub struct SliderResult {
    /// index of this widget
    index: usize,
    /// \[0,1\] range for child positioning
    ///
    /// The actual value is passed by mutable reference
    pub pos: f32,
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
        border_radius: Vec4::ZERO,

        gap: 0.0,

        main_axis_alignment: Alignment::Start,
        cross_axis_alignment: Alignment::Start,

        color: None,

        text: String::new(),
        text_color: Vec4::ZERO,
        font_size: 0.0,
        text_wrap: false,

        computed_pos: vec2(0.0, 0.0),
        computed_size: vec2(screen_size.width as f32, screen_size.height as f32),

        children: Vec::new(),
    }
}
