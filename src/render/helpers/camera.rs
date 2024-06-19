use crate::Context;
use encase::ShaderType;
use glam::{vec3, Mat4, Vec3};
use std::f32::consts::PI;

/// Right handed perspective camera
///
/// yaw = pitch = 0
/// => forw  (0, 0, -1)
/// => right (1, 0, 0)
/// => up    (0, 1, 0)
pub struct PerspectiveCamera {
    pub pos: Vec3,
    pub yaw: f32,
    pub pitch: f32,

    pub fov: f32,
    pub znear: f32,
    pub zfar: f32,
}

impl PerspectiveCamera {
    pub fn new() -> Self {
        Self {
            pos: Vec3::ZERO,
            yaw: 0.0,
            pitch: 0.0,
            fov: PI / 2.0,
            znear: 0.01,
            zfar: 1000.0,
        }
    }

    pub fn world_up(&self) -> Vec3 {
        vec3(0.0, 1.0, 0.0)
    }

    pub fn forward(&self) -> Vec3 {
        vec3(
            -self.yaw.sin() * self.pitch.cos(),
            self.pitch.sin(),
            -self.yaw.cos() * self.pitch.cos(),
        )
        .normalize()
    }
    #[rustfmt::skip]
    pub fn right(&self) -> Vec3 {
        vec3(
            self.yaw.cos(),
            0.0,
            -self.yaw.sin(),
        )
    }
    pub fn up(&self) -> Vec3 {
        vec3(
            self.yaw.sin() * self.pitch.sin(),
            self.pitch.cos(),
            self.yaw.cos() * self.pitch.sin(),
        )
        .normalize()
    }

    // Make this non mut?
    pub fn uniform(&mut self, ctx: &Context) -> PerspectiveCameraUniform {
        const MIN_PITCH: f32 = -PI / 2.0 + 0.1;
        const MAX_PITCH: f32 = PI / 2.0 - 0.1;
        const MIN_FOV: f32 = 0.1;
        const MAX_FOV: f32 = PI - 0.1;

        // left handed coords
        self.pitch = self.pitch.clamp(MIN_PITCH, MAX_PITCH);
        self.fov = self.fov.clamp(MIN_FOV, MAX_FOV);
        let view = Mat4::look_to_rh(self.pos, self.forward(), self.up());
        let aspect = ctx.render.aspect_ratio();
        let proj = Mat4::perspective_rh(self.fov, aspect, self.znear, self.zfar);

        let view_proj = proj * view;

        let pos = self.pos;
        let facing = self.forward();

        PerspectiveCameraUniform {
            view_proj,
            pos,
            facing,
        }
    }
}

#[derive(ShaderType)]
pub struct PerspectiveCameraUniform {
    view_proj: Mat4,
    pos: Vec3,
    facing: Vec3,
}
