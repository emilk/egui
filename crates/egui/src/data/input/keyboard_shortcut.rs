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

    pub fn format(&self, names: &ModifierNames<'_>, is_mac: bool) -> String {
        let mut s = names.format(&self.modifiers, is_mac);
        if !s.is_empty() {
            s += names.concat;
        }
        if names.is_short {
            s += self.logical_key.symbol_or_name();
        } else {
            s += self.logical_key.name();
        }
        s
    }
}

#[test]
fn format_kb_shortcut() {
    let cmd_shift_f = KeyboardShortcut::new(Modifiers::COMMAND | Modifiers::SHIFT, Key::F);
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
