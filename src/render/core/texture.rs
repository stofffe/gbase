use crate::{render, Context};

//
// Sampler
//

pub struct SamplerBuilder<'a> {
    label: Option<&'a str>,
    address_mode_u: wgpu::AddressMode,
    address_mode_v: wgpu::AddressMode,
    address_mode_w: wgpu::AddressMode,
    mag_filter: wgpu::FilterMode,
    min_filter: wgpu::FilterMode,
}

impl<'a> SamplerBuilder<'a> {
    pub fn new() -> Self {
        Self {
            label: None,
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
        }
    }
    pub fn build(&self, ctx: &Context) -> Sampler {
        let device = render::device(ctx);
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: self.label,
            address_mode_u: self.address_mode_u,
            address_mode_v: self.address_mode_v,
            address_mode_w: self.address_mode_w,
            mag_filter: self.mag_filter,
            min_filter: self.min_filter,
            mipmap_filter: wgpu::FilterMode::Nearest,
            lod_min_clamp: 0.0,
            lod_max_clamp: 0.0,
            anisotropy_clamp: 1,
            compare: None,
            border_color: None,
        });
        Sampler { sampler }
    }
}

impl<'a> SamplerBuilder<'a> {
    pub fn label(mut self, value: &'a str) -> Self {
        self.label = Some(value);
        self
    }
    pub fn min_mag_filter(mut self, min: wgpu::FilterMode, mag: wgpu::FilterMode) -> Self {
        self.min_filter = min;
        self.mag_filter = mag;
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
}

pub struct Sampler {
    sampler: wgpu::Sampler,
}

impl Sampler {
    pub fn binding_filtering(&self) -> wgpu::BindingType {
        wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering)
    }
    pub fn binding_nonfiltering(&self) -> wgpu::BindingType {
        wgpu::BindingType::Sampler(wgpu::SamplerBindingType::NonFiltering)
    }

    pub fn resource(&self) -> wgpu::BindingResource<'_> {
        wgpu::BindingResource::Sampler(&self.sampler)
    }
}

//
// Texture
//

pub struct TextureBuilder<'a> {
    label: Option<&'a str>,
    usage: wgpu::TextureUsages,
    format: wgpu::TextureFormat,
}

impl<'a> TextureBuilder<'a> {
    pub fn new() -> Self {
        Self {
            label: None,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            format: wgpu::TextureFormat::Rgba8Unorm,
        }
    }

    pub fn build(self, ctx: &Context, width: u32, height: u32) -> Texture {
        let device = render::device(ctx);

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: self.label,
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.format,
            usage: self.usage,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        Texture { texture, view }
    }

    pub fn build_single_pixel(self, ctx: &Context, color: [u8; 4]) -> Texture {
        let texture = self.build(ctx, 1, 1);
        let queue = render::queue(ctx);
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &color,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4),
                rows_per_image: Some(1),
            },
            texture.texture.size(),
        );
        texture
    }

    pub fn build_init(self, ctx: &Context, bytes: &[u8]) -> Texture {
        let device = render::device(ctx);
        let queue = render::queue(ctx);

        let img = image::load_from_memory(bytes)
            .expect("could not load texture from bytes")
            .to_rgba8();

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: self.label,
            size: wgpu::Extent3d {
                width: img.width(),
                height: img.height(),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.format,
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
            &img,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * img.width()),
                rows_per_image: Some(img.height()),
            },
            texture.size(),
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        Texture { texture, view }
    }
}
impl<'a> TextureBuilder<'a> {
    pub fn label(mut self, value: &'a str) -> Self {
        self.label = Some(value);
        self
    }
    pub fn usage(mut self, value: wgpu::TextureUsages) -> Self {
        self.usage = value;
        self
    }
    pub fn format(mut self, value: wgpu::TextureFormat) -> Self {
        self.format = value;
        self
    }
}

pub struct Texture {
    pub(crate) texture: wgpu::Texture,
    pub(crate) view: wgpu::TextureView,
}

impl Texture {
    pub fn texture(&self) -> &wgpu::Texture {
        &self.texture
    }

    pub fn view(&self) -> &wgpu::TextureView {
        &self.view
    }

    pub fn binding_type(&self) -> wgpu::BindingType {
        wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true }, // TODO option?
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        }
    }
    pub fn binding_type_nonfilter(&self) -> wgpu::BindingType {
        wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: false }, // TODO option?
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        }
    }

    pub fn resource(&self) -> wgpu::BindingResource<'_> {
        wgpu::BindingResource::TextureView(self.view())
    }
}
