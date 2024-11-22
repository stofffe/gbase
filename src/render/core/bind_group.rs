use crate::{render, Context};
use render::{ArcBindGroup, ArcBindGroupLayout, ArcBuffer, ArcSampler, ArcTextureView};

//
// Bind Group Layout
//

#[derive(Clone, Eq, PartialEq, Hash)]
pub struct BindGroupLayoutBuilder {
    label: Option<String>,
    entries: Vec<BindGroupLayoutEntry>,
}

impl BindGroupLayoutBuilder {
    pub fn new() -> Self {
        Self {
            label: None,
            entries: Vec::new(),
        }
    }
    pub fn build_uncached(&self, ctx: &Context) -> ArcBindGroupLayout {
        let device = render::device(ctx);
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: self.label.as_deref(),
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

        ArcBindGroupLayout::new(layout)
    }

    pub fn build(&self, ctx: &mut Context) -> ArcBindGroupLayout {
        if let Some(bindgroup_layout) = ctx.render.cache.bindgroup_layouts.get(self) {
            log::info!("Fetch cached bindgroup layout");
            return bindgroup_layout.clone();
        }

        log::info!("Create cached bindgroup layout");
        let bindgrouo_layout = self.build_uncached(ctx);
        ctx.render
            .cache
            .bindgroup_layouts
            .insert(self.clone(), bindgrouo_layout.clone());
        bindgrouo_layout
    }
}

impl BindGroupLayoutBuilder {
    pub fn label(mut self, value: String) -> Self {
        self.label = Some(value);
        self
    }
    pub fn entries(mut self, value: Vec<BindGroupLayoutEntry>) -> Self {
        self.entries = value;
        self
    }
}

#[derive(Clone, Eq, PartialEq, Hash)]
pub struct BindGroupLayoutEntry {
    visibility: wgpu::ShaderStages,
    ty: wgpu::BindingType,
}

impl BindGroupLayoutEntry {
    pub const fn new() -> Self {
        Self {
            visibility: wgpu::ShaderStages::empty(),
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
    pub const fn storage_readonly(self) -> Self {
        self.ty(wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only: true },
            has_dynamic_offset: false,
            min_binding_size: None,
        })
    }

