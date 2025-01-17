use encase::ShaderType;
use glam::{Mat4, Quat, Vec3};

//
// Transform
//

#[derive(Debug, Clone)]
pub struct Transform {
    pub pos: Vec3,
    pub rot: Quat,
    pub scale: Vec3,
}

impl Transform {
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

impl Default for Transform {
    fn default() -> Self {
        Self {
            pos: Vec3::ZERO,
            rot: Quat::IDENTITY,
            scale: Vec3::ONE,
        }
    }
}

#[derive(ShaderType)]
pub struct TransformUniform {
    matrix: Mat4,
}

//
// Transform GPU
//

//
// pub struct TransformGPU {
//     bind_group_layout: ArcBindGroupLayout,
//     bind_group: new::ArcBindGroup,
//     buffer: ArcBuffer,
// }
//
// impl TransformGPU {
//     pub fn new(ctx: &Context) -> Self {
//         let buffer = super::BufferBuilder::new(TransformUniform::min_size())
//             .usage(wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST)
//             .build(ctx);
//
//         let bind_group_layout = super::BindGroupLayoutBuilder::new()
//             .entries(&[super::BindGroupLayoutEntry::new().uniform()])
//             .build(ctx);
//         let bind_group = super::BindGroupBuilder::new(bind_group_layout.clone())
//             .entries(&[BindGroupEntry::new(buffer.as_entire_binding())])
//             .build(ctx);
//
//         Self {
//             buffer,
//             bind_group,
//             bind_group_layout,
//         }
//     }
//
//     pub fn update_buffer(&mut self, ctx: &Context, transform: &Transform) {
//         self.buffer.write_uniform(ctx, &transform.uniform());
//     }
//
//     pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
//         &self.bind_group_layout
//     }
//     pub fn bind_group(&self) -> &wgpu::BindGroup {
//         &self.bind_group
//     }
//     pub fn buffer(&self) -> &wgpu::Buffer {
//         &self.buffer
//     }
// }

// Re-export transform function
// impl TransformGPU {
//     pub fn pos(mut self, pos: Vec3) -> Self {
//         self.transform.pos = pos;
//         self
//     }
//     pub fn rotation(mut self, rotation: Quat) -> Self {
//         self.transform.rot = rotation;
//         self
//     }
//     pub fn scale(mut self, scale: Vec3) -> Self {
//         self.transform.scale = scale;
//         self
//     }
// }

// let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
//     label: Some("transform bg layout"),
//     entries: &[wgpu::BindGroupLayoutEntry {
//         binding: 0,
//         visibility: wgpu::ShaderStages::VERTEX,
//         ty: wgpu::BindingType::Buffer {
//             ty: wgpu::BufferBindingType::Uniform,
//             has_dynamic_offset: false,
//             min_binding_size: None,
//         },
//         count: None,
//     }],
// });
//
// let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
//     label: Some("transform bg"),
//     layout: &bind_group_layout,
//     entries: &[wgpu::BindGroupEntry {
//         binding: 0,
//         resource: buffer.as_entire_binding(),
//     }],
// });
