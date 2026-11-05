use crate::Key;

use super::{ModifierNames, Modifiers};

/// A keyboard shortcut, e.g. `Ctrl+Alt+W`.
///
/// Can be used with [`crate::InputState::consume_shortcut`]
/// and [`crate::Context::format_shortcut`].
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct KeyboardShortcut {
    pub modifiers: Modifiers,

    pub logical_key: Key,
}

impl KeyboardShortcut {
    pub const fn new(modifiers: Modifiers, logical_key: Key) -> Self {
        Self {
            modifiers,
            logical_key,
        }
    }

    /// Format the shortcut, e.g. `"Ctrl+Shift+F"`.
    ///
    /// The result is wrapped in U+2066 LEFT-TO-RIGHT ISOLATE … U+2069 POP
    /// DIRECTIONAL ISOLATE, so it stays one left-to-right run when embedded in
    /// right-to-left text (the isolates render as invisible zero-advance
    /// glyphs, so left-to-right output is unchanged).
    pub fn format(&self, names: &ModifierNames<'_>, is_mac: bool) -> String {
        self.format_for(names, is_mac, crate::emath::ReadingDirection::Ltr)
    }

    /// Format the shortcut like [`Self::format`], mirroring the display names
    /// of the horizontal arrow keys to the reading direction.
    ///
    /// In a right-to-left interface the arrows follow the reading direction
    /// (the same convention as the menu and panel mirroring), so a shortcut
    /// for "toward the reading start" is labeled `Right`. See
    /// [`Key::name_for`] and [`Key::symbol_or_name_for`].
    pub fn format_for(
        &self,
        names: &ModifierNames<'_>,
        is_mac: bool,
        dir: crate::emath::ReadingDirection,
    ) -> String {
        let mut s = names.format(&self.modifiers, is_mac);
        if !s.is_empty() {
            s += names.concat;
        }
        if names.is_short {
            s += self.logical_key.symbol_or_name_for(dir);
        } else {
            s += self.logical_key.name_for(dir);
        }
        // Keep the shortcut one left-to-right run when embedded in
        // right-to-left text: without the isolation, a trailing neutral glyph
        // (a separator, or a symbol-form key like `⌃⇧−`) resolves to the
        // surrounding paragraph direction (UAX #9 N1/N2) and lands reversed
        // on the wrong side of the run.
        format!("\u{2066}{s}\u{2069}")
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
    /// # use egui::{Key, KeyboardShortcut, Modifiers};
    /// assert!(KeyboardShortcut::new(Modifiers::NONE, Key::Space).conflicts_with_text_editing());
    /// assert!(KeyboardShortcut::new(Modifiers::ALT, Key::ArrowLeft).conflicts_with_text_editing());
    /// assert!(!KeyboardShortcut::new(Modifiers::COMMAND, Key::S).conflicts_with_text_editing());
    /// ```
    pub fn conflicts_with_text_editing(&self) -> bool {
        self.modifiers.is_none()
            || matches!(
                self.logical_key,
                Key::Space
                    | Key::ArrowLeft
                    | Key::ArrowRight
                    | Key::ArrowUp
                    | Key::ArrowDown
                    | Key::Home
                    | Key::End
            )
    }
}

#[test]
fn format_kb_shortcut() {
    let cmd_shift_f = KeyboardShortcut::new(Modifiers::COMMAND | Modifiers::SHIFT, Key::F);
    assert_eq!(
        cmd_shift_f.format(&ModifierNames::NAMES, false),
        "\u{2066}Ctrl+Shift+F\u{2069}"
    );
    assert_eq!(
        cmd_shift_f.format(&ModifierNames::NAMES, true),
        "\u{2066}Shift+Cmd+F\u{2069}"
    );
    assert_eq!(
        cmd_shift_f.format(&ModifierNames::SYMBOLS, false),
        "\u{2066}⌃⇧F\u{2069}"
    );
    assert_eq!(
        cmd_shift_f.format(&ModifierNames::SYMBOLS, true),
        "\u{2066}⇧⌘F\u{2069}"
    );
}
