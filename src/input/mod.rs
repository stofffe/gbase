mod keyboard;
mod mouse;

pub use keyboard::*;
pub use mouse::*;

#[derive(Default)]
pub(crate) struct InputContext {
    pub keyboard: KeyboardContext,
    pub mouse: MouseContext,
}

impl InputContext {}
