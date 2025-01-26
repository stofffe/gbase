use glam::Vec2;

//
// Types
//

pub type Point = Vec2;

#[derive(Clone, Copy, Debug)]
pub struct AABB {
    pub pos: Vec2,
    pub size: Vec2,
}

impl AABB {
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

#[derive(Clone, Copy, Debug)]
pub struct Circle {
    pub origin: Vec2,
    pub radius: f32,
}

impl Circle {
    pub fn new(origin: Vec2, radius: f32) -> Self {
        Self { origin, radius }
    }
}

//
// Collisions
//

pub fn point_aabb_collision(point: Point, quad: AABB) -> bool {
    let miss_x = point.x < quad.left() || point.x > quad.right();
    let miss_y = point.y < quad.top() || point.y > quad.bottom();
    !(miss_x || miss_y)
}

pub fn point_circle_collision(point: Point, circle: Circle) -> bool {
    point.distance(circle.origin) <= circle.radius
}

pub fn point_point_collision(point1: Point, point2: Point) -> bool {
    point1 == point2
}

pub fn circle_circle_collision(circle1: Circle, circle2: Circle) -> bool {
    let dx = circle1.origin.x - circle2.origin.x;
    let dy = circle1.origin.y - circle2.origin.y;
    let radius_sum = circle1.radius + circle2.radius;

    (dx * dx + dy * dy) <= (radius_sum * radius_sum)
}

pub fn circle_aabb_collision(circle: Circle, quad: AABB) -> bool {
    let closest_x = circle.origin.x.clamp(quad.left(), quad.right());
    let closest_y = circle.origin.y.clamp(quad.top(), quad.bottom());

    let distance_x = circle.origin.x - closest_x;
    let distance_y = circle.origin.y - closest_y;

    (distance_x * distance_x + distance_y * distance_y) <= (circle.radius * circle.radius)
}

pub fn aabb_aabb_collision(quad1: AABB, quad2: AABB) -> bool {
    let miss_x = quad1.right() < quad2.left() || quad1.left() > quad2.right();
    let miss_y = quad1.bottom() < quad2.top() || quad1.top() > quad2.bottom();

    !(miss_x || miss_y)
}

mod test {
    #![allow(unused_imports)]
    use super::{
        aabb_aabb_collision, circle_aabb_collision, circle_circle_collision, point_point_collision,
        Circle, AABB,
    };
    use glam::vec2;

    //
    // Self intersection
    //

    #[test]
    fn point_self_intersect() {
        let c1 = vec2(1.0, 1.0);
        assert!(point_point_collision(c1, c1));
    }

    #[test]
    fn circle_self_intersect() {
        let p1 = Circle::new(vec2(3.0, 4.0), 5.0);
        assert!(circle_circle_collision(p1, p1));
    }

    #[test]
    fn aabb_self_intersect() {
        let q1 = AABB::new(vec2(0.0, 0.0), vec2(1.0, 1.0));
        assert!(aabb_aabb_collision(q1, q1));
    }

    //
    // Tangent
    //

    #[test]
    fn aabb_tangent_aabb() {
        let q1 = AABB::new(vec2(-1.0, 0.0), vec2(2.0, 1.0));
        let q2 = AABB::new(vec2(1.0, 0.0), vec2(1.0, 3.0));
        assert!(aabb_aabb_collision(q1, q2));
    }

    #[test]
    fn circle_tangent_circle() {
        let c1 = Circle::new(vec2(-1.0, 0.0), 2.0);
        let c2 = Circle::new(vec2(2.0, 0.0), 1.0);
        assert!(circle_circle_collision(c1, c2));
    }

    #[test]
    fn circle_tangent_aabb() {
        let q = AABB::new(vec2(0.0, 0.0), vec2(2.0, 2.0));
        let c = Circle::new(vec2(2.0, 0.0), 1.0);
        assert!(circle_aabb_collision(c, q));
    }

    //
    // Inside
    //

    #[test]
    fn circle_inside_aabb() {
        let q = AABB::new(vec2(0.0, 0.0), vec2(10.0, 10.0));
        let c = Circle::new(vec2(1.0, -1.0), 2.0);
        assert!(circle_aabb_collision(c, q));
    }

    #[test]
    fn circle_inside_circle() {
        let c1 = Circle::new(vec2(1.0, 1.0), 3.0);
        let c2 = Circle::new(vec2(1.0, 1.0), 1.0);
        assert!(circle_circle_collision(c1, c2));
    }

    #[test]
    fn circle_outside_aabb() {
        let c = Circle::new(vec2(3.0, 2.0), 1.0);
        let q = AABB::new(vec2(0.0, 0.0), vec2(2.0, 1.0));
        assert!(!circle_aabb_collision(c, q));
    }
}
