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

impl TryFrom<PhysicalKey> for crate::Key {
    type Error = ();

    fn try_from(value: PhysicalKey) -> Result<Self, Self::Error> {
        match value {
            PhysicalKey::Code(code) => code.try_into(),
            PhysicalKey::Unidentified(_) => Err(()),
        }
    }
}

impl TryFrom<KeyCode> for crate::Key {
    type Error = ();

    fn try_from(value: KeyCode) -> Result<Self, Self::Error> {
        Ok(match value {
            // Commands:
            KeyCode::ArrowDown => Self::ArrowDown,
            KeyCode::ArrowLeft => Self::ArrowLeft,
            KeyCode::ArrowRight => Self::ArrowRight,
            KeyCode::ArrowUp => Self::ArrowUp,

            KeyCode::Escape => Self::Escape,
            KeyCode::Tab => Self::Tab,
            KeyCode::Backspace | KeyCode::NumpadBackspace => Self::Backspace,
            KeyCode::Enter | KeyCode::NumpadEnter => Self::Enter,
            KeyCode::Space => Self::Space,

            KeyCode::Insert => Self::Insert,
            KeyCode::Delete => Self::Delete,
            KeyCode::Home => Self::Home,
            KeyCode::End => Self::End,
            KeyCode::PageUp => Self::PageUp,
            KeyCode::PageDown => Self::PageDown,

            KeyCode::Copy => Self::Copy,
            KeyCode::Cut => Self::Cut,
            KeyCode::Paste => Self::Paste,

            // Punctuation.
            // `Colon`, `Pipe`, `Questionmark`, `Exclamationmark`, `OpenCurlyBracket` and
            // `CloseCurlyBracket` have no dedicated physical key; they are only produced
            // as logical keys (e.g. via Shift), so no `KeyCode` maps to them.
            KeyCode::Comma | KeyCode::NumpadComma => Self::Comma,
            KeyCode::Backslash => Self::Backslash,
            KeyCode::Slash | KeyCode::NumpadDivide => Self::Slash,
            KeyCode::BracketLeft => Self::OpenBracket,
            KeyCode::BracketRight => Self::CloseBracket,
            KeyCode::Backquote => Self::Backtick,
            KeyCode::Minus | KeyCode::NumpadSubtract => Self::Minus,
            KeyCode::Period => Self::Period,
            KeyCode::NumpadAdd => Self::Plus,
            KeyCode::Equal | KeyCode::NumpadEqual => Self::Equals,
            KeyCode::Semicolon => Self::Semicolon,
            KeyCode::Quote => Self::Quote,

            // Digits:
            KeyCode::Digit0 | KeyCode::Numpad0 => Self::Num0,
            KeyCode::Digit1 | KeyCode::Numpad1 => Self::Num1,
            KeyCode::Digit2 | KeyCode::Numpad2 => Self::Num2,
            KeyCode::Digit3 | KeyCode::Numpad3 => Self::Num3,
            KeyCode::Digit4 | KeyCode::Numpad4 => Self::Num4,
            KeyCode::Digit5 | KeyCode::Numpad5 => Self::Num5,
            KeyCode::Digit6 | KeyCode::Numpad6 => Self::Num6,
            KeyCode::Digit7 | KeyCode::Numpad7 => Self::Num7,
            KeyCode::Digit8 | KeyCode::Numpad8 => Self::Num8,
            KeyCode::Digit9 | KeyCode::Numpad9 => Self::Num9,

            // Letters:
            KeyCode::KeyA => Self::A,
            KeyCode::KeyB => Self::B,
            KeyCode::KeyC => Self::C,
            KeyCode::KeyD => Self::D,
            KeyCode::KeyE => Self::E,
            KeyCode::KeyF => Self::F,
            KeyCode::KeyG => Self::G,
            KeyCode::KeyH => Self::H,
            KeyCode::KeyI => Self::I,
            KeyCode::KeyJ => Self::J,
            KeyCode::KeyK => Self::K,
            KeyCode::KeyL => Self::L,
            KeyCode::KeyM => Self::M,
            KeyCode::KeyN => Self::N,
            KeyCode::KeyO => Self::O,
            KeyCode::KeyP => Self::P,
            KeyCode::KeyQ => Self::Q,
            KeyCode::KeyR => Self::R,
            KeyCode::KeyS => Self::S,
            KeyCode::KeyT => Self::T,
            KeyCode::KeyU => Self::U,
            KeyCode::KeyV => Self::V,
            KeyCode::KeyW => Self::W,
            KeyCode::KeyX => Self::X,
            KeyCode::KeyY => Self::Y,
            KeyCode::KeyZ => Self::Z,

            // Function keys:
            KeyCode::F1 => Self::F1,
            KeyCode::F2 => Self::F2,
            KeyCode::F3 => Self::F3,
            KeyCode::F4 => Self::F4,
            KeyCode::F5 => Self::F5,
            KeyCode::F6 => Self::F6,
            KeyCode::F7 => Self::F7,
            KeyCode::F8 => Self::F8,
            KeyCode::F9 => Self::F9,
            KeyCode::F10 => Self::F10,
            KeyCode::F11 => Self::F11,
            KeyCode::F12 => Self::F12,
            KeyCode::F13 => Self::F13,
            KeyCode::F14 => Self::F14,
            KeyCode::F15 => Self::F15,
            KeyCode::F16 => Self::F16,
            KeyCode::F17 => Self::F17,
            KeyCode::F18 => Self::F18,
            KeyCode::F19 => Self::F19,
            KeyCode::F20 => Self::F20,
            KeyCode::F21 => Self::F21,
            KeyCode::F22 => Self::F22,
            KeyCode::F23 => Self::F23,
            KeyCode::F24 => Self::F24,
            KeyCode::F25 => Self::F25,
            KeyCode::F26 => Self::F26,
            KeyCode::F27 => Self::F27,
            KeyCode::F28 => Self::F28,
            KeyCode::F29 => Self::F29,
            KeyCode::F30 => Self::F30,
            KeyCode::F31 => Self::F31,
            KeyCode::F32 => Self::F32,
            KeyCode::F33 => Self::F33,
            KeyCode::F34 => Self::F34,
            KeyCode::F35 => Self::F35,

            // Browser / multimedia navigation keys:
            KeyCode::BrowserBack => Self::BrowserBack,
            KeyCode::BrowserForward => Self::BrowserForward,
            KeyCode::BrowserRefresh => Self::BrowserRefresh,
            KeyCode::BrowserSearch => Self::BrowserSearch,
            KeyCode::BrowserHome => Self::BrowserHome,
            KeyCode::BrowserFavorites => Self::BrowserFavorites,
            KeyCode::BrowserStop => Self::BrowserStop,

            // Media control keys:
            KeyCode::MediaPlayPause => Self::MediaPlayPause,
            KeyCode::MediaTrackNext => Self::MediaTrackNext,
            KeyCode::MediaTrackPrevious => Self::MediaTrackPrevious,
            KeyCode::MediaStop => Self::MediaStop,
            KeyCode::AudioVolumeMute => Self::AudioVolumeMute,
            KeyCode::AudioVolumeDown => Self::AudioVolumeDown,
            KeyCode::AudioVolumeUp => Self::AudioVolumeUp,

            // Launch keys:
            KeyCode::LaunchMail => Self::LaunchMail,
            KeyCode::LaunchApp1 => Self::LaunchApp1,
            KeyCode::LaunchApp2 => Self::LaunchApp2,

            // Modifier keys:
            KeyCode::ShiftLeft => Self::ShiftLeft,
            KeyCode::ShiftRight => Self::ShiftRight,
            KeyCode::ControlLeft => Self::ControlLeft,
            KeyCode::ControlRight => Self::ControlRight,
            KeyCode::AltLeft => Self::AltLeft,
            KeyCode::AltRight => Self::AltRight,
            KeyCode::MetaLeft => Self::SuperLeft,
            KeyCode::MetaRight => Self::SuperRight,

            // International keys:
            KeyCode::IntlBackslash => Self::IntlBackslash,

            // Lock / System keys:
            KeyCode::CapsLock => Self::CapsLock,
            KeyCode::NumLock => Self::NumLock,
            KeyCode::ScrollLock => Self::ScrollLock,
            KeyCode::PrintScreen => Self::PrintScreen,
            KeyCode::Pause => Self::Pause,
            KeyCode::ContextMenu => Self::Menu,

            // Mac / other system keys:
            KeyCode::Fn => Self::Fn,
            KeyCode::Eject => Self::Eject,
            KeyCode::Help => Self::Help,
            KeyCode::Power => Self::Power,
            KeyCode::Sleep => Self::Sleep,
            KeyCode::NumpadClear => Self::Clear,

            _ => return Err(()),
        })
    }
}
