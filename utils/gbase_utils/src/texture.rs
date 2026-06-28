use gbase::{
    glam::UVec2,
    render::{self},
    wgpu, Context,
};

/// Creates a RGBA8 texture source from the bytes
///
/// Uses ```image``` crate for decoding
///
/// Does not account for gamma correction
pub fn texture_source_from_image_bytes(
    bytes: &[u8],
) -> Result<render::TextureSource, image::ImageError> {
    let img = image::load_from_memory(bytes)?.to_rgba8();
    let source = render::TextureSource::Data(img.width(), img.height(), img.to_vec());
    Ok(source)
}

//
// Atlas
//

pub struct TextureAtlasBuilder {}

impl TextureAtlasBuilder {
    pub fn new() -> Self {
        Self {}
    }
    pub fn build(self, texture: render::GpuImage) -> TextureAtlas {
        TextureAtlas { texture }
    }
}

impl Default for TextureAtlasBuilder {
    fn default() -> Self {
        Self::new()
    }
}

pub struct TextureAtlas {
    texture: render::GpuImage,
}

impl TextureAtlas {
    pub fn write_texture(&mut self, ctx: &Context, origin: UVec2, dimensions: UVec2, bytes: &[u8]) {
        let queue = render::queue(ctx);

        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture.texture(),
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: origin.x,
                    y: origin.y,
                    z: 0,
                },
                aspect: wgpu::TextureAspect::All,
            },
            bytes,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(dimensions.x), // TODO * 4? * block size
                rows_per_image: Some(dimensions.y),
            },
            wgpu::Extent3d {
                width: dimensions.x,
                height: dimensions.y,
                depth_or_array_layers: 1,
            },
        );
    }

    pub fn texture(&self) -> &render::GpuImage {
        &self.texture
    }
}
