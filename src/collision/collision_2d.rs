use glam::Vec2;

#[derive(Clone, Copy)]
pub struct Quad {
    pub origin: Vec2,
    pub dimension: Vec2,
}

impl Quad {
    pub fn new(origin: Vec2, dimension: Vec2) -> Self {
        Self { origin, dimension }
    }
    pub fn left(&self) -> f32 {
        self.origin.x
    }
    pub fn right(&self) -> f32 {
        self.origin.x + self.dimension.x
    }
    pub fn top(&self) -> f32 {
        self.origin.y
    }
    pub fn bottom(&self) -> f32 {
        self.origin.y + self.dimension.y
    }
}

pub fn point_quad_collision(point: Vec2, quad: Quad) -> bool {
    let collides_x = point.x >= quad.left() && point.x <= quad.right();
    let collides_y = point.y >= quad.top() && point.y <= quad.bottom();
    collides_x && collides_y
}
