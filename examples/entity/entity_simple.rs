use glam::Vec2;

// Caveman approach

struct Player {
    pos: Vec2,
    health: f32,
}

struct Wall {
    pos: Vec2,
    size: Vec2,
}

struct Enemy {
    pos: Vec2,
    attack: Vec2,
}

fn raycast() {}
