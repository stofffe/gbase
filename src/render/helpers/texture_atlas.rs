//
// Texture Atlas
//

use glam::UVec2;

use crate::{
    render::{self},
    Context,
};

pub struct TextureAtlasBuilder {}

impl TextureAtlasBuilder {
    pub fn new() -> Self {
        Self {}
    }
    pub fn build(self, texture: render::TextureWithView) -> TextureAtlas {
        TextureAtlas { texture }
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
                texture: &self.texture.texture,
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
