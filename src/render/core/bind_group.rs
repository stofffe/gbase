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
    pub const fn new() -> Self {
        Self {
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
        }
    }

    /// Set shader visibility
    pub const fn visibility(mut self, value: wgpu::ShaderStages) -> Self {
        self.visibility = value;
        self
    }

    /// Set binding type
    pub const fn ty(mut self, value: wgpu::BindingType) -> Self {
        self.ty = value;
        self
    }

    /// Set binding type to ```Uniform```
    pub const fn uniform(self) -> Self {
        self.ty(wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        })
    }

    /// Set binding type to ```Storage```
    pub const fn storage(self, read_only: bool) -> Self {
        self.ty(wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only },
            has_dynamic_offset: false,
            min_binding_size: None,
        })
    }
    /// Add ```Vertex``` to shader visibility
    pub const fn vertex(self) -> Self {
        let v = self.visibility;
        self.visibility(v.union(wgpu::ShaderStages::VERTEX))
    }
    /// Add ```Fragment``` to shader visibility
    pub const fn fragment(self) -> Self {
        let v = self.visibility;
        self.visibility(v.union(wgpu::ShaderStages::FRAGMENT))
    }
    /// Add ```Compute``` to shader visibility
    pub const fn compute(self) -> Self {
        let v = self.visibility;
        self.visibility(v.union(wgpu::ShaderStages::COMPUTE))
    }
    /// Set Binding type to float texture
    pub const fn texture_float(self, filterable: bool) -> Self {
        self.ty(wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        })
    }
    /// Set Binding type to depth texture
    pub const fn texture_depth(self) -> Self {
        self.ty(wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Depth,
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        })
    }
    /// Set Binding type to filtering sampler
    pub const fn sampler_filtering(self) -> Self {
        self.ty(wgpu::BindingType::Sampler(
            wgpu::SamplerBindingType::Filtering,
        ))
    }
    /// Set Binding type to non filtering sampler
    pub const fn sampler_nonfiltering(self) -> Self {
        self.ty(wgpu::BindingType::Sampler(
            wgpu::SamplerBindingType::NonFiltering,
        ))
    }
}

//
// Bind Group
//

pub struct BindGroupBuilder<'a> {
    label: Option<&'a str>,
    entries: &'a [BindGroupEntry<'a>],
}

impl<'a> BindGroupBuilder<'a> {
    pub fn new() -> Self {
        Self {
            label: None,
            entries: &[],
        }
    }
    pub fn build(self, ctx: &Context, layout: &'a wgpu::BindGroupLayout) -> wgpu::BindGroup {
        let device = render::device(ctx);
        let group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: self.label,
            layout,
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
    pub const fn new(resource: wgpu::BindingResource<'a>) -> Self {
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
                .map(|(i, entry)| wgpu::BindGroupLayoutEntry {
                    binding: i as u32,
                    visibility: entry.bindgroup_layout.visibility,
                    ty: entry.bindgroup_layout.ty,
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
                .map(|(i, entry)| wgpu::BindGroupEntry {
                    binding: i as u32,
                    resource: entry.bindgroup.resource.clone(),
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
    bindgroup: BindGroupEntry<'a>,
    bindgroup_layout: BindGroupLayoutEntry,
}

impl<'a> BindGroupCombinedEntry<'a> {
    pub const fn new(resource: wgpu::BindingResource<'a>) -> Self {
        Self {
            bindgroup: BindGroupEntry::new(resource),
            bindgroup_layout: BindGroupLayoutEntry::new(),
        }
    }

    /// Set shader visibility
    pub const fn visibility(mut self, value: wgpu::ShaderStages) -> Self {
        self.bindgroup_layout = self.bindgroup_layout.visibility(value);
        self
    }

    /// Set binding type
    pub const fn ty(mut self, value: wgpu::BindingType) -> Self {
        self.bindgroup_layout = self.bindgroup_layout.ty(value);
        self
    }

    /// Set binding type to ```Uniform```
    pub const fn uniform(mut self) -> Self {
        self.bindgroup_layout = self.bindgroup_layout.uniform();
        self
    }

    /// Set binding type to ```Storage```
    pub const fn storage(mut self, read_only: bool) -> Self {
        self.bindgroup_layout = self.bindgroup_layout.storage(read_only);
        self
    }
    /// Add ```Vertex``` to shader visibility
    pub const fn vertex(mut self) -> Self {
        self.bindgroup_layout = self.bindgroup_layout.vertex();
        self
    }
    /// Add ```Fragment``` to shader visibility
    pub const fn fragment(mut self) -> Self {
        self.bindgroup_layout = self.bindgroup_layout.fragment();
        self
    }
    /// Add ```Compute``` to shader visibility
    pub const fn compute(mut self) -> Self {
        self.bindgroup_layout = self.bindgroup_layout.compute();
        self
    }
    /// Set Binding type to float texture
    pub const fn texture_float(mut self, filterable: bool) -> Self {
        self.bindgroup_layout = self.bindgroup_layout.texture_float(filterable);
        self
    }
    /// Set Binding type to depth texture
    pub const fn texture_depth(mut self) -> Self {
        self.bindgroup_layout = self.bindgroup_layout.texture_depth();
        self
    }
    /// Set Binding type to filtering sampler
    pub const fn sampler_filtering(mut self) -> Self {
        self.bindgroup_layout = self.bindgroup_layout.sampler_filtering();
        self
    }
    /// Set Binding type to non filtering sampler
    pub const fn sampler_nonfiltering(mut self) -> Self {
        self.bindgroup_layout = self.bindgroup_layout.sampler_nonfiltering();
        self
    }
}
