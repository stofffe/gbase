use crate::{
    render::{self, next_id, ArcSampler, ArcTexture, ArcTextureView},
    Context,
};

#[derive(Debug, Clone)]
pub struct Image {
    pub texture: render::TextureBuilder,
    pub sampler: render::SamplerBuilder,
}

impl Image {
    pub fn new_pixel_texture(color: [u8; 4]) -> Self {
        Self {
            texture: TextureBuilder::new(render::TextureSource::Data(1, 1, color.to_vec()))
                .with_format(wgpu::TextureFormat::Rgba8Unorm),
            sampler: SamplerBuilder::new()
                .min_mag_filter(wgpu::FilterMode::Nearest, wgpu::FilterMode::Nearest),
        }
    }
}

//
// Sampler
//

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct SamplerBuilder {
    label: Option<String>,
    address_mode_u: wgpu::AddressMode,
    address_mode_v: wgpu::AddressMode,
    address_mode_w: wgpu::AddressMode,
    mag_filter: wgpu::FilterMode,
    min_filter: wgpu::FilterMode,
    mip_map_filter: wgpu::FilterMode,
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
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mip_map_filter: wgpu::FilterMode::Linear,
            lod_min_clamp_u32: 0,
            lod_max_clamp_u32: 0,
            anisotropy_clamp: 1,
            compare: None,
            border_color: None,
        }
    }

    pub fn descriptor(&self) -> wgpu::SamplerDescriptor<'_> {
        let lod_min_clamp_f32 = self.lod_min_clamp_u32 as f32 / 10.0;
        let lod_max_clamp_f32 = self.lod_max_clamp_u32 as f32 / 10.0;

        wgpu::SamplerDescriptor {
            label: self.label.as_deref(),
            address_mode_u: self.address_mode_u,
            address_mode_v: self.address_mode_v,
            address_mode_w: self.address_mode_w,
            mag_filter: self.mag_filter,
            min_filter: self.min_filter,
            mipmap_filter: self.mip_map_filter,
            lod_min_clamp: lod_min_clamp_f32,
            lod_max_clamp: lod_max_clamp_f32,
            anisotropy_clamp: self.anisotropy_clamp,
            compare: self.compare,
            border_color: self.border_color,
        }
    }

    pub fn build_uncached_inner(&self, ctx: &mut Context) -> ArcSampler {
        let device = &ctx.render.device;
        let sampler = device.create_sampler(&self.descriptor());
        ArcSampler::new(ctx, sampler)
    }

    pub fn build_uncached(&self, ctx: &mut Context) -> ArcSampler {
        self.build_uncached_inner(ctx)
    }

    pub fn build(self, ctx: &mut Context) -> ArcSampler {
        if let Some(sampler) = ctx.render.cache.samplers.get(&self) {
            return sampler.clone();
        }

        tracing::info!("Create cached sampler");
        let sampler = self.build_uncached_inner(ctx);
        ctx.render
            .cache
            .samplers
            .insert(self.clone(), sampler.clone());
        sampler
    }
}

impl SamplerBuilder {
    pub fn label(mut self, value: impl Into<String>) -> Self {
        self.label = Some(value.into());
        self
    }
    pub fn min_mag_filter(mut self, min: wgpu::FilterMode, mag: wgpu::FilterMode) -> Self {
        self.min_filter = min;
        self.mag_filter = mag;
        self
    }
    pub fn mip_map_filer(mut self, filter: wgpu::FilterMode) -> Self {
        self.mip_map_filter = filter;
        self
    }
    pub fn with_address_mode(mut self, mode: wgpu::AddressMode) -> Self {
        self.address_mode_u = mode;
        self.address_mode_v = mode;
        self.address_mode_w = mode;
        self
    }
    pub fn set_address_mode(&mut self, mode: wgpu::AddressMode) {
        self.address_mode_u = mode;
        self.address_mode_v = mode;
        self.address_mode_w = mode;
    }

