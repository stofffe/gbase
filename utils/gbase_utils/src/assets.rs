// use gbase::{asset, render, wgpu};
// use std::collections::HashMap;
//
// use crate::Image;

// pub struct PixelCache {
//     default_textures: HashMap<[u8; 4], asset::AssetHandle<GltfTexture>>,
// }

// impl PixelCache {
//     pub fn new() -> Self {
//         Self {
//             default_textures: HashMap::new(),
//         }
//     }
//
//     pub fn allocate(
//         &mut self,
//         cache: &mut gbase::asset::AssetCache,
//         value: [u8; 4],
//     ) -> asset::AssetHandle<GltfTexture> {
//         match self.default_textures.get(&value) {
//             Some(handle) => handle.clone(),
//             None => {
//                 let image_
//
//                 let image = GltfTexture {
//                     source: render::TextureSource::Data(1, 1, value.to_vec()),
//                     texture_config: render::TextureBuilder::new(),
//                     sampler_config: render::SamplerBuilder::new()
//                         .min_mag_filter(wgpu::FilterMode::Nearest, wgpu::FilterMode::Nearest),
//                 };
//                 let handle = asset::insert_asset_force(cache, image);
//                 self.default_textures.insert(value, handle.clone());
//                 handle
//             }
//         }
//     }
// }
// let name = format!("single pixel rgb {:?}", color);
// let pixel_image = RgbaImage::from_pixel(1, 1, image::Rgba(color));
// let image_handle = load_ctx
//     .insert_asset_scoped::<RgbaImage, NamedInserter>(name, pixel_image)
//     .await;
//
// let sampler_config = render::SamplerBuilder::new()
//     .min_mag_filter(wgpu::FilterMode::Nearest, wgpu::FilterMode::Nearest);
// let texture_config =
//     render::TextureBuilder::new().with_format(wgpu::TextureFormat::Rgba8Unorm);
//
// let name = format!("gltf single pixel rgb {:?}", color);
// let gltf_texture_handle = load_ctx
//     .insert_asset_scoped::<GltfTexture, NamedInserter>(
//         name,
//         GltfTexture {
//             image_handle,
//             sampler_config,
//             texture_config,
//         },
//     )
//     .await;
