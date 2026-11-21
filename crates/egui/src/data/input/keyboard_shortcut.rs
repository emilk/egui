use crate::{Key, NamedKey, ShortcutKey};

use super::{ModifierNames, Modifiers};

/// A keyboard shortcut, e.g. `Ctrl+Alt+W`.
///
/// Can be used with [`crate::InputState::consume_shortcut`]
/// and [`crate::Context::format_shortcut`].
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct KeyboardShortcut {
    pub modifiers: Modifiers,

    pub shortcut_key: ShortcutKey,
}

impl KeyboardShortcut {
    pub const fn new(modifiers: Modifiers, shortcut_key: ShortcutKey) -> Self {
        Self {
            modifiers,
            shortcut_key,
        }
    }

    pub fn format(&self, names: &ModifierNames<'_>, is_mac: bool) -> String {
        let mut s = names.format(&self.modifiers, is_mac);
        if !s.is_empty() {
            s += names.concat;
        }
        if names.is_short {
            s += self.shortcut_key.symbol_or_name();
        } else {
            s += self.shortcut_key.name();
        }
        s
    }

    /// Would consuming this shortcut interfere with editing text
    /// (e.g. moving the text cursor, or typing a space)?
    ///
    /// This is true for any shortcut without modifiers (e.g. `Space`, `A`, `Delete`),
    /// as well as for shortcuts using keys that are commonly combined with modifiers
    /// when editing text (e.g. `Alt+ArrowLeft` to move one word, or `Shift+Home` to select to the start of a line).
    ///
    /// Such shortcuts should only be consumed when no text field has keyboard focus.
    /// See also [`crate::Memory::focused`] and [`crate::Context::egui_wants_keyboard_input`].
    ///
    /// ```
    /// # use egui::{KeyboardShortcut, Modifiers, NamedKey, ShortcutKey, ShortcutKeyExt as _};
    /// assert!(KeyboardShortcut::new(Modifiers::NONE, ShortcutKey::simple(" ")).conflicts_with_text_editing());
    /// assert!(KeyboardShortcut::new(Modifiers::ALT, NamedKey::ArrowLeft.shortcut_key()).conflicts_with_text_editing());
    /// assert!(!KeyboardShortcut::new(Modifiers::COMMAND, ShortcutKey::simple("s")).conflicts_with_text_editing());
    /// ```
    pub fn conflicts_with_text_editing(&self) -> bool {
        self.modifiers.is_none()
            || *self.shortcut_key == Key::character(" ")
            || matches!(
                *self.shortcut_key,
                Key::Named(
                    NamedKey::ArrowLeft
                        | NamedKey::ArrowRight
                        | NamedKey::ArrowUp
                        | NamedKey::ArrowDown
                        | NamedKey::Home
                        | NamedKey::End
                )
            )
    }
}

#[test]
fn format_kb_shortcut() {
    let cmd_shift_f = KeyboardShortcut::new(
        Modifiers::COMMAND | Modifiers::SHIFT,
        ShortcutKey::simple("f"),
    );
    assert_eq!(
        cmd_shift_f.format(&ModifierNames::NAMES, false),
        "Ctrl+Shift+F"
    );
    assert_eq!(
        cmd_shift_f.format(&ModifierNames::NAMES, true),
        "Shift+Cmd+F"
    );
    assert_eq!(cmd_shift_f.format(&ModifierNames::SYMBOLS, false), "⌃⇧F");
    assert_eq!(cmd_shift_f.format(&ModifierNames::SYMBOLS, true), "⇧⌘F");
}
