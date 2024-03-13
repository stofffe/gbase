use crate::{render, Context};

//
// Bind Group Layout
//

pub struct BindGroupLayoutBuilder<'a> {
    label: Option<&'a str>,
    entries: &'a [BindGroupLayoutEntry],
}

impl<'a> BindGroupLayoutBuilder<'a> {
    pub fn new() -> Self {
        Self {
            label: None,
            entries: &[],
        }
    }
    pub fn build(self, ctx: &Context) -> wgpu::BindGroupLayout {
        let device = render::device(ctx);
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: self.label,
            entries: &self
                .entries
                .iter()
                .enumerate()
                .map(|(i, e)| wgpu::BindGroupLayoutEntry {
                    binding: i as u32,
                    visibility: e.visibility,
                    ty: e.ty,
                    count: None,
                })
                .collect::<Vec<_>>(),
        });

        layout
    }
}

impl<'a> BindGroupLayoutBuilder<'a> {
    pub fn label(mut self, value: &'a str) -> Self {
        self.label = Some(value);
        self
    }
    pub fn entries(mut self, value: &'a [BindGroupLayoutEntry]) -> Self {
        self.entries = value;
        self
    }
}

pub struct BindGroupLayoutEntry {
    visibility: wgpu::ShaderStages,
    ty: wgpu::BindingType,
}

impl BindGroupLayoutEntry {
    pub fn new() -> Self {
        Self {
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
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

//
// Bind Group
//

pub struct BindGroupBuilder<'a> {
    label: Option<&'a str>,
    layout: &'a wgpu::BindGroupLayout,
    entries: &'a [BindGroupEntry<'a>],
}

impl<'a> BindGroupBuilder<'a> {
    pub fn new(layout: &'a wgpu::BindGroupLayout) -> Self {
        Self {
            layout,
            label: None,
            entries: &[],
        }
    }
    pub fn build(self, ctx: &Context) -> wgpu::BindGroup {
        let device = render::device(ctx);
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: self.label,
            layout: self.layout,
            entries: &self
                .entries
                .iter()
                .enumerate()
                .map(|(i, e)| wgpu::BindGroupEntry {
                    binding: i as u32,
                    resource: e.resource.clone(),
                })
                .collect::<Vec<_>>(),
        });

        group
    }
}

impl<'a> BindGroupBuilder<'a> {
    pub fn label(mut self, value: &'a str) -> Self {
        self.label = Some(value);
        self
    }
    pub fn entries(mut self, value: &'a [BindGroupEntry<'_>]) -> Self {
        self.entries = value;
        self
    }
}

pub struct BindGroupEntry<'a> {
    resource: wgpu::BindingResource<'a>,
}

impl<'a> BindGroupEntry<'a> {
    pub fn new(resource: wgpu::BindingResource<'a>) -> Self {
        Self { resource }
    }
}

//
// Combined
//

pub struct BindGroupCombinedBuilder<'a> {
    label: Option<&'a str>,
    entries: &'a [BindGroupCombinedEntry<'a>],
}

impl<'a> BindGroupCombinedBuilder<'a> {
    pub fn new() -> Self {
        Self {
            label: None,
            entries: &[],
        }
    }
    pub fn build(self, ctx: &Context) -> (wgpu::BindGroupLayout, wgpu::BindGroup) {
        let device = render::device(ctx);
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: self.label,
            entries: &self
                .entries
                .iter()
                .enumerate()
                .map(|(i, e)| wgpu::BindGroupLayoutEntry {
                    binding: i as u32,
                    visibility: e.visibility,
                    ty: e.ty,
                    count: None,
                })
                .collect::<Vec<_>>(),
        });

        let bindgroup = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: self.label,
            layout: &layout,
            entries: &self
                .entries
                .iter()
                .enumerate()
                .map(|(i, e)| wgpu::BindGroupEntry {
                    binding: i as u32,
                    resource: e.resource.clone(),
                })
                .collect::<Vec<_>>(),
        });

        (layout, bindgroup)
    }
}

impl<'a> BindGroupCombinedBuilder<'a> {
    pub fn label(mut self, value: &'a str) -> Self {
        self.label = Some(value);
        self
    }
    pub fn entries(mut self, value: &'a [BindGroupCombinedEntry<'a>]) -> Self {
        self.entries = value;
        self
    }
}

pub struct BindGroupCombinedEntry<'a> {
    resource: wgpu::BindingResource<'a>,
    visibility: wgpu::ShaderStages,
    ty: wgpu::BindingType,
}

impl<'a> BindGroupCombinedEntry<'a> {
    pub fn new(resource: wgpu::BindingResource<'a>) -> Self {
        Self {
            resource,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
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
