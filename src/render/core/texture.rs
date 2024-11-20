use crate::{
    render::{self, ArcSampler, ArcTexture, ArcTextureView},
    Context,
};

//
// Sampler
//

#[derive(Clone, Eq, PartialEq, Hash)]
pub struct SamplerBuilder {
    label: Option<String>,
    address_mode_u: wgpu::AddressMode,
    address_mode_v: wgpu::AddressMode,
    address_mode_w: wgpu::AddressMode,
    mag_filter: wgpu::FilterMode,
    min_filter: wgpu::FilterMode,
    lod_min_clamp_u32: u32, // 1 => 0.1
    lod_max_clamp_u32: u32,
    anisotropy_clamp: u16,
    compare: Option<wgpu::CompareFunction>,
    border_color: Option<wgpu::SamplerBorderColor>,
}

impl SamplerBuilder {
    pub fn new() -> Self {
        Self {
            label: None,
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            lod_min_clamp_u32: 0,
            lod_max_clamp_u32: 0,
            anisotropy_clamp: 1,
            compare: None,
            border_color: None,
        }
    }
    pub fn build_uncached(&self, ctx: &Context) -> ArcSampler {
        let device = render::device(ctx);

        let lod_min_clamp_f32 = self.lod_min_clamp_u32 as f32 / 10.0;
        let lod_max_clamp_f32 = self.lod_max_clamp_u32 as f32 / 10.0;

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: self.label.as_deref(),
            address_mode_u: self.address_mode_u,
            address_mode_v: self.address_mode_v,
            address_mode_w: self.address_mode_w,
            mag_filter: self.mag_filter,
            min_filter: self.min_filter,
            mipmap_filter: wgpu::FilterMode::Nearest,
            lod_min_clamp: lod_min_clamp_f32,
            lod_max_clamp: lod_max_clamp_f32,
            anisotropy_clamp: self.anisotropy_clamp,
            compare: self.compare,
            border_color: self.border_color,
        });

        ArcSampler::new(sampler)
    }
    pub fn build(&self, ctx: &mut Context) -> ArcSampler {
        if let Some(sampler) = ctx.render.cache.samplers.get(self) {
            log::info!("Fetch cached sampler");
            return sampler.clone();
        }

        log::info!("Create cached sampler");
        let sampler = self.build_uncached(ctx);
        ctx.render
            .cache
            .samplers
            .insert(self.clone(), sampler.clone());
        sampler
    }
}

impl SamplerBuilder {
    pub fn label(mut self, value: String) -> Self {
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
    pub fn lod_clamp(mut self, min: f32, max: f32) -> Self {
        self.lod_min_clamp_u32 = (min * 10.0) as u32;
        self.lod_max_clamp_u32 = (max * 10.0) as u32;
        self
    }
    pub fn anisotropy_clamp(mut self, value: u16) -> Self {
        self.anisotropy_clamp = value;
        self
    }
    pub fn compare(mut self, value: wgpu::CompareFunction) -> Self {
        self.compare = Some(value);
        self
    }
    pub fn border_color(mut self, value: wgpu::SamplerBorderColor) -> Self {
        self.border_color = Some(value);
        self
    }
}

//
// Texture
//

// TODO use struct notation?
#[derive(Clone, Eq, PartialEq, Hash)]
pub enum TextureSource {
    Empty(u32, u32),
    Filled(u32, u32, Vec<u8>),
    Bytes(Vec<u8>),
}

#[derive(Clone, Eq, PartialEq, Hash)]
pub struct TextureBuilder {
    source: TextureSource,

    label: Option<String>,
    usage: wgpu::TextureUsages,
    format: wgpu::TextureFormat,
    depth_or_array_layers: u32,
    mip_level_count: u32,
    sample_count: u32,
    dimension: wgpu::TextureDimension,
    view_formats: Vec<wgpu::TextureFormat>,
}

impl TextureBuilder {
    pub fn new(source: TextureSource) -> Self {
        Self {
            source,
            label: None,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            depth_or_array_layers: 1,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            view_formats: Vec::new(),
        }
    }

