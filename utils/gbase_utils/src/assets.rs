use std::collections::HashMap;

use gbase::{
    asset,
    render::{self, Image},
    wgpu::{self},
};

pub struct PixelCache {
    default_textures: HashMap<[u8; 4], asset::AssetHandle<Image>>,
}

impl PixelCache {
    pub fn new() -> Self {
        Self {
            default_textures: HashMap::new(),
        }
    }

    pub fn allocate(
        &mut self,
        cache: &mut gbase::asset::AssetCache,
        value: [u8; 4],
    ) -> asset::AssetHandle<Image> {
        match self.default_textures.get(&value) {
            Some(handle) => handle.clone(),
            None => {
                let image = Image {
                    texture: render::TextureBuilder::new(render::TextureSource::Data(
                        1,
                        1,
                        value.to_vec(),
                    )),
                    sampler: render::SamplerBuilder::new()
                        .min_mag_filter(wgpu::FilterMode::Nearest, wgpu::FilterMode::Nearest),
                };
                let handle = asset::AssetBuilder::insert(image).build(cache);
                self.default_textures.insert(value, handle.clone());
                handle
            }
        }
    }
}
