pub use keyboard_types::Code as KeyCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum PhysicalKey {
    /// A known key code
    Code(KeyCode),

    /// This variant is used when the key cannot be translated to a [`KeyCode`]
    ///
    /// The native keycode is provided (if available) so you're able to more reliably match
    /// key-press and key-release events by hashing the [`PhysicalKey`]. It is also possible to use
    /// this for keybinds for non-standard keys, but such keybinds are tied to a given platform.
    Unidentified(u32),
}

impl PhysicalKey {
    /// The ASCII character this key produces on a US QWERTY layout, without modifiers.
    ///
    /// Used by [`crate::Key::shortcut_key`] to match keyboard shortcuts on non-Latin layouts.
    ///
    /// Ideally, if winit gains support for querying keyboard layouts, we could use that to better
    /// map the physical key, to what the user's actual key labels say (since most layouts have
    /// QWERTY-like labels even when not primarily an English keyboard)
    pub fn us_layout_character(self) -> Option<&'static str> {
        let Self::Code(code) = self else {
            return None;
        };
        Some(match code {
            KeyCode::Space => " ",
            KeyCode::Comma => ",",
            KeyCode::Period | KeyCode::NumpadDecimal => ".",
            KeyCode::Semicolon => ";",
            KeyCode::Backslash => "\\",
            KeyCode::Slash | KeyCode::NumpadDivide => "/",
            KeyCode::BracketLeft => "[",
            KeyCode::BracketRight => "]",
            KeyCode::Backquote => "`",
            KeyCode::Quote => "'",
            KeyCode::Minus | KeyCode::NumpadSubtract => "-",
            KeyCode::NumpadAdd => "+",
            KeyCode::NumpadMultiply => "*",
            KeyCode::Equal => "=",

            KeyCode::Digit0 | KeyCode::Numpad0 => "0",
            KeyCode::Digit1 | KeyCode::Numpad1 => "1",
            KeyCode::Digit2 | KeyCode::Numpad2 => "2",
            KeyCode::Digit3 | KeyCode::Numpad3 => "3",
            KeyCode::Digit4 | KeyCode::Numpad4 => "4",
            KeyCode::Digit5 | KeyCode::Numpad5 => "5",
            KeyCode::Digit6 | KeyCode::Numpad6 => "6",
            KeyCode::Digit7 | KeyCode::Numpad7 => "7",
            KeyCode::Digit8 | KeyCode::Numpad8 => "8",
            KeyCode::Digit9 | KeyCode::Numpad9 => "9",

            KeyCode::KeyA => "a",
            KeyCode::KeyB => "b",
            KeyCode::KeyC => "c",
            KeyCode::KeyD => "d",
            KeyCode::KeyE => "e",
            KeyCode::KeyF => "f",
            KeyCode::KeyG => "g",
            KeyCode::KeyH => "h",
            KeyCode::KeyI => "i",
            KeyCode::KeyJ => "j",
            KeyCode::KeyK => "k",
            KeyCode::KeyL => "l",
            KeyCode::KeyM => "m",
            KeyCode::KeyN => "n",
            KeyCode::KeyO => "o",
            KeyCode::KeyP => "p",
            KeyCode::KeyQ => "q",
            KeyCode::KeyR => "r",
            KeyCode::KeyS => "s",
            KeyCode::KeyT => "t",
            KeyCode::KeyU => "u",
            KeyCode::KeyV => "v",
            KeyCode::KeyW => "w",
            KeyCode::KeyX => "x",
            KeyCode::KeyY => "y",
            KeyCode::KeyZ => "z",

            _ => return None,
        })
    }
}
