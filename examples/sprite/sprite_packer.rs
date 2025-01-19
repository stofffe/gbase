use gbase::glam::{vec2, Vec2};
use gbase::log;
use gbase::{render, wgpu, Context};

pub type SpriteHandle = usize;
#[derive(Debug, Default, Clone)]
pub struct SpriteInfo {
    offset: Vec2,
    dimension: Vec2,
}

impl SpriteInfo {
    fn new(x: u32, y: u32, w: u32, h: u32, atlas_width: u32, atlas_height: u32) -> Self {
        Self {
            offset: vec2(
                x as f32 / atlas_width as f32,
                y as f32 / atlas_height as f32,
            ),
            dimension: vec2(
                w as f32 / atlas_width as f32,
                h as f32 / atlas_height as f32,
            ),
        }
    }
}

pub struct SpriteAtlasBuilder {
    atlas_width: u32,
    atlas_height: u32,
    sprites: Vec<(render::ArcTexture, SpriteHandle)>,
}

impl SpriteAtlasBuilder {
    pub fn new(atlas_width: u32, atlas_height: u32) -> Self {
        Self {
            atlas_width,
            atlas_height,
            sprites: Vec::new(),
        }
    }

    pub fn add_sprite(&mut self, texture: render::ArcTexture) -> SpriteHandle {
        let handle = self.sprites.len();
        self.sprites.push((texture, handle));
        handle
    }

    pub fn build(mut self, ctx: &mut Context) -> SpriteAtlas {
        self.sprites
            .sort_by_key(|(texture, _)| std::cmp::Reverse(texture.height()));

        let atlas_texture = render::TextureBuilder::new(render::TextureSource::Empty(
            self.atlas_width,
            self.atlas_height,
        ))
        .label("atlas")
        .usage(wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST)
        .format(wgpu::TextureFormat::Rgba8Unorm)
        .build(ctx);
        let atlas_texture_view = render::TextureViewBuilder::new(atlas_texture.clone()).build(ctx);

        let mut info = vec![SpriteInfo::default(); self.sprites.len()];

        let mut encoder = render::EncoderBuilder::new().build(ctx);
        let mut shelf_height = 0;
        let mut x = self.atlas_width;
        let mut y = 0;
        for (sprite, handle) in self.sprites {
            let (w, h) = (sprite.width(), sprite.height());

            // bounds check
            if x + w > self.atlas_width {
                log::info!("wrap atlas");

                x = 0;
                y += shelf_height;
                shelf_height = h;
            }
            if y + h > self.atlas_height {
                panic!("ran out of space packing sprites")
            }

            info[handle] = SpriteInfo::new(x, y, w, h, self.atlas_width, self.atlas_height);
            encoder.copy_texture_to_texture(
                sprite.as_image_copy(),
                wgpu::ImageCopyTextureBase {
                    texture: &atlas_texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d { x, y, z: 0 },
                    aspect: wgpu::TextureAspect::All,
                },
                sprite.size(),
            );

            x += w;
        }

        render::queue(ctx).submit(Some(encoder.finish()));

        SpriteAtlas {
            atlas_texture,
            atlas_texture_view,
            info,
        }
    }
}

pub struct SpriteAtlas {
    atlas_texture: render::ArcTexture,
    atlas_texture_view: render::ArcTextureView,

    info: Vec<SpriteInfo>,
}

impl SpriteAtlas {
    pub fn get_info(&self, handle: SpriteHandle) -> SpriteInfo {
        self.info[handle].clone()
    }

    pub fn view(&self) -> render::ArcTextureView {
        self.atlas_texture_view.clone()
    }
}
