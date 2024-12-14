use super::{GUIRenderer, UiID, VertexUI, VERTEX_TYPE_SHAPE, VERTEX_TYPE_TEXT};
use crate::collision::Quad;
use crate::{collision, input, render, Context};
use glam::Vec2;
use glam::{vec2, vec4, Vec4};
use winit::event::MouseButton;

const _NORMAL_COLOR: Vec4 = render::GRAY;
const _HOT_COLOR: Vec4 = render::RED;
const _ACTIVE_COLOR: Vec4 = render::GREEN;

impl GUIRenderer {
    #[rustfmt::skip]
    pub fn quad(&mut self, quad: Quad, color: Vec4) {
        let (x, y) = (quad.origin.x ,quad.origin.y);
        let (sx, sy) = (quad.dimension.x, quad.dimension.y);
        let color = color.to_array();
        let uv = [0.0,0.0];
        let ty = VERTEX_TYPE_SHAPE;

        let offset = self.dynamic_vertices.len() as u32;
        self.dynamic_vertices.push(VertexUI { position: [-1.0 + x * 2.0,            1.0 - y * 2.0,            0.0], color, ty, uv }); // tl
        self.dynamic_vertices.push(VertexUI { position: [-1.0 + x * 2.0 + sx * 2.0, 1.0 - y * 2.0,            0.0], color, ty, uv }); // tr
        self.dynamic_vertices.push(VertexUI { position: [-1.0 + x * 2.0,            1.0 - y * 2.0 - sy * 2.0, 0.0], color, ty, uv }); // bl
        self.dynamic_vertices.push(VertexUI { position: [-1.0 + x * 2.0 + sx * 2.0, 1.0 - y * 2.0 - sy * 2.0, 0.0], color, ty, uv }); // br
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
    pub fn text(&mut self, text: &str, quad: Quad, line_height: f32, color: Vec4, wrap: bool) {
        let mut global_offset = vec2(0.0, 0.0); // [0,1]
        for letter in text.chars() {
            let info = self.font_atlas.get_info(letter);
            let size = info.size * line_height;
            let loc_offset = info.local_offset * line_height;
            let adv = info.advance * line_height;

            // word wrapping
            if wrap && (global_offset.x + size.x) > quad.dimension.x {
                global_offset.x = 0.0;
                global_offset.y += line_height;
            }

            let offset = quad.origin
                + global_offset
                + vec2(loc_offset.x, -loc_offset.y)
                + vec2(0.0, line_height - size.y);
            self.letter(offset, line_height, letter, color);
            global_offset.x += adv.x;
        }
    }

    #[rustfmt::skip]
    /// pos \[0,1\]
    /// line height \[0,1\]
    pub fn letter(&mut self, pos: Vec2, line_height: f32, letter: char, color: Vec4) {
        let info = self.font_atlas.get_info(letter);

        let atlas_offset = info.atlas_offset;
        let atlas_dim = info.atlas_dimensions;

        let scaled_dim = info.size * line_height;

        let (x, y) = (pos.x, pos.y);
        let (sx, sy)= (scaled_dim.x, scaled_dim.y);
        let (tox, toy) = (atlas_offset.x, atlas_offset.y);
        let (tdx, tdy) =(atlas_dim.x, atlas_dim.y);
        let color = color.to_array();
        let ty = VERTEX_TYPE_TEXT;

        let vertex_offset = self.dynamic_vertices.len() as u32;
        self.dynamic_vertices.push(VertexUI { position: [-1.0 + x * 2.0,            1.0 - y * 2.0,            0.0], ty, color, uv: [tox,       toy] }); // tl
        self.dynamic_vertices.push(VertexUI { position: [-1.0 + x * 2.0 + sx * 2.0, 1.0 - y * 2.0,            0.0], ty, color, uv: [tox + tdx, toy] }); // tr
        self.dynamic_vertices.push(VertexUI { position: [-1.0 + x * 2.0,            1.0 - y * 2.0 - sy * 2.0, 0.0], ty, color, uv: [tox,       toy + tdy] }); // bl
        self.dynamic_vertices.push(VertexUI { position: [-1.0 + x * 2.0 + sx * 2.0, 1.0 - y * 2.0 - sy * 2.0, 0.0], ty, color, uv: [tox + tdx, toy + tdy] }); // br
        self.dynamic_indices.push(vertex_offset); // tl
        self.dynamic_indices.push(vertex_offset + 1); // bl
        self.dynamic_indices.push(vertex_offset + 2); // tr
        self.dynamic_indices.push(vertex_offset + 2); // tr
        self.dynamic_indices.push(vertex_offset + 1); // bl
        self.dynamic_indices.push(vertex_offset + 3); // br
    }

    #[allow(clippy::too_many_arguments)]
    pub fn button_text(
        &mut self,
        ctx: &mut Context,
        label: &str,
        quad: Quad,
        color: Vec4,
        text: &str,
        line_height: f32,
        wrap: bool,
    ) -> bool {
        let result = self.button(ctx, label, quad, color);
        self.text(text, quad, line_height, vec4(1.0, 1.0, 1.0, 1.0), wrap);
        result
    }
    pub fn button(&mut self, ctx: &Context, label: &str, quad: Quad, color: Vec4) -> bool {
        let id = UiID::new(label);

        let mouse_up = input::mouse_button_released(ctx, MouseButton::Left);
        let mouse_down = input::mouse_button_just_pressed(ctx, MouseButton::Left);
        let inside = collision::point_quad_collision(input::mouse_pos_unorm(ctx), quad);

        let mut result = false;

        // active
        if self.check_active(id) && mouse_up {
            if self.check_hot(id) {
                result = true;
            }
            self.clear_active();
        } else if self.check_hot(id) && mouse_down {
            self.set_active(id);
        }

        if inside {
            self.set_hot(id);
        } else if self.check_hot(id) {
            self.clear_hot();
        }

        self.quad(quad, color);

        result
    }
}
