mod keyboard;
mod mouse;

pub use keyboard::*;
pub use mouse::*;

#[derive(Default)]
pub(crate) struct InputContext {
    pub(crate) keyboard: KeyboardContext,
    pub(crate) mouse: MouseContext,
}

impl InputContext {
    pub fn new() -> Self {
        Self {
            keyboard: KeyboardContext::new(),
            mouse: MouseContext::new(),
        }
    }
}
