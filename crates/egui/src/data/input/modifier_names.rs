use super::Modifiers;

/// Names of different modifier keys.
///
/// Used to name modifiers.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ModifierNames<'a> {
    pub is_short: bool,

    pub alt: &'a str,
    pub ctrl: &'a str,
    pub shift: &'a str,
    pub mac_cmd: &'a str,
    pub mac_alt: &'a str,

    /// What goes between the names
    pub concat: &'a str,
}

impl ModifierNames<'static> {
    /// ⌥ ⌃ ⇧ ⌘ - NOTE: not supported by the default egui font.
    pub const SYMBOLS: Self = Self {
        is_short: true,
        alt: "⌥",
        ctrl: "⌃",
        shift: "⇧",
        mac_cmd: "⌘",
        mac_alt: "⌥",
        concat: "",
    };

    /// Alt, Ctrl, Shift, Cmd
    pub const NAMES: Self = Self {
        is_short: false,
        alt: "Alt",
        ctrl: "Ctrl",
        shift: "Shift",
        mac_cmd: "Cmd",
        mac_alt: "Option",
        concat: "+",
    };
}

impl ModifierNames<'_> {
    pub fn format(&self, modifiers: &Modifiers, is_mac: bool) -> String {
        let mut s = String::new();

        let mut append_if = |modifier_is_active, modifier_name| {
            if modifier_is_active {
                if !s.is_empty() {
                    s += self.concat;
                }
                s += modifier_name;
            }
        };

        if is_mac {
            append_if(modifiers.ctrl, self.ctrl);
            append_if(modifiers.shift, self.shift);
            append_if(modifiers.alt, self.mac_alt);
            append_if(modifiers.mac_cmd || modifiers.command, self.mac_cmd);
        } else {
            append_if(modifiers.ctrl || modifiers.command, self.ctrl);
            append_if(modifiers.alt, self.alt);
            append_if(modifiers.shift, self.shift);
        }

        s
    }
}
