use encase::ShaderType;
use glam::Vec3;

use crate::render::Transform;

#[derive(Debug, Clone, Copy, ShaderType)]
pub struct Box3D {
    pub min: Vec3,
    pub max: Vec3,
}

impl Box3D {
    pub fn new(center: Vec3, dim: Vec3) -> Self {
        let half_size = dim * 0.5;
        debug_assert!(half_size.x >= 0.0 && half_size.y >= 0.0 && half_size.z >= 0.0);
        Self {
            min: center - half_size,
            max: center + half_size,
        }
    }

    pub fn to_transform(&self) -> Transform {
        Transform {
            pos: (self.min + self.max) * 0.5,
            rot: glam::Quat::IDENTITY,
            scale: self.max - self.min,
        }
    }
}

pub fn point_box3d_collision(point: Vec3, box_3d: Box3D) -> bool {
    let collides_x = point.x >= box_3d.min.x && point.x <= box_3d.max.x;
    let collides_y = point.y >= box_3d.min.y && point.y <= box_3d.max.y;
    let collides_z = point.x >= box_3d.min.z && point.x <= box_3d.max.z;
    collides_x && collides_y && collides_z
}
