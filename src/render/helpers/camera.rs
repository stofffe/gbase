use crate::{input, time, Context};
use encase::ShaderType;
use glam::{vec3, Mat4, Vec3};
use std::f32::consts::PI;

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

        let aspect = ctx.render.aspect_ratio();
        let view = Mat4::look_to_rh(self.pos, self.forward(), self.up());
        let proj = match self.projection {
            CameraProjection::Perspective { fov } => {
                Mat4::perspective_rh(fov, aspect, self.znear, self.zfar)
            }
            CameraProjection::Orthographic { height } => Mat4::orthographic_rh(
                aspect * -height / 2.0,
                aspect * height / 2.0,
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

    /// Simple controls for free flying camera
    pub fn flying_controls(&mut self, ctx: &Context) {
        let dt = time::delta_time(ctx);

        // Camera rotation
        let (mouse_dx, mouse_dy) = input::mouse_delta(ctx);
        self.yaw -= 1.0 * dt * mouse_dx;
        self.pitch -= 1.0 * dt * mouse_dy;

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
        const CAMERA_MOVE_SPEED: f32 = 15.0;
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

//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//
//

#[derive(Debug)]
pub struct OrthographicCamera {
    pub pos: Vec3,
    pub yaw: f32,
    pub pitch: f32,

    pub znear: f32,
    pub zfar: f32,

    pub left: f32,
    pub right: f32,
    pub bottom: f32,
    pub top: f32,
}

impl OrthographicCamera {
    pub fn new() -> Self {
        let width = 10.0;
        let height = 10.0;

        Self {
            pos: Vec3::ZERO,
            yaw: 0.0,
            pitch: 0.0,

            left: -width / 2.0,
            right: width / 2.0,
            bottom: -height / 2.0,
            top: height / 2.0,
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

    pub fn uniform(&self, _ctx: &Context) -> CameraUniform {
        let pos = self.pos;
        let facing = self.forward();

        let view = Mat4::look_to_rh(self.pos, self.forward(), self.up());
        let proj = Mat4::orthographic_rh(
            self.left,
            self.right,
            self.bottom,
            self.top,
            self.znear,
            self.zfar,
        );
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
}

/// Right handed perspective camera
///
/// yaw = pitch = 0
/// => forw  (0, 0, -1)
/// => right (1, 0, 0)
/// => up    (0, 1, 0)
#[derive(Debug)]
pub struct PerspectiveCamera {
    pub pos: Vec3,
    pub yaw: f32,
    pub pitch: f32,

    pub znear: f32,
    pub zfar: f32,

    pub fov: f32,
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
    pub fn uniform(&mut self, ctx: &Context) -> CameraUniform {
        const MIN_PITCH: f32 = -PI / 2.0 + 0.1;
        const MAX_PITCH: f32 = PI / 2.0 - 0.1;
        const MIN_FOV: f32 = 0.1;
        const MAX_FOV: f32 = PI - 0.1;
        self.pitch = self.pitch.clamp(MIN_PITCH, MAX_PITCH);
        self.fov = self.fov.clamp(MIN_FOV, MAX_FOV);

        let pos = self.pos;
        let facing = self.forward();

        // right handed coords
        let view = Mat4::look_to_rh(self.pos, self.forward(), self.up());
        let proj = Mat4::perspective_rh(self.fov, ctx.render.aspect_ratio(), self.znear, self.zfar);
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

    /// Simple controls for free flying camera
    pub fn flying_controls(&mut self, ctx: &Context) {
        let dt = time::delta_time(ctx);

        // Camera rotation
        let (mouse_dx, mouse_dy) = input::mouse_delta(ctx);
        self.yaw -= 1.0 * dt * mouse_dx;
        self.pitch -= 1.0 * dt * mouse_dy;

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
        const CAMERA_MOVE_SPEED: f32 = 25.0;
        if camera_movement_dir != Vec3::ZERO {
            if input::key_pressed(ctx, winit::keyboard::KeyCode::KeyM) {
                self.pos += camera_movement_dir.normalize() * dt * CAMERA_MOVE_SPEED / 10.0;
            } else {
                self.pos += camera_movement_dir.normalize() * dt * CAMERA_MOVE_SPEED;
            }
        }
    }
}