    /// Set binding type to ```Storage```
    pub const fn storage(self) -> Self {
        self.ty(wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only: false },
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
    /// Set Binding type to float texture filtering
    pub const fn texture_float_nonfilterable(self) -> Self {
        self.ty(wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: false },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        })
    }
    /// Set Binding type to float texture nonfiltering
    pub const fn texture_float_filterable(self) -> Self {
        self.ty(wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
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
    /// Set Binding type to storage texture
    pub const fn storage_texture_2d_write(self, format: wgpu::TextureFormat) -> Self {
        self.ty(wgpu::BindingType::StorageTexture {
            access: wgpu::StorageTextureAccess::WriteOnly,
            format,
            view_dimension: wgpu::TextureViewDimension::D2,
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
    /// Set Binding type to filtering sampler
    pub const fn sampler_comparison(self) -> Self {
        self.ty(wgpu::BindingType::Sampler(
            wgpu::SamplerBindingType::Comparison,
        ))
    }
}

//
// Bind Group
//

#[derive(Clone, Eq, PartialEq, Hash)]
pub struct BindGroupBuilder {
    layout: ArcBindGroupLayout,
    label: Option<String>,
    entries: Vec<BindGroupEntry>,
}

impl BindGroupBuilder {
    pub fn new(layout: ArcBindGroupLayout) -> Self {
        Self {
            layout,
            label: None,
            entries: Vec::new(),
        }
    }
    pub fn build_uncached(&self, ctx: &Context) -> ArcBindGroup {
        let device = render::device(ctx);

        let bindgroup = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: self.label.as_deref(),
            layout: &self.layout,
            entries: &self
                .entries
                .iter()
                .enumerate()
                .map(|(i, e)| wgpu::BindGroupEntry {
                    binding: i as u32,
                    resource: e.resource(),
                })
                .collect::<Vec<_>>(),
        });

        ArcBindGroup::new(bindgroup)
    }

    pub fn build(&self, ctx: &mut Context) -> ArcBindGroup {
        if let Some(bindgroup) = ctx.render.cache.bindgroups.get(self) {
            log::info!("Fetch cached bindgroup");
            return bindgroup.clone();
        }

        log::info!("Create cached bindgroup");
        let bindgroup = self.build_uncached(ctx);
        ctx.render
            .cache
            .bindgroups
            .insert(self.clone(), bindgroup.clone());
        bindgroup
    }
}

impl BindGroupBuilder {
    pub fn label(mut self, value: String) -> Self {
        self.label = Some(value);
        self
    }
    pub fn entries(mut self, value: Vec<BindGroupEntry>) -> Self {
        self.entries = value;
        self
    }
}

#[derive(Clone, Eq, PartialEq, Hash)]
pub enum BindGroupEntry {
    Buffer(ArcBuffer), // TODO add offset and size
    Texture(ArcTextureView),
    Sampler(ArcSampler),
}

impl BindGroupEntry {
    pub fn resource(&self) -> wgpu::BindingResource<'_> {
        match self {
            BindGroupEntry::Buffer(buffer) => buffer.as_entire_binding(),
            BindGroupEntry::Texture(texture) => wgpu::BindingResource::TextureView(texture),
            BindGroupEntry::Sampler(sampler) => wgpu::BindingResource::Sampler(sampler),
        }
    }
}

//
// Combined
//

// pub struct CombinedBingroupBuilder {
//     label: Option<String>,
//     entries: Vec<CombinedBindgroupEntry>,
// }
//
// pub struct CombinedBindgroupEntry {
//     bindgroup_layout: BindGroupLayoutEntry,
//     bindgroup: BindGroupEntry,
// }

pub struct BindGroupCombinedBuilder {
    label: Option<String>,
    entries: Vec<BindGroupCombinedEntry>,
}

impl BindGroupCombinedBuilder {
    pub fn new() -> Self {
        Self {
            label: None,
            entries: Vec::new(),
        }
    }
    pub fn build_uncached(self, ctx: &Context) -> (ArcBindGroupLayout, ArcBindGroup) {
        let device = render::device(ctx);
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: self.label.as_deref(),
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
            label: self.label.as_deref(),
            layout: &layout,
            entries: &self
                .entries
                .iter()
                .enumerate()
                .map(|(i, entry)| wgpu::BindGroupEntry {
                    binding: i as u32,
                    resource: entry.bindgroup.resource().clone(),
                })
                .collect::<Vec<_>>(),
        });

        (
            ArcBindGroupLayout::new(layout),
            ArcBindGroup::new(bindgroup),
        )
    }
    pub fn build(&self, ctx: &mut Context) -> (ArcBindGroupLayout, ArcBindGroup) {
        let mut bindgroup_layout = BindGroupLayoutBuilder::new();
        bindgroup_layout.entries = self
            .entries
            .iter()
            .map(|b| b.bindgroup_layout.clone())
            .collect::<Vec<_>>();
        bindgroup_layout.label = self.label.clone();

        let bindgroup_layout = match ctx.render.cache.bindgroup_layouts.get(&bindgroup_layout) {
            Some(bindgroup_layout) => bindgroup_layout.clone(),
            None => bindgroup_layout.build(ctx),
        };

        let mut bindgroup = BindGroupBuilder::new(bindgroup_layout.clone());
        bindgroup.entries = self
            .entries
            .iter()
            .map(|b| b.bindgroup.clone())
            .collect::<Vec<_>>();
        bindgroup.label = self.label.clone();

        let bindgroup = match ctx.render.cache.bindgroups.get(&bindgroup) {
            Some(bindgroup) => bindgroup.clone(),
            None => bindgroup.build(ctx),
        };

        (bindgroup_layout, bindgroup)
    }
}

impl BindGroupCombinedBuilder {
    pub fn label(mut self, value: String) -> Self {
        self.label = Some(value);
        self
    }
    pub fn entries(mut self, value: Vec<BindGroupCombinedEntry>) -> Self {
        self.entries = value;
        self
    }
}

pub struct BindGroupCombinedEntry {
    bindgroup: BindGroupEntry,
    bindgroup_layout: BindGroupLayoutEntry,
}

impl BindGroupCombinedEntry {
    pub const fn new(resource: BindGroupEntry) -> Self {
        Self {
            bindgroup: resource,
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
    pub const fn storage(mut self) -> Self {
        self.bindgroup_layout = self.bindgroup_layout.storage();
        self
    }
    /// Set binding type to ```Storage``` readonly
    pub const fn storage_readonly(mut self) -> Self {
        self.bindgroup_layout = self.bindgroup_layout.storage_readonly();
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
    /// Set Binding type to float texture filterable
    pub const fn texture_float_filterable(mut self) -> Self {
        self.bindgroup_layout = self.bindgroup_layout.texture_float_filterable();
        self
    }
    /// Set Binding type to float texture nonfilterable
    pub const fn texture_float_nonfilterable(mut self) -> Self {
        self.bindgroup_layout = self.bindgroup_layout.texture_float_nonfilterable();
        self
    }
    /// Set Binding type to depth texture
    pub const fn texture_depth(mut self) -> Self {
        self.bindgroup_layout = self.bindgroup_layout.texture_depth();
        self
    }
    /// Set Binding type to storage texture
    pub const fn storage_texture_2d_write(mut self, format: wgpu::TextureFormat) -> Self {
        self.bindgroup_layout = self.bindgroup_layout.storage_texture_2d_write(format);
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
