use encase::ShaderType;
use gbase::glam::{self, Mat4, Quat, Vec2, Vec3, Vec3Swizzles};

//
// Transform
//

#[derive(Debug, Clone)]
pub struct Transform2D {
    pub pos: Vec2,
    pub rot: f32,
    pub scale: Vec2,
}

impl Transform2D {
    pub const fn new(pos: Vec2, rot: f32, scale: Vec2) -> Self {
        Self { pos, rot, scale }
    }

    pub const fn from_pos(pos: Vec2) -> Self {
        Self::new(pos, 0.0, Vec2::ONE)
    }
    pub const fn from_rot(rot: f32) -> Self {
        Self::new(Vec2::ZERO, rot, Vec2::ONE)
    }
    pub const fn from_scale(scale: Vec2) -> Self {
        Self::new(Vec2::ZERO, 0.0, scale)
    }
    pub const fn from_pos_rot(pos: Vec2, rot: f32) -> Self {
        Self::new(pos, rot, Vec2::ONE)
    }
    pub const fn from_pos_scale(pos: Vec2, scale: Vec2) -> Self {
        Self::new(pos, 0.0, scale)
    }
    pub const fn from_rot_scale(rot: f32, scale: Vec2) -> Self {
        Self::new(Vec2::ZERO, rot, scale)
    }

    pub const fn with_pos(mut self, pos: Vec2) -> Self {
        self.pos = pos;
        self
    }
    pub const fn with_rot(mut self, rot: f32) -> Self {
        self.rot = rot;
        self
    }
    pub const fn with_scale(mut self, scale: Vec2) -> Self {
        self.scale = scale;
        self
    }

    pub fn set_pos(&mut self, pos: Vec2) {
        self.pos = pos;
    }
    pub fn set_rot(&mut self, rot: f32) {
        self.rot = rot;
    }
    pub fn set_scale(&mut self, scale: Vec2) {
        self.scale = scale;
    }

    pub fn matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(
            self.scale.extend(1.0),
            Quat::from_rotation_z(self.rot),
            self.pos.extend(0.0),
        )
    }
    pub fn from_matrix(matrix: Mat4) -> Self {
        let (scale, rot, pos) = matrix.to_scale_rotation_translation();
        Self {
            pos: pos.xy(),
            rot: rot.to_euler(glam::EulerRot::XYZ).2,
            scale: scale.xy(),
        }
    }

    pub fn uniform(&self) -> TransformUniform {
        TransformUniform {
            matrix: self.matrix(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Transform3D {
    pub pos: Vec3,
    pub rot: Quat,
    pub scale: Vec3,
}

impl Transform3D {
    pub const fn new(pos: Vec3, rot: Quat, scale: Vec3) -> Self {
        Self { pos, rot, scale }
    }
    pub const fn from_pos(pos: Vec3) -> Self {
        Self::new(pos, Quat::IDENTITY, Vec3::ONE)
    }
    pub const fn from_rot(rot: Quat) -> Self {
        Self::new(Vec3::ZERO, rot, Vec3::ONE)
    }
    pub const fn from_scale(scale: Vec3) -> Self {
        Self::new(Vec3::ZERO, Quat::IDENTITY, scale)
    }
    pub const fn from_pos_rot(pos: Vec3, rot: Quat) -> Self {
        Self::new(pos, rot, Vec3::ONE)
    }
    pub const fn from_pos_scale(pos: Vec3, scale: Vec3) -> Self {
        Self::new(pos, Quat::IDENTITY, scale)
    }
    pub const fn from_rot_scale(rot: Quat, scale: Vec3) -> Self {
        Self::new(Vec3::ZERO, rot, scale)
    }

    pub const fn with_pos(mut self, pos: Vec3) -> Self {
        self.pos = pos;
        self
    }
    pub const fn with_rot(mut self, rot: Quat) -> Self {
        self.rot = rot;
        self
    }
    pub const fn with_scale(mut self, scale: Vec3) -> Self {
        self.scale = scale;
        self
    }

    pub fn set_pos(&mut self, pos: Vec3) {
        self.pos = pos;
    }
    pub fn set_rot(&mut self, rot: Quat) {
        self.rot = rot;
    }
    pub fn set_scale(&mut self, scale: Vec3) {
        self.scale = scale;
    }

    pub fn matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(self.scale, self.rot, self.pos)
    }
    pub fn from_matrix(matrix: Mat4) -> Self {
        let (scale, rot, pos) = matrix.to_scale_rotation_translation();
        Self { pos, rot, scale }
    }

    pub fn uniform(&self) -> TransformUniform {
        TransformUniform {
            matrix: self.matrix(),
        }
    }
}

impl Default for Transform3D {
    fn default() -> Self {
        Self {
            pos: Vec3::ZERO,
            rot: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }
}

impl Default for Transform2D {
    fn default() -> Self {
        Self {
            pos: Vec2::ZERO,
            rot: 0.0,
            scale: Vec2::ONE,
        }
    }
}

#[derive(ShaderType)]
pub struct TransformUniform {
    matrix: Mat4,
}
