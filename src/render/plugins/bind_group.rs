use std::num::NonZeroU32;

use crate::{render, Context};

// Bindgroup entry

pub struct BindGroupEntry<'a> {
    resource: wgpu::BindingResource<'a>,

    visibility: wgpu::ShaderStages,
    ty: wgpu::BindingType,
    count: Option<NonZeroU32>,
}

impl<'a> BindGroupEntry<'a> {
    pub fn new(resource: wgpu::BindingResource<'a>) -> Self {
        Self {
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,

            resource,
        }
    }

    pub fn visibility(mut self, value: wgpu::ShaderStages) -> Self {
        self.visibility = value;
        self
    }
    pub fn ty(mut self, value: wgpu::BindingType) -> Self {
        self.ty = value;
        self
    }
    pub fn count(mut self, value: NonZeroU32) -> Self {
        self.count = Some(value);
        self
    }

    // Shortcuts, assumes no dynamic offset and no min binding size

    pub fn uniform(mut self) -> Self {
        self.ty = wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        };
        self
    }
    pub fn storage(mut self, read_only: bool) -> Self {
        self.ty = wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only },
            has_dynamic_offset: false,
            min_binding_size: None,
        };
        self
    }
}

// Bindgroup

pub struct BindGroupBuilder<'a> {
    entries: Vec<BindGroupEntry<'a>>,

    label: Option<String>,
}

impl<'a> BindGroupBuilder<'a> {
    pub fn new(entries: Vec<BindGroupEntry<'a>>) -> Self {
        Self {
            label: None,
            entries,
        }
    }

    pub fn build(self, ctx: &Context) -> BindGroup {
        let device = render::device(ctx);
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: self.label.as_deref(),
            entries: &self
                .entries
                .iter()
                .enumerate()
                .map(|(i, e)| wgpu::BindGroupLayoutEntry {
                    binding: i as u32,
                    visibility: e.visibility,
                    ty: e.ty,
                    count: e.count,
                })
                .collect::<Vec<_>>(),
        });
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: self.label.as_deref(),
            layout: &bind_group_layout,
            entries: &self
                .entries
                .into_iter()
                .enumerate()
                .map(|(i, e)| wgpu::BindGroupEntry {
                    binding: i as u32,
                    resource: e.resource,
                })
                .collect::<Vec<_>>(),
        });
        BindGroup {
            bind_group_layout,
            bind_group,
        }
    }
    pub fn label(mut self, value: &str) -> Self {
        self.label = Some(value.to_string());
        self
    }
}

pub struct BindGroup {
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
}

impl BindGroup {
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }
}
