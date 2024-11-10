pub use winit::event::MouseButton;

use std::collections::HashSet;

use crate::Context;

#[derive(Default)]
pub(crate) struct MouseContext {
    on_screen: bool,
    pos: (f64, f64),
    mouse_delta: (f64, f64),
    pressed: HashSet<MouseButton>,
    previous_pressed: HashSet<MouseButton>,
    scroll_delta: (f64, f64),
}

impl MouseContext {
    /// Returns true if Button is down
    /// Accepts repeating
    pub(crate) fn button_pressed(&self, keycode: MouseButton) -> bool {
        self.pressed.contains(&keycode)
    }

    /// Returns true if Button was pressed this frame
    /// Does not accept repeating
    pub(crate) fn button_just_pressed(&self, keycode: MouseButton) -> bool {
        self.pressed.contains(&keycode) && !self.previous_pressed.contains(&keycode)
    }

    /// Returns true is MouseButton was released this frame
    pub(crate) fn button_released(&self, keycode: MouseButton) -> bool {
        !self.pressed.contains(&keycode) && self.previous_pressed.contains(&keycode)
    }

    pub(crate) fn mouse_on_screen(&self) -> bool {
        self.on_screen
    }
}

impl MouseContext {
    /// Sets mouse off screen
    pub(crate) fn set_on_screen(&mut self, on_screen: bool) {
        self.on_screen = on_screen;
    }

    // Sets the current position of the mouse
    pub(crate) fn set_pos(&mut self, pos: (f64, f64)) {
        self.pos = pos;
    }

    /// Sets the (dx, dy) change in mouse position
    pub(crate) fn set_mouse_delta(&mut self, change: (f64, f64)) {
        self.mouse_delta = change;
    }

    pub(crate) fn set_scroll_delta(&mut self, change: (f64, f64)) {
        self.scroll_delta = change;
    }

    /// Sets button for current frame
    pub(crate) fn press_button(&mut self, keycode: MouseButton) {
        self.pressed.insert(keycode);
    }

    /// Release button
    pub(crate) fn release_button(&mut self, keycode: MouseButton) {
        self.pressed.remove(&keycode);
    }

    /// Save current buttons in previous
    /// Should be called each frame
    pub(crate) fn store_buttons(&mut self) {
        self.previous_pressed = self.pressed.clone()
    }
}

//
// Commands
//

/// Returns the mouse delta for the current frame
pub fn mouse_delta(ctx: &Context) -> (f32, f32) {
    let (dx, dy) = ctx.input.mouse.mouse_delta;
    (dx as f32, dy as f32)
}

/// Returns if mouse is on screen or not
pub fn mouse_on_screen(ctx: &Context) -> bool {
    ctx.input.mouse.on_screen
}

/// Returns the current physical coordinates for the mouse
pub fn mouse_pos(ctx: &Context) -> (f32, f32) {
    let (x, y) = ctx.input.mouse.pos;
    (x as f32, y as f32)
}

/// Returns true if MouseButton is pressed
/// Accepts repeating
pub fn mouse_button_pressed(ctx: &Context, keycode: MouseButton) -> bool {
    ctx.input.mouse.button_pressed(keycode)
}

/// Returns true if MouseButton was pressed this frame
pub fn mouse_button_just_pressed(ctx: &Context, keycode: MouseButton) -> bool {
    ctx.input.mouse.button_just_pressed(keycode)
}

/// Returns true if MouseButton was released this frame
pub fn mouse_button_released(ctx: &Context, keycode: MouseButton) -> bool {
    ctx.input.mouse.button_released(keycode)
}

/// Returns the scroll delta for the current frame
pub fn scroll_delta(ctx: &Context) -> (f32, f32) {
    let (dx, dy) = ctx.input.mouse.scroll_delta;
    (dx as f32, dy as f32)
}

// /// Returns the mouse position in screen space
// pub fn mouse_pos_screen(ctx: &Context) -> Vec2 {
//     let window_dim = vec2(
//         ctx.render.window_size.width as f32,
//         ctx.render.window_size.height as f32,
//     );
//     let camera_dim = vec2(ctx.render.camera.width, ctx.render.camera.height);
//     let physical_pos = mouse_pos_physical(ctx);
//     let scale = physical_pos / window_dim * camera_dim;
//     let center = scale - camera_dim / 2.0;
//     center * vec2(1.0, -1.0)
// }

// /// Returns the mouse position in world space
// pub fn mouse_pos_world(ctx: &Context) -> Vec2 {
//     mouse_pos_screen(ctx) - ctx.render.camera.pos
// }
