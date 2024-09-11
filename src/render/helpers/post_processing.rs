use crate::render;

pub fn fill(texture: render::Texture) {
    let new_texture = render::TextureBuilder::new(render::TextureSource::Empty(
        texture.texture_ref().width(),
        texture.texture_ref().height(),
    ));
}
