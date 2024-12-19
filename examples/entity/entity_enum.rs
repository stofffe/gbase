use glam::{vec2, Vec2};

// Enum Approach

struct Entity {
    handle: u64,
    variant: EntityVariant,
}

// macro_rules! entity_force {
//     ($typ:ty) => {
//
//
//     };
// }

const PLAYER_REF: &EntityVariant = &EntityVariant::Player {
    pos: Vec2::ZERO,
    health: 0.0,
};

impl Entity {
    fn is_player(&self) -> bool {
        std::mem::discriminant(&self.variant) == std::mem::discriminant(PLAYER_REF)
    }

    fn attack(&self, other: &mut Entity) {}

    fn hurt_player(&mut self, damage: f32) {
        let EntityVariant::Player { pos, health } = &mut self.variant else {
            return;
        };

        *health -= damage;
    }
}

struct Player {
    pos: Vec2,
    health: f32,
}

enum EntityVariant {
    Player { pos: Vec2, health: f32 },

    Wall { pos: Vec2, size: Vec2 },

    Enemy { pos: Vec2, attack: f32 },
}

struct Game {
    entites: Vec<Entity>,
}

fn test() {
    let enemy = Entity {
        handle: 0,
        variant: EntityVariant::Enemy {
            pos: vec2(0.0, 0.0),
            attack: 1.0,
        },
    };

    let player = Entity {
        handle: 1,
        variant: EntityVariant::Player {
            pos: vec2(0.0, 0.0),
            health: 10.0,
        },
    };

    // player raycasts down
    let hits = Vec::<Entity>::new();
    for hit in hits {}

    // enemy attack player
}
