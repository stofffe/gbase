use super::{GUIRenderer, UiID};
use crate::{
    collision::{self, Quad},
    input, Context,
};
use glam::{vec2, vec4, Vec2, Vec4};
use winit::event::MouseButton;

// what should all ui components have?
// - id
// - origin
// - dimensions

pub struct Element {
    id: UiID,
    origin: Vec2,
    dimension: Vec2,
}

#[derive(Clone, Debug)]
pub struct Button {
    origin: Vec2,
    dimension: Vec2,
    color: Vec4,
    label: String,
}

#[derive(Clone, Copy, Debug)]
pub struct ButtonResult {
    pub clicked: bool,
    pub origin: Vec2,
    pub id: UiID,
}

impl Button {
    pub fn new_with_parent(parent: ButtonResult) -> Self {
        Self {
            origin: parent.origin,
            dimension: vec2(1.0, 1.0),
            color: vec4(0.0, 0.0, 0.0, 1.0),
            label: String::new(),
        }
    }
    pub fn new() -> Self {
        Self {
            origin: vec2(0.0, 0.0),
            dimension: vec2(1.0, 1.0),
            color: vec4(0.0, 0.0, 0.0, 1.0),
            label: String::new(),
        }
    }

    pub fn render(&self, ctx: &Context, renderer: &mut GUIRenderer) -> ButtonResult {
        let id = UiID::new(&self.label);
        let bounds = Quad::new(self.origin, self.dimension);

        let mouse_up = input::mouse_button_released(ctx, MouseButton::Left);
        let mouse_down = input::mouse_button_just_pressed(ctx, MouseButton::Left);
        let inside = collision::point_quad_collision(input::mouse_pos_unorm(ctx), bounds);

        let mut clicked = false;

        if inside {
            renderer.set_hot_this_frame(id);

            // active
            if renderer.check_hot(id) {
                if renderer.check_active(id) && mouse_up {
                    clicked = true;
                } else if mouse_down {
                    renderer.set_active(id);
                }
            }
        }

        renderer.quad(bounds, self.color);

        ButtonResult {
            clicked,
            id,
            origin: self.origin,
        }
    }
    pub fn render_with_children(
        &self,
        ctx: &Context,
        renderer: &mut GUIRenderer,
        children: impl Fn(&mut GUIRenderer, Quad, bool),
    ) -> bool {
        let result = self.render(ctx, renderer);

        children(
            renderer,
            Quad::new(self.origin, self.dimension),
            result.clicked,
        );

        result.clicked
    }
}

impl Button {
    pub fn origin(mut self, value: Vec2) -> Self {
        self.origin += value;
        self
    }
    pub fn dimension(mut self, value: Vec2) -> Self {
        self.dimension = value;
        self
    }
    pub fn color(mut self, value: Vec4) -> Self {
        self.color = value;
        self
    }
    pub fn label(mut self, value: impl Into<String>) -> Self {
        self.label = value.into();
        self
    }
}
