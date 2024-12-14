use crate::{collision::Quad, Context};
use glam::{vec2, vec4, Vec2, Vec4};

use super::GUIRenderer;

pub struct Button {
    origin: Vec2,
    dimension: Vec2,
    color: Vec4,
}

impl Button {
    pub fn new() -> Self {
        Self {
            origin: vec2(0.0, 0.0),
            dimension: vec2(1.0, 1.0),
            color: vec4(0.0, 0.0, 0.0, 1.0),
        }
    }

    pub fn render(&self, _ctx: &Context, renderer: &mut GUIRenderer) {
        renderer.quad(
            Quad {
                origin: self.origin,
                dimension: self.dimension,
            },
            self.color,
        );
    }
}

impl Button {
    pub fn origin(mut self, value: Vec2) -> Self {
        self.origin = value;
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
}