    pub fn build_uncached(&self, ctx: &Context) -> Texture {
        let device = render::device(ctx);
        let queue = render::queue(ctx);
        match &self.source {
            TextureSource::Empty(width, height) => {
                let texture = device.create_texture(&wgpu::TextureDescriptor {
                    label: self.label.as_deref(),
                    size: wgpu::Extent3d {
                        width: *width,
                        height: *height,
                        depth_or_array_layers: self.depth_or_array_layers,
                    },
                    mip_level_count: self.mip_level_count,
                    sample_count: self.sample_count,
                    dimension: self.dimension,
                    format: self.format,
                    usage: self.usage,
                    view_formats: &self.view_formats,
                });

                let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

                Texture {
                    texture: ArcTexture::new(texture),
                    view: ArcTextureView::new(view),
                }
            }
            TextureSource::Filled(width, height, bytes) => {
                let texture = device.create_texture(&wgpu::TextureDescriptor {
                    label: self.label.as_deref(),
                    size: wgpu::Extent3d {
                        width: *width,
                        height: *height,
                        depth_or_array_layers: self.depth_or_array_layers,
                    },
                    mip_level_count: self.mip_level_count,
                    sample_count: self.sample_count,
                    dimension: self.dimension,
                    format: self.format,
                    usage: self.usage,
                    view_formats: &self.view_formats,
                });
                queue.write_texture(
                    wgpu::ImageCopyTexture {
                        texture: &texture,
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    bytes,
                    wgpu::ImageDataLayout {
                        offset: 0,
                        bytes_per_row: self // TODO check if correct
                            .format
                            .block_copy_size(Some(wgpu::TextureAspect::All))
                            .map(|n| *width * n),
                        rows_per_image: Some(*height),
                    },
                    texture.size(),
                );

                let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

                Texture {
                    texture: ArcTexture::new(texture),
                    view: ArcTextureView::new(view),
                }
            }
            TextureSource::Bytes(bytes) => {
                let img = image::load_from_memory(bytes)
                    .expect("could not load texture from bytes")
                    .to_rgba8();

                let texture = device.create_texture(&wgpu::TextureDescriptor {
                    label: self.label.as_deref(),
                    size: wgpu::Extent3d {
                        width: img.width(),
                        height: img.height(),
                        depth_or_array_layers: self.depth_or_array_layers,
                    },
                    mip_level_count: self.mip_level_count,
                    sample_count: self.sample_count,
                    dimension: self.dimension,
                    format: self.format,
                    usage: self.usage,
                    view_formats: &self.view_formats,
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

                Texture {
                    texture: ArcTexture::new(texture),
                    view: ArcTextureView::new(view),
                }
            }
        }
    }
    pub fn build(&self, ctx: &mut Context) -> Texture {
        if let Some(texture) = ctx.render.cache.textures.get(self) {
            log::info!("Fetch cached texture");
            return texture.clone();
        }

        log::info!("Create cached texture");
        let texture = self.build_uncached(ctx);
        ctx.render
            .cache
            .textures
            .insert(self.clone(), texture.clone());
        texture
    }
}

impl TextureBuilder {
    pub fn label(mut self, value: String) -> Self {
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
    pub fn depth_or_array_layers(mut self, value: u32) -> Self {
        self.depth_or_array_layers = value;
        self
    }
    pub fn mip_level_count(mut self, value: u32) -> Self {
        self.mip_level_count = value;
        self
    }
    pub fn sample_count(mut self, value: u32) -> Self {
        self.sample_count = value;
        self
    }
    pub fn dimension(mut self, value: wgpu::TextureDimension) -> Self {
        self.dimension = value;
        self
    }
    pub fn view_formats(mut self, value: Vec<wgpu::TextureFormat>) -> Self {
        self.view_formats = value;
        self
    }
}

#[derive(Clone, Eq, PartialEq, Hash)]
pub struct Texture {
    pub(crate) texture: ArcTexture,
    pub(crate) view: ArcTextureView,
}

impl Texture {
    pub fn texture(&self) -> ArcTexture {
        self.texture.clone()
    }

    pub fn view(&self) -> ArcTextureView {
        self.view.clone()
    }
    pub fn texture_ref(&self) -> &wgpu::Texture {
        &self.texture
    }

    pub fn view_ref(&self) -> &wgpu::TextureView {
        &self.view
    }
}