    pub fn address_mode_separate(
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
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum TextureSource {
    /// (width, height, bytes)
    Data(u32, u32, Vec<u8>),
    /// (width, height)
    Empty(u32, u32),
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct TextureBuilder {
    pub source: TextureSource,

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

    pub fn build(&self, ctx: &mut Context) -> render::ArcTexture {
        let device = render::device(ctx);
        let queue = render::queue(ctx);
        match self.source {
            TextureSource::Empty(width, height) => {
                let texture = device.create_texture(&wgpu::TextureDescriptor {
                    label: self.label.as_deref(),
                    size: wgpu::Extent3d {
                        width,
                        height,
                        depth_or_array_layers: self.depth_or_array_layers,
                    },
                    mip_level_count: self.mip_level_count,
                    sample_count: self.sample_count,
                    dimension: self.dimension,
                    format: self.format,
                    usage: self.usage,
                    view_formats: &self.view_formats,
                });

                ArcTexture::new(ctx, texture)
            }
            TextureSource::Data(width, height, ref bytes) => {
                let texture = device.create_texture(&wgpu::TextureDescriptor {
                    label: self.label.as_deref(),
                    size: wgpu::Extent3d {
                        width,
                        height,
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
                    wgpu::TexelCopyTextureInfo {
                        texture: &texture,
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    bytes,
                    wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: self // TODO check if correct
                            .format
                            .block_copy_size(Some(wgpu::TextureAspect::All))
                            .map(|n| width * n),
                        rows_per_image: Some(height),
                    },
                    texture.size(),
                );

                ArcTexture::new(ctx, texture)
            }
        }
    }
}

impl TextureBuilder {
    pub fn label(mut self, value: impl Into<String>) -> Self {
        self.label = Some(value.into());
        self
    }
    pub fn usage(mut self, value: wgpu::TextureUsages) -> Self {
        self.usage = value;
        self
    }
    pub fn with_format(mut self, value: wgpu::TextureFormat) -> Self {
        self.format = value;
        self
    }
    pub fn set_format(&mut self, value: wgpu::TextureFormat) {
        self.format = value;
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

//
// Texture view
//

#[derive(Clone, Eq, PartialEq, Hash)]
pub struct TextureViewBuilder {
    texture: render::ArcTexture,

    label: Option<String>,
    format: Option<wgpu::TextureFormat>,
    dimension: Option<wgpu::TextureViewDimension>,
    aspect: wgpu::TextureAspect,
    base_mip_level: u32,
    mip_level_count: Option<u32>,
    base_array_layer: u32,
    array_layer_count: Option<u32>,
    usage: Option<wgpu::TextureUsages>,
}

impl TextureViewBuilder {
    pub fn new(texture: render::ArcTexture) -> Self {
        Self {
            texture,
            label: None,
            format: None,
            dimension: None,
            aspect: wgpu::TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: None,
            base_array_layer: 0,
            array_layer_count: None,
            usage: None,
        }
    }

    pub fn build_uncached(&self, ctx: &mut Context) -> render::ArcTextureView {
        let view = self.texture.create_view(&wgpu::TextureViewDescriptor {
            label: self.label.as_deref(),
            format: self.format,
            dimension: self.dimension,
            aspect: self.aspect,
            base_mip_level: self.base_mip_level,
            mip_level_count: self.mip_level_count,
            base_array_layer: self.base_array_layer,
            array_layer_count: self.array_layer_count,
            usage: self.usage,
        });

        render::ArcTextureView::new(ctx, view)
    }

    pub fn build(self, ctx: &mut Context) -> render::ArcTextureView {
        if let Some(view) = ctx.render.cache.texture_views.get(&self) {
            return view.clone();
        }

        tracing::info!("Create cached texture view");
        let view = self.build_uncached(ctx);
        ctx.render.cache.texture_views.insert(self, view.clone());
        view
    }
}

impl TextureViewBuilder {
    pub fn label(mut self, value: impl Into<String>) -> Self {
        self.label = Some(value.into());
        self
    }
    pub fn format(mut self, value: wgpu::TextureFormat) -> Self {
        self.format = Some(value);
        self
    }
    pub fn dimension(mut self, value: wgpu::TextureViewDimension) -> Self {
        self.dimension = Some(value);
        self
    }
    pub fn aspect(mut self, value: wgpu::TextureAspect) -> Self {
        self.aspect = value;
        self
    }
    pub fn base_mip_level(mut self, value: u32) -> Self {
        self.base_mip_level = value;
        self
    }
    pub fn mip_level_count(mut self, value: u32) -> Self {
        self.mip_level_count = Some(value);
        self
    }
    pub fn base_array_layer(mut self, value: u32) -> Self {
        self.base_array_layer = value;
        self
    }
    pub fn array_layer_count(mut self, value: u32) -> Self {
        self.array_layer_count = Some(value);
        self
    }
    pub fn usage(mut self, value: wgpu::TextureUsages) -> Self {
        self.usage = Some(value);
        self
    }
}

//
// Texture with view
//

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub struct GpuImage {
    texture: ArcTexture,
    view: ArcTextureView,
    sampler: ArcSampler,
}

impl GpuImage {
    pub fn new(texture: ArcTexture, view: ArcTextureView, sampler: ArcSampler) -> Self {
        Self {
            texture,
            view,
            sampler,
        }
    }
    pub fn from_texture(ctx: &mut Context, texture: ArcTexture) -> Self {
        let view = render::TextureViewBuilder::new(texture.clone()).build(ctx);
        let sampler = SamplerBuilder::new().build(ctx);
        Self {
            texture,
            view,
            sampler,
        }
    }
    pub fn from_image(ctx: &mut Context, image: Image) -> Self {
        let texture = image.texture.clone().build(ctx);
        let sampler = image.sampler.clone().build(ctx);
        let view = render::TextureViewBuilder::new(texture.clone()).build(ctx);
        Self {
            texture,
            view,
            sampler,
        }
    }
    pub fn texture(&self) -> ArcTexture {
        self.texture.clone()
    }
    pub fn view(&self) -> ArcTextureView {
        self.view.clone()
    }
    pub fn sampler(&self) -> ArcSampler {
        self.sampler.clone()
    }
    pub fn texture_ref(&self) -> &wgpu::Texture {
        &self.texture
    }
    pub fn view_ref(&self) -> &wgpu::TextureView {
        &self.view
    }
    pub fn sampler_ref(&self) -> &wgpu::Sampler {
        &self.sampler
    }
}

impl render::ArcTexture {
    pub fn with_default_sampler_and_view(self, ctx: &mut Context) -> render::GpuImage {
        render::GpuImage::from_texture(ctx, self)
    }
}
