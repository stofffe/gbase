use gbase::{
    glam::Vec4,
    glam::{vec2, Vec2},
};

#[derive(Debug)]
struct UIElement {
    pos: Vec2,
    dimensions: Vec2,
    background_color: Vec4,

    parent: usize,
    children: Vec<usize>,
}

impl UIElement {
    pub fn new() -> Self {
        Self {
            pos: vec2(0.0, 0.0),
            dimensions: vec2(0.0, 0.0),
            children: Vec::new(),
            background_color: todo!(),
            parent: 0,
        }
    }

    //
    // Attributes
    //

    pub fn pos(mut self, pos: Vec2) -> Self {
        self.pos = pos;
        self
    }
    pub fn dimensions(mut self, dimensions: Vec2) -> Self {
        self.dimensions = dimensions;
        self
    }
    pub fn background_color(mut self, background_color: Vec4) -> Self {
        self.background_color = background_color;
        self
    }

    //
    // Layout and interaction
    //

    pub fn layout(mut self, children: fn(&mut Self)) -> Self {
        children(&mut self);

        self.close();

        self
    }

    pub fn close(&mut self) {}
}

struct UILayoutState {
    root: UIElement,
}

impl UILayoutState {
    pub fn new() -> Self {
        let root = UIElement::new();
        Self { root }
    }
}
