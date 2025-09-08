use std::collections::HashSet;
pub use winit::keyboard::KeyCode;
use winit::keyboard::ModifiersKeyState;

use crate::Context;

#[derive(Default)]
pub(crate) struct KeyboardContext {
    pressed: HashSet<KeyCode>,
    previous_pressed: HashSet<KeyCode>,
    pressed_modifiers: HashSet<KeyModifier>,
    previous_pressed_modifiers: HashSet<KeyModifier>,
}

impl KeyboardContext {
    pub fn new() -> Self {
        Self {
            pressed: HashSet::new(),
            previous_pressed: HashSet::new(),
            pressed_modifiers: HashSet::new(),
            previous_pressed_modifiers: HashSet::new(),
        }
    }
    pub(crate) fn store_state(&mut self) {
        self.store_keys();
        self.store_modifiers();
    }
}

#[derive(Debug, Hash, Ord, PartialOrd, PartialEq, Eq, Clone, Copy)]
pub enum KeyModifier {
    LShift,
    RShift,
    LCtrl,
    RCtrl,
    LAlt,
    RAlt,
    LSuper,
    RSuper,
}

// Getting keys
impl KeyboardContext {
    /// Returns true if KeyCode is down
    /// Accepts repeating
    pub fn key_pressed(&self, keycode: KeyCode) -> bool {
        self.pressed.contains(&keycode)
    }

    /// Returns true if KeyCode was pressed this frame
    /// Does not accepts repeating
    pub fn key_just_pressed(&self, keycode: KeyCode) -> bool {
        self.pressed.contains(&keycode) && !self.previous_pressed.contains(&keycode)
    }

    /// Returns true is KeyCode was released this frame
    pub fn key_released(&self, keycode: KeyCode) -> bool {
        !self.pressed.contains(&keycode) && self.previous_pressed.contains(&keycode)
    }

    pub fn modifier_pressed(&self, modifier: KeyModifier) -> bool {
        self.pressed_modifiers.contains(&modifier)
    }

    pub fn modifier_just_pressed(&self, modifier: KeyModifier) -> bool {
        self.pressed_modifiers.contains(&modifier)
            && !self.previous_pressed_modifiers.contains(&modifier)
    }

    pub fn modifier_released(&self, modifier: KeyModifier) -> bool {
        !self.pressed_modifiers.contains(&modifier)
            && self.previous_pressed_modifiers.contains(&modifier)
    }
}

impl KeyboardContext {
    /// Sets key for current frame
    pub(crate) fn set_key(&mut self, keycode: KeyCode) {
        self.pressed.insert(keycode);
    }

    /// Release key
    pub(crate) fn release_key(&mut self, keycode: KeyCode) {
        self.pressed.remove(&keycode);
    }

    pub(crate) fn modifiers_changed(&mut self, state: winit::event::Modifiers) {
        self.pressed_modifiers.clear();
        if let ModifiersKeyState::Pressed = state.lshift_state() {
            self.pressed_modifiers.insert(KeyModifier::LShift);
        }
        if let ModifiersKeyState::Pressed = state.rshift_state() {
            self.pressed_modifiers.insert(KeyModifier::RShift);
        }
        if let ModifiersKeyState::Pressed = state.lcontrol_state() {
            self.pressed_modifiers.insert(KeyModifier::LCtrl);
        }
        if let ModifiersKeyState::Pressed = state.rcontrol_state() {
            self.pressed_modifiers.insert(KeyModifier::RCtrl);
        }
        if let ModifiersKeyState::Pressed = state.lalt_state() {
            self.pressed_modifiers.insert(KeyModifier::LAlt);
        }
        if let ModifiersKeyState::Pressed = state.ralt_state() {
            self.pressed_modifiers.insert(KeyModifier::RAlt);
        }
        if let ModifiersKeyState::Pressed = state.lsuper_state() {
            self.pressed_modifiers.insert(KeyModifier::LSuper);
        }
        if let ModifiersKeyState::Pressed = state.rsuper_state() {
            self.pressed_modifiers.insert(KeyModifier::RSuper);
        }
    }

    /// Save current keys in previous
    /// Should be called each frame
    pub(crate) fn store_keys(&mut self) {
        self.previous_pressed = self.pressed.clone();
    }

    /// Save current modifiers in previous
    /// Should be called each frame
    pub(crate) fn store_modifiers(&mut self) {
        self.previous_pressed_modifiers = self.pressed_modifiers.clone();
    }
}

//
// Commands
//

/// Returns true if KeyCode is pressed
/// Accepts repeating
pub fn key_pressed(ctx: &Context, keycode: KeyCode) -> bool {
    ctx.input.keyboard.key_pressed(keycode)
}

/// Returns true if KeyCode was pressed this frame
pub fn key_just_pressed(ctx: &Context, keycode: KeyCode) -> bool {
    ctx.input.keyboard.key_just_pressed(keycode)
}

/// Returns true is KeyCode was released this frame
pub fn key_released(ctx: &Context, keycode: KeyCode) -> bool {
    ctx.input.keyboard.key_released(keycode)
}

