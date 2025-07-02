use std::f32::consts::PI;

use encase::ShaderType;
use gbase::{
    glam::{vec3, Mat4, Vec3, Vec4Swizzles},
    input,
    render::{self, BoundingBox},
    time, winit, Context,
};

use crate::Transform3D;

#[derive(ShaderType)]
pub struct CameraUniform {
    pub pos: Vec3,
    pub near: f32,
    pub facing: Vec3,
    pub far: f32,

    pub view: Mat4,
    pub proj: Mat4,
    pub view_proj: Mat4,

    pub inv_view: Mat4,
    pub inv_proj: Mat4,
    pub inv_view_proj: Mat4,
}

#[derive(Debug, Clone)]
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

#[derive(Debug, Clone)]
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

    pub fn view_matrix(&self) -> Mat4 {
        Mat4::look_to_rh(self.pos, self.forward(), self.up())
    }

    pub fn projection_matrix(&self, ctx: &Context) -> Mat4 {
        let aspect_ratio = render::aspect_ratio(ctx);
        match self.projection {
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
        }
    }

    pub fn view_projection_matrix(&self, ctx: &Context) -> Mat4 {
        self.projection_matrix(ctx) * self.view_matrix()
    }

    pub fn uniform(&self, ctx: &Context) -> CameraUniform {
        let pos = self.pos;
        let facing = self.forward();

        let view = self.view_matrix();
        let proj = self.projection_matrix(ctx);
        let view_proj = proj * view;

        let inv_view = view.inverse();
        let inv_proj = proj.inverse();
        let inv_view_proj = view_proj.inverse();

        CameraUniform {
            pos,
            near: self.znear,
            facing,
            far: self.zfar,
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

#[derive(ShaderType, Clone)]
pub struct Plane {
    pub origin: Vec3,
    pub normal: Vec3,
}

impl Plane {
    pub fn point_in_front(&self, point: Vec3) -> bool {
        Vec3::dot(point - self.origin, self.normal) >= 0.0
    }
    pub fn sphere_in_front(&self, sphere: &BoundingSphere) -> bool {
        Vec3::dot(sphere.center - self.origin, self.normal) + sphere.radius >= 0.0
    }
}

#[derive(ShaderType, Clone)]
pub struct CameraFrustum {
    pub near: Plane,
    pub far: Plane,
    pub left: Plane,
    pub right: Plane,
    pub bottom: Plane,
    pub top: Plane,
}

impl CameraFrustum {
    pub fn planes(&self) -> [Plane; 6] {
        [
            self.left.clone(),
            self.right.clone(),
            self.bottom.clone(),
            self.top.clone(),
            self.near.clone(),
            self.far.clone(),
        ]
    }
    pub fn center(&self) -> Vec3 {
        (self.near.origin + self.far.origin) / 2.0
    }
}

pub struct BoundingSphere {
    pub center: Vec3,
    pub radius: f32,
}

impl BoundingSphere {
    pub fn new(bounds: &BoundingBox, transform: &Transform3D) -> Self {
        let local_center = (bounds.min + bounds.max) * 0.5;
        let max_distance_from_center = f32::max(
            (bounds.min - local_center).length(),
            (bounds.max - local_center).length(),
        );

        let center = (transform.matrix() * local_center.extend(1.0)).xyz();
        let radius = max_distance_from_center * transform.scale.max_element();
        Self { center, radius }
    }
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

    pub fn sphere_inside(&self, bounds: &BoundingBox, transform: &Transform3D) -> bool {
        let bounding_sphere = BoundingSphere::new(bounds, transform);
        self.near.sphere_in_front(&bounding_sphere)
            && self.far.sphere_in_front(&bounding_sphere)
            && self.left.sphere_in_front(&bounding_sphere)
            && self.right.sphere_in_front(&bounding_sphere)
            && self.bottom.sphere_in_front(&bounding_sphere)
            && self.top.sphere_in_front(&bounding_sphere)
    }
}

impl Camera {
    // TODO: cache in camera?
    pub fn calculate_frustum(&self, ctx: &Context) -> CameraFrustum {
        let cam_forward = self.forward();
        let cam_right = self.right();
        let cam_up = self.up();

        match self.projection {
            CameraProjection::Orthographic { height } => {
                let aspect_ratio = render::aspect_ratio(ctx);
                let half_height = height / 2.0;
                let half_width = half_height * aspect_ratio;

                let near = Plane {
                    origin: self.pos + self.znear * cam_forward,
                    normal: cam_forward.normalize(),
                };
                let far = Plane {
                    origin: self.pos + self.zfar * cam_forward,
                    normal: -cam_forward.normalize(),
                };
                let left = Plane {
                    origin: self.pos - half_width * cam_right,
                    normal: cam_right.normalize(),
                };
                let right = Plane {
                    origin: self.pos + half_width * cam_right,
                    normal: -cam_right.normalize(),
                };
                let bottom = Plane {
                    origin: self.pos - half_height * cam_up,
                    normal: cam_up.normalize(),
                };
                let top = Plane {
                    origin: self.pos + half_height * cam_up,
                    normal: -cam_up.normalize(),
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
            CameraProjection::Perspective { fov } => {
                let aspect_ratio = render::aspect_ratio(ctx);
                let half_far_height = self.zfar * f32::tan(fov / 2.0);
                let half_far_width = half_far_height * aspect_ratio;

                let far_left = cam_forward * self.zfar - cam_right * half_far_width;
                let far_right = cam_forward * self.zfar + cam_right * half_far_width;
                let far_bottom = cam_forward * self.zfar - cam_up * half_far_height;
                let far_top = cam_forward * self.zfar + cam_up * half_far_height;

                let near = Plane {
                    origin: self.pos + self.znear * cam_forward,
                    normal: cam_forward.normalize(),
                };
                let far = Plane {
                    origin: self.pos + self.zfar * cam_forward,
                    normal: -cam_forward.normalize(),
                };
                let left = Plane {
                    origin: self.pos,
                    normal: Vec3::cross(far_left, cam_up).normalize(),
                };
                let right = Plane {
                    origin: self.pos,
                    normal: Vec3::cross(cam_up, far_right).normalize(),
                };
                let bottom = Plane {
                    origin: self.pos,
                    normal: Vec3::cross(cam_right, far_bottom).normalize(),
                };
                let top = Plane {
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
