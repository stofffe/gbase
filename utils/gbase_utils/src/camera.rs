use std::f32::consts::PI;

use encase::ShaderType;
use gbase::{
    glam::{vec3, Mat4, Vec3},
    input, log, render, time, winit, Context,
};

#[derive(ShaderType)]
pub struct CameraUniform {
    pos: Vec3,
    facing: Vec3,

    view: Mat4,
    proj: Mat4,
    view_proj: Mat4,

    inv_view: Mat4,
    inv_proj: Mat4,
    inv_view_proj: Mat4,
}

#[derive(Debug)]
pub enum CameraProjection {
    Perspective { fov: f32 },
    Orthographic { height: f32 },
}

impl CameraProjection {
    pub fn perspective(fov: f32) -> Self {
        Self::Perspective { fov }
    }
    pub fn orthographic(height: f32) -> Self {
        Self::Orthographic { height }
    }
}

#[derive(Debug)]
pub struct Camera {
    pub pos: Vec3,
    pub yaw: f32,
    pub pitch: f32,

    pub znear: f32,
    pub zfar: f32,

    pub projection: CameraProjection,
    // TODO: add aspect here
}

impl Camera {
    pub fn new(projection: CameraProjection) -> Self {
        Self {
            pos: Vec3::ZERO,
            yaw: 0.0,
            pitch: 0.0,

            znear: 0.01,
            zfar: 1000.0,

            projection,
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

    pub fn uniform(&self, ctx: &Context) -> CameraUniform {
        let pos = self.pos;
        let facing = self.forward();

        let aspect_ratio = render::aspect_ratio(ctx);
        let view = Mat4::look_to_rh(self.pos, self.forward(), self.up());
        let proj = match self.projection {
            CameraProjection::Perspective { fov } => {
                Mat4::perspective_rh(fov, aspect_ratio, self.znear, self.zfar)
            }
            CameraProjection::Orthographic { height } => Mat4::orthographic_rh(
                aspect_ratio * -height / 2.0,
                aspect_ratio * height / 2.0,
                -height / 2.0,
                height / 2.0,
                self.znear,
                self.zfar,
            ),
        };
        let view_proj = proj * view;

        let inv_view = view.inverse();
        let inv_proj = proj.inverse();
        let inv_view_proj = view_proj.inverse();

        CameraUniform {
            pos,
            facing,
            view,
            proj,
            view_proj,
            inv_view,
            inv_proj,
            inv_view_proj,
        }
    }

    pub fn zoom(&mut self, amount: f32) {
        match &mut self.projection {
            CameraProjection::Perspective { fov } => *fov += amount,
            CameraProjection::Orthographic { height } => *height += amount,
        };
    }

    /// Simple controls for free flying camera
    pub fn flying_controls(&mut self, ctx: &Context) {
        let dt = time::delta_time(ctx);

        // Camera rotation
        let (mouse_dx, mouse_dy) = input::mouse_delta(ctx);
        self.yaw -= 1.0 * dt * mouse_dx;
        self.pitch -= 1.0 * dt * mouse_dy;
        self.pitch = self.pitch.clamp(-PI / 2.0 + 0.01, PI / 2.0 - 0.01);

        // Camera movement
        let mut camera_movement_dir = Vec3::ZERO;
        if input::key_pressed(ctx, winit::keyboard::KeyCode::KeyW) {
            camera_movement_dir += self.forward();
        }
        if input::key_pressed(ctx, winit::keyboard::KeyCode::KeyS) {
            camera_movement_dir -= self.forward();
        }
        if input::key_pressed(ctx, winit::keyboard::KeyCode::KeyA) {
            camera_movement_dir -= self.right();
        }
        if input::key_pressed(ctx, winit::keyboard::KeyCode::KeyD) {
            camera_movement_dir += self.right();
        }
        camera_movement_dir.y = 0.0;
        if input::key_pressed(ctx, winit::keyboard::KeyCode::Space) {
            camera_movement_dir += self.world_up();
        }
        if input::key_pressed(ctx, winit::keyboard::KeyCode::ShiftLeft) {
            camera_movement_dir -= self.world_up();
        }
        const CAMERA_MOVE_SPEED: f32 = 50.0;
        if camera_movement_dir != Vec3::ZERO {
            if input::key_pressed(ctx, winit::keyboard::KeyCode::KeyM) {
                self.pos += camera_movement_dir.normalize() * dt * CAMERA_MOVE_SPEED / 10.0;
            } else {
                self.pos += camera_movement_dir.normalize() * dt * CAMERA_MOVE_SPEED;
            }
        }
    }
}

impl Camera {
    pub fn pos(mut self, value: Vec3) -> Self {
        self.pos = value;
        self
    }
    pub fn yaw(mut self, value: f32) -> Self {
        self.yaw = value;
        self
    }
    pub fn pitch(mut self, value: f32) -> Self {
        self.pitch = value;
        self
    }
    pub fn znear(mut self, value: f32) -> Self {
        self.znear = value;
        self
    }
    pub fn zfar(mut self, value: f32) -> Self {
        self.zfar = value;
        self
    }
}

pub struct FrustumPlane {
    pub origin: Vec3,
    pub normal: Vec3,
}

impl FrustumPlane {
    pub fn point_in_front(&self, point: Vec3) -> bool {
        Vec3::dot(point - self.origin, self.normal) >= 0.0
    }
    pub fn sphere_in_front(&self, point: Vec3, radius: f32) -> bool {
        Vec3::dot(point - self.origin, self.normal) + radius >= 0.0
    }
}

pub struct CameraFrustum {
    pub near: FrustumPlane,
    pub far: FrustumPlane,
    pub left: FrustumPlane,
    pub right: FrustumPlane,
    pub bottom: FrustumPlane,
    pub top: FrustumPlane,
}

impl CameraFrustum {
    pub fn point_inside(&self, point: Vec3) -> bool {
        self.near.point_in_front(point)
            && self.far.point_in_front(point)
            && self.left.point_in_front(point)
            && self.right.point_in_front(point)
            && self.bottom.point_in_front(point)
            && self.top.point_in_front(point)
    }

    pub fn sphere_inside(&self, point: Vec3, radius: f32) -> bool {
        self.near.point_in_front(point)
            && self.far.sphere_in_front(point, radius)
            && self.left.sphere_in_front(point, radius)
            && self.right.sphere_in_front(point, radius)
            && self.bottom.sphere_in_front(point, radius)
            && self.top.sphere_in_front(point, radius)
    }
}

impl Camera {
    // TODO: cache in camera?
    pub fn calculate_frustum(&self, ctx: &Context) -> CameraFrustum {
        let cam_forward = self.forward();
        let cam_right = self.right();
        let cam_up = self.up();

        match self.projection {
            CameraProjection::Orthographic { height } => todo!(),
            CameraProjection::Perspective { fov } => {
                let aspect_ratio = render::aspect_ratio(ctx);
                let half_far_height = self.zfar * f32::tan(fov / 2.0);
                let half_far_width = half_far_height * aspect_ratio;

                let far_left = cam_forward * self.zfar - cam_right * half_far_width;
                let far_right = cam_forward * self.zfar + cam_right * half_far_width;
                let far_bottom = cam_forward * self.zfar - cam_up * half_far_height;
                let far_top = cam_forward * self.zfar + cam_up * half_far_height;

                let near = FrustumPlane {
                    origin: self.pos + self.znear * cam_forward,
                    normal: cam_forward.normalize(),
                };
                let far = FrustumPlane {
                    origin: self.pos + self.zfar * cam_forward,
                    normal: -cam_forward.normalize(),
                };
                let left = FrustumPlane {
                    origin: self.pos,
                    normal: Vec3::cross(far_left, cam_up).normalize(),
                };
                let right = FrustumPlane {
                    origin: self.pos,
                    normal: Vec3::cross(cam_up, far_right).normalize(),
                };
                let bottom = FrustumPlane {
                    origin: self.pos,
                    normal: Vec3::cross(cam_right, far_bottom).normalize(),
                };
                let top = FrustumPlane {
                    origin: self.pos,
                    normal: Vec3::cross(far_top, cam_right).normalize(),
                };
                CameraFrustum {
                    near,
                    far,
                    left,
                    right,
                    bottom,
                    top,
                }
            }
        }
    }
}
