use std::num::NonZero;

use crate::{
    render::{self, next_id},
    Context,
};
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
    pub fn build_uncached(&self, ctx: &mut Context) -> ArcBindGroupLayout {
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

        ArcBindGroupLayout::new(ctx, layout)
    }

    pub fn build(self, ctx: &mut Context) -> ArcBindGroupLayout {
        if let Some(bindgroup_layout) = ctx.render.cache.bindgroup_layouts.get(&self) {
            // log::info!("Fetch cached bindgroup layout");
            return bindgroup_layout.clone();
        }

        tracing::info!("Create cached bindgroup layout");
        let bindgrouo_layout = self.build_uncached(ctx);
        ctx.render
            .cache
            .bindgroup_layouts
            .insert(self, bindgrouo_layout.clone());
        bindgrouo_layout
    }
}

impl BindGroupLayoutBuilder {
    pub fn label(mut self, value: impl Into<String>) -> Self {
        self.label = Some(value.into());
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
    pub fn build_uncached(&self, ctx: &mut Context) -> ArcBindGroup {
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

        ArcBindGroup::new(ctx, bindgroup)
    }

    pub fn build(self, ctx: &mut Context) -> ArcBindGroup {
        if let Some(bindgroup) = ctx.render.cache.bindgroups.get(&self) {
            // tracing::info!("Fetch cached bindgroup");
            return bindgroup.clone();
        }

        tracing::info!("Create cached bindgroup");
        let bindgroup = self.build_uncached(ctx);
        ctx.render.cache.bindgroups.insert(self, bindgroup.clone());
        bindgroup
    }
}

impl BindGroupBuilder {
    pub fn label(mut self, value: impl Into<String>) -> Self {
        self.label = Some(value.into());
        self
    }
    pub fn entries(mut self, value: Vec<BindGroupEntry>) -> Self {
        self.entries = value;
        self
    }
}

#[derive(Clone, Eq, PartialEq, Hash)]
pub enum BindGroupEntry {
    Buffer(ArcBuffer),
    BufferSlice {
        buffer: ArcBuffer,
        offset: u64,
        size: u64,
    },
    Texture(ArcTextureView),
    Sampler(ArcSampler),
}

impl BindGroupEntry {
    pub fn resource(&self) -> wgpu::BindingResource<'_> {
        match self {
            BindGroupEntry::Buffer(buffer) => buffer.as_entire_binding(),
            BindGroupEntry::Texture(texture) => wgpu::BindingResource::TextureView(texture),
            BindGroupEntry::Sampler(sampler) => wgpu::BindingResource::Sampler(sampler),
            BindGroupEntry::BufferSlice {
                buffer,
                offset,
                size,
            } => wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                buffer,
                offset: *offset,
                size: NonZero::new(*size),
            }),
        }
    }
}

pub trait BindGroupBindable<T> {
    fn bindgroup_entry(&self) -> BindGroupEntry;
}
