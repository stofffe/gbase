use gbase::{
    glam::UVec2,
    render::{self},
    wgpu, Context,
};

/// Creates a texture builder from the bytes
///
/// Uses ```image``` crate for decoding
///
/// Does not account for gamma correction
pub fn texture_builder_from_image_bytes(
    bytes: &[u8],
) -> Result<render::TextureBuilder, image::ImageError> {
    let img = image::load_from_memory(bytes)?.to_rgba8();
    let builder = render::TextureBuilder::new(render::TextureSource::Data(
        img.width(),
        img.height(),
        img.to_vec(),
    ))
    .format(gbase::wgpu::TextureFormat::Rgba8Unorm);
    Ok(builder)
}

//
// Atlas
//

pub struct TextureAtlasBuilder {}

impl TextureAtlasBuilder {
    pub fn new() -> Self {
        Self {}
    }
    pub fn build(self, texture: render::TextureWithView) -> TextureAtlas {
        TextureAtlas { texture }
    }
}

impl Default for TextureAtlasBuilder {
    fn default() -> Self {
        Self::new()
    }
}

pub struct TextureAtlas {
    texture: render::TextureWithView,
}

impl TextureAtlas {
    pub fn write_texture(&mut self, ctx: &Context, origin: UVec2, dimensions: UVec2, bytes: &[u8]) {
        let queue = render::queue(ctx);

        queue.write_texture(
            wgpu::ImageCopyTexture {
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
            wgpu::ImageDataLayout {
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

    pub fn texture(&self) -> &render::TextureWithView {
        &self.texture
    }
}