/// Returns true if KeyModifer is pressed
/// Accepts repeating
pub fn modifier_pressed(ctx: &Context, key_modifier: KeyModifier) -> bool {
    ctx.input.keyboard.modifier_pressed(key_modifier)
}

/// Returns true if KeyModifer was pressed this frame
pub fn modifer_just_pressed(ctx: &Context, key_modifier: KeyModifier) -> bool {
    ctx.input.keyboard.modifier_just_pressed(key_modifier)
}

/// Returns true if KeyModifier was released this frame
pub fn modifer_released(ctx: &Context, key_modifier: KeyModifier) -> bool {
    ctx.input.keyboard.modifier_released(key_modifier)
}

//
// Tests
//

#[cfg(test)]
mod tests {
    // use winit::event::Modifiers;
    // use winit::keyboard::ModifiersKeyState;
    // use winit::keyboard::ModifiersState;

    use crate::input::KeyCode;
    // use crate::input::KeyModifier;
    use crate::input::KeyboardContext;

    #[test]
    fn key_pressed_test() {
        let mut kc = KeyboardContext::default();

        kc.set_key(KeyCode::KeyA);

        assert!(kc.key_pressed(KeyCode::KeyA));
        assert!(!kc.key_pressed(KeyCode::KeyB));

        kc.store_keys();
        kc.set_key(KeyCode::KeyB);

        assert!(kc.key_pressed(KeyCode::KeyA));
        assert!(kc.key_pressed(KeyCode::KeyB));

        kc.store_keys();
        kc.release_key(KeyCode::KeyA);

        assert!(!kc.key_pressed(KeyCode::KeyA));
        assert!(kc.key_pressed(KeyCode::KeyB));
    }

    #[test]
    fn key_just_pressed_test() {
        let mut kc = KeyboardContext::default();
        kc.set_key(KeyCode::KeyA);

        assert!(kc.key_just_pressed(KeyCode::KeyA));

        kc.store_keys();
        kc.set_key(KeyCode::KeyA);

        assert!(!kc.key_just_pressed(KeyCode::KeyA));
    }

    #[test]
    fn key_released_test() {
        let mut kc = KeyboardContext::default();
        kc.set_key(KeyCode::KeyA);

        assert!(!kc.key_released(KeyCode::KeyA));

        kc.store_keys();
        kc.release_key(KeyCode::KeyA);

        assert!(kc.key_released(KeyCode::KeyA));
    }

    // #[test]
    // fn modifer_pressed_test() {
    //     let mut kc = KeyboardContext::default();
    //
    //     // Press Shift
    //     let mut modifiers = ModifiersState::default();
    //     modifiers.insert(ModifiersState::SHIFT);
    //     kc.modifiers_changed(modifiers.into());
    //
    //     assert!(kc.modifier_pressed(KeyModifier::LShift));
    //     assert!(!kc.modifier_pressed(KeyModifier::LCtrl));
    //
    //     kc.save_modifiers();
    //
    //     // Press Shift and Ctrl
    //     let mut modifiers = ModifiersState::default();
    //     modifiers.insert(ModifiersState::SHIFT);
    //     modifiers.insert(ModifiersState::CONTROL);
    //     kc.modifiers_changed(modifiers.into());
    //
    //     assert!(kc.modifier_pressed(KeyModifier::LShift));
    //     assert!(kc.modifier_pressed(KeyModifier::LCtrl));
    //
    //     kc.save_modifiers();
    //
    //     // Release Shift
    //     let mut modifiers = ModifiersState::default();
    //     modifiers.insert(ModifiersState::CONTROL);
    //     kc.modifiers_changed(modifiers.into());
    //
    //     assert!(!kc.modifier_pressed(KeyModifier::LShift));
    //     assert!(kc.modifier_pressed(KeyModifier::LCtrl));
    // }
    //
    // #[test]
    // fn modifier_just_pressed_test() {
    //     let mut kc = KeyboardContext::default();
    //     // Press shift
    //     kc.modifiers_changed(Modifiers::from(ModifiersState::SHIFT));
    //
    //     assert!(kc.modifier_just_pressed(KeyModifier::LShift));
    //
    //     kc.save_modifiers();
    //
    //     // Release shift
    //     kc.modifiers_changed(Modifiers::from(ModifiersState::default()));
    //
    //     assert!(!kc.modifier_just_pressed(KeyModifier::LShift));
    // }
    //
    // #[test]
    // fn modifier_released_test() {
    //     let mut kc = KeyboardContext::default();
    //
    //     // Press shift
    //     kc.modifiers_changed(Modifiers::from(ModifiersState::SHIFT));
    //
    //     assert!(!kc.modifier_released(KeyModifier::LShift));
    //     assert!(!kc.modifier_released(KeyModifier::LCtrl));
    //
    //     kc.save_modifiers();
    //
    //     // Release shift
    //     kc.modifiers_changed(Modifiers::from(ModifiersState::default()));
    //
    //     assert!(kc.modifier_released(KeyModifier::LShift));
    //     assert!(!kc.modifier_released(KeyModifier::LCtrl));
    // }
}
