use super::{GUIRenderer, VertexUI, VERTEX_TYPE_SHAPE, VERTEX_TYPE_TEXT};
use gbase::glam::{vec2, Vec2, Vec4};

impl GUIRenderer {
    /// Draw a quad
    ///
    /// Internal
    #[rustfmt::skip]
    pub fn quad(&mut self, pos: Vec2, size: Vec2, color: Vec4) {
        let (x, y) = (pos.x ,pos.y);
        let (sx, sy) = (size.x, size.y);
        let color = color.to_array();
        let uv = [0.0,0.0];
        let ty = VERTEX_TYPE_SHAPE;

        let offset = self.dynamic_vertices.len() as u32;
        self.dynamic_vertices.push(VertexUI { position: [x,      -y,        0.0], color, ty, uv }); // tl
        self.dynamic_vertices.push(VertexUI { position: [x + sx, -y,        0.0], color, ty, uv }); // tr
        self.dynamic_vertices.push(VertexUI { position: [x,      -(y + sy), 0.0], color, ty, uv }); // bl
        self.dynamic_vertices.push(VertexUI { position: [x + sx, -(y + sy), 0.0], color, ty, uv }); // br
        self.dynamic_indices.push(offset); // tl
        self.dynamic_indices.push(offset + 1); // bl
        self.dynamic_indices.push(offset + 2); // tr
        self.dynamic_indices.push(offset + 2); // tr
        self.dynamic_indices.push(offset + 1); // bl
        self.dynamic_indices.push(offset + 3); // br
    }

    // TODO scaling a bit weird
    // currently size.y does nothing
    /// pos \[0,1\]
    /// scale \[0,1\]
    pub fn text(
        &mut self,
        text: &str,
        top_left: Vec2,
        bounds_size: Vec2,
        font_size: f32, // line height
        color: Vec4,
        wrap: bool,
    ) {
        let base_offset = self.font_atlas.font_info.base_offset * font_size;

        let mut global_offset = Vec2::ZERO;
        for letter in text.chars() {
            let info = self.font_atlas.get_info(letter);

            let size = info.size_unorm * font_size;

            let loc_offset = info.local_offset * font_size;
            let adv = info.advance * font_size;

            // word wrapping
            if wrap && (global_offset.x + size.x + loc_offset.x) > bounds_size.x {
                global_offset.x = 0.0;
                global_offset.y += font_size;
            }

            let offset = top_left
                + global_offset
                + vec2(0.0, -base_offset)
                + vec2(loc_offset.x, -loc_offset.y)
                + vec2(0.0, font_size - size.y);

            self.letter(offset, font_size, letter, color);
            global_offset.x += adv.x;
        }
    }

    #[rustfmt::skip]
    /// pos \[0,1\]
    /// line height \[0,1\]
    pub fn letter(&mut self, pos: Vec2, font_size: f32, letter: char, color: Vec4) {
        let info = self.font_atlas.get_info(letter);

        let atlas_offset = info.atlas_offset;
        let atlas_dim = info.atlas_dimensions;

        let scaled_dim = info.size_unorm * font_size;

        let (x, y) = (pos.x, pos.y);
        let (sx, sy)= (scaled_dim.x, scaled_dim.y);
        let (tox, toy) = (atlas_offset.x, atlas_offset.y);
        let (tdx, tdy) =(atlas_dim.x, atlas_dim.y);
        let color = color.to_array();
        let ty = VERTEX_TYPE_TEXT;

        let vertex_offset = self.dynamic_vertices.len() as u32;
        self.dynamic_vertices.push(VertexUI { position: [x,      -y,        0.0], ty, color, uv: [tox,       toy      ] }); // tl
        self.dynamic_vertices.push(VertexUI { position: [x + sx, -y,        0.0], ty, color, uv: [tox + tdx, toy      ] }); // tr
        self.dynamic_vertices.push(VertexUI { position: [x,      -(y + sy), 0.0], ty, color, uv: [tox,       toy + tdy] }); // bl
        self.dynamic_vertices.push(VertexUI { position: [x + sx, -(y + sy), 0.0], ty, color, uv: [tox + tdx, toy + tdy] }); // br
        self.dynamic_indices.push(vertex_offset);     // tl
        self.dynamic_indices.push(vertex_offset + 1); // bl
        self.dynamic_indices.push(vertex_offset + 2); // tr
        self.dynamic_indices.push(vertex_offset + 2); // tr
        self.dynamic_indices.push(vertex_offset + 1); // bl
        self.dynamic_indices.push(vertex_offset + 3); // br
    }
}
