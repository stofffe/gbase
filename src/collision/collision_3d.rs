use encase::ShaderType;
use glam::Vec3;

#[derive(Clone, Copy, ShaderType)]
pub struct Box3D {
    pub origin: Vec3,
    pub dimension: Vec3,
}

impl Box3D {
    pub fn new(origin: Vec3, dimension: Vec3) -> Self {
        Self { origin, dimension }
    }
    pub fn x_min(&self) -> f32 {
        self.origin.x
    }
    pub fn x_max(&self) -> f32 {
        self.origin.x + self.dimension.x
    }
    pub fn y_min(&self) -> f32 {
        self.origin.y
    }
    pub fn y_max(&self) -> f32 {
        self.origin.y + self.dimension.y
    }
    pub fn z_min(&self) -> f32 {
        self.origin.z
    }
    pub fn z_maz(&self) -> f32 {
        self.origin.z + self.dimension.z
    }
}

pub fn point_box3d_collision(point: Vec3, box_3d: Box3D) -> bool {
    let collides_x = point.x >= box_3d.x_min() && point.x <= box_3d.x_max();
    let collides_y = point.y >= box_3d.y_min() && point.y <= box_3d.y_max();
    let collides_z = point.x >= box_3d.z_min() && point.x <= box_3d.x_max();
    collides_x && collides_y && collides_z
}
