use glam::Vec2;

pub type Point = Vec2;

#[derive(Clone, Copy)]
pub struct Quad {
    pub pos: Vec2,
    pub size: Vec2,
}

impl Quad {
    pub fn new(origin: Vec2, dimension: Vec2) -> Self {
        Self {
            pos: origin,
            size: dimension,
        }
    }
    pub fn left(&self) -> f32 {
        self.pos.x
    }
    pub fn right(&self) -> f32 {
        self.pos.x + self.size.x
    }
    pub fn top(&self) -> f32 {
        self.pos.y
    }
    pub fn bottom(&self) -> f32 {
        self.pos.y + self.size.y
    }
}

pub fn point_quad_collision(point: Point, quad: Quad) -> bool {
    let miss_x = point.x < quad.left() || point.x > quad.right();
    let miss_y = point.y < quad.top() || point.y > quad.bottom();
    !(miss_x || miss_y)
}

pub fn quad_quad_collision(quad1: Quad, quad2: Quad) -> bool {
    let miss_x = quad1.right() < quad2.left() || quad1.left() > quad2.right();
    let miss_y = quad1.bottom() < quad2.top() || quad1.top() > quad2.bottom();

    !(miss_x || miss_y)
}

mod test {
    #![allow(unused_imports)]
    use super::quad_quad_collision;
    use super::Quad;
    use glam::vec2;

    #[test]
    fn quad_self_intersect() {
        let q1 = Quad::new(vec2(0.0, 0.0), vec2(1.0, 1.0));
        assert!(quad_quad_collision(q1, q1));
    }

    #[test]
    fn tangent_quads() {
        let q1 = Quad::new(vec2(0.0, 0.0), vec2(1.0, 1.0));
        let q2 = Quad::new(vec2(1.0, 0.0), vec2(1.0, 1.0));
        assert!(quad_quad_collision(q1, q2));
    }
}
