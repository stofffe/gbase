use crate::{render, Context};
use glam::UVec2;

/// Type of source for TextureBuilder
pub enum TextureSource {
    /// Bytes which contains header supported by the image crate
    /// png, jpeg, bmp, ...
    /// Always converted to rgba8
    FormattedBytes(Vec<u8>),
    /// Raw bytes without header
    RawBytes {
        width: u32,
        height: u32,
        format: TextureFormat,
        bytes: Vec<u8>,
    },
}

/// Supported texture formats for TextureBuilder
pub enum TextureFormat {
    /// r, \[0, 255] => \[0.0, 1.0]
    R8Unorm,
    /// r g b a, \[0, 255] => \[0.0, 1.0]
    Rgba8Unorm,
}

impl TextureFormat {
    fn to_wgpu(&self) -> wgpu::TextureFormat {
        match self {
            TextureFormat::R8Unorm => wgpu::TextureFormat::R8Unorm,
            TextureFormat::Rgba8Unorm => wgpu::TextureFormat::Rgba8Unorm,
        }
    }

    /// Returns the amount of bytes per element
    fn size(&self) -> u32 {
        match self {
            TextureFormat::R8Unorm => 1,
            TextureFormat::Rgba8Unorm => 4,
        }
    }
}

pub struct TextureBuilder {
    source: TextureSource, // TODO add enum for multiple variants such as single pixel

    label: Option<String>,
    usage: wgpu::TextureUsages,
    visibility: wgpu::ShaderStages,
    address_mode_u: wgpu::AddressMode,
    address_mode_v: wgpu::AddressMode,
    address_mode_w: wgpu::AddressMode,
    mag_filter: wgpu::FilterMode,
    min_filter: wgpu::FilterMode,
    mipmap_filter: wgpu::FilterMode,
}

impl TextureBuilder {
    #[allow(clippy::new_without_default)]
    pub fn new(source: TextureSource) -> Self {
        Self {
            label: None,
            source,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            visibility: wgpu::ShaderStages::FRAGMENT,
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
        }
    }

    pub fn build(self, ctx: &Context) -> Texture {
        let device = render::device(ctx);
        let queue = render::queue(ctx);

        #[rustfmt::skip]
        let (width, height, bytes, format) = match self.source {
            TextureSource::FormattedBytes(bytes) => {
                let img = image::load_from_memory(&bytes).unwrap().to_rgba8();
                (img.width(), img.height(), img.to_vec(), TextureFormat::Rgba8Unorm)
            }
            TextureSource::RawBytes { width, height, bytes, format, } => (width, height, bytes, format),
        };

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: self.label.as_deref(),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: format.to_wgpu(),
            usage: self.usage,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &bytes,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(format.size() * width),
                rows_per_image: Some(height),
            },
            texture.size(),
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: self.address_mode_u,
            address_mode_v: self.address_mode_v,
            address_mode_w: self.address_mode_w,
            mag_filter: self.mag_filter,
            min_filter: self.min_filter,
            mipmap_filter: self.mipmap_filter,
            ..Default::default()
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: self.label.as_deref(),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: self.visibility,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: self.visibility,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: self.label.as_deref(),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });
        Texture {
            texture,
            view,
            sampler,
            bind_group_layout,
            bind_group,
        }
    }

    pub fn label(mut self, value: impl Into<String>) -> Self {
        self.label = Some(value.into());
        self
    }
    pub fn usage(mut self, value: wgpu::TextureUsages) -> Self {
        self.usage = value;
        self
    }
    pub fn visibility(mut self, value: wgpu::ShaderStages) -> Self {
        self.visibility = value;
        self
    }
    pub fn address_mode(
        mut self,
        u: wgpu::AddressMode,
        v: wgpu::AddressMode,
        w: wgpu::AddressMode,
    ) -> Self {
        self.address_mode_u = u;
        self.address_mode_v = v;
        self.address_mode_w = w;
        self
    }
    pub fn filter_mode(
        mut self,
        mag: wgpu::FilterMode,
        min: wgpu::FilterMode,
        mipmap: wgpu::FilterMode,
    ) -> Self {
        self.mag_filter = mag;
        self.min_filter = min;
        self.mipmap_filter = mipmap;
        self
    }
}

pub struct Texture {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    sampler: wgpu::Sampler,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
}

impl Texture {
    pub fn texture(&self) -> &wgpu::Texture {
        &self.texture
    }
    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }
    pub fn sampler(&self) -> &wgpu::Sampler {
        &self.sampler
    }
    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}

pub struct TextureAtlas {
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    sampler: wgpu::Sampler,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
}

impl TextureAtlas {
    pub fn write_texture(&mut self, ctx: &Context, origin: UVec2, dimensions: UVec2, bytes: &[u8]) {
        let queue = render::queue(ctx);

        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: origin.x,
                    y: origin.y,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            bytes,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(dimensions.x),
                rows_per_image: Some(dimensions.y),
            },
            wgpu::Extent3d {
                width: dimensions.x,
                height: dimensions.y,
                depth_or_array_layers: 1,
            },
        );
    }
    pub fn new(ctx: &Context, size: UVec2) -> Self {
        let device = render::device(ctx);

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: size.x,
                height: size.y,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: None,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        Self {
            texture,
            view,
            sampler,
            bind_group_layout,
            bind_group,
        }
    }

    pub fn texture(&self) -> &wgpu::Texture {
        &self.texture
    }
    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }
    pub fn sampler(&self) -> &wgpu::Sampler {
        &self.sampler
    }
    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}
