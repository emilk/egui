use super::ModifierNames;

/// State of the modifier keys. These must be fed to egui.
///
/// The best way to compare [`Modifiers`] is by using [`Modifiers::matches_logically`] or [`Modifiers::matches_exact`].
///
/// To access the [`Modifiers`] you can use the [`crate::Context::input`] function
///
/// ```rust
/// # let ctx = egui::Context::default();
/// let modifiers = ctx.input(|i| i.modifiers);
/// ```
///
/// NOTE: For cross-platform uses, ALT+SHIFT is a bad combination of modifiers
/// as on mac that is how you type special characters,
/// so those key presses are usually not reported to egui.
#[derive(Clone, Copy, Default, Hash, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Modifiers {
    /// Either of the alt keys are down (option ⌥ on Mac).
    pub alt: bool,

    /// Either of the control keys are down.
    /// When checking for keyboard shortcuts, consider using [`Self::command`] instead.
    pub ctrl: bool,

    /// Either of the shift keys are down.
    pub shift: bool,

    /// The Mac ⌘ Command key. Should always be set to `false` on other platforms.
    pub mac_cmd: bool,

    /// On Windows and Linux, set this to the same value as `ctrl`.
    /// On Mac, this should be set whenever one of the ⌘ Command keys are down (same as `mac_cmd`).
    /// This is so that egui can, for instance, select all text by checking for `command + A`
    /// and it will work on both Mac and Windows.
    pub command: bool,
}

impl std::fmt::Debug for Modifiers {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_none() {
            return write!(f, "Modifiers::NONE");
        }

        let Self {
            alt,
            ctrl,
            shift,
            mac_cmd,
            command,
        } = *self;

        let mut debug = f.debug_struct("Modifiers");
        if alt {
            debug.field("alt", &true);
        }
        if ctrl {
            debug.field("ctrl", &true);
        }
        if shift {
            debug.field("shift", &true);
        }
        if mac_cmd {
            debug.field("mac_cmd", &true);
        }
        if command {
            debug.field("command", &true);
        }
        debug.finish()
    }
}

impl Modifiers {
    pub const NONE: Self = Self {
        alt: false,
        ctrl: false,
        shift: false,
        mac_cmd: false,
        command: false,
    };

    pub const ALT: Self = Self {
        alt: true,
        ctrl: false,
        shift: false,
        mac_cmd: false,
        command: false,
    };
    pub const CTRL: Self = Self {
        alt: false,
        ctrl: true,
        shift: false,
        mac_cmd: false,
        command: false,
    };
    pub const SHIFT: Self = Self {
        alt: false,
        ctrl: false,
        shift: true,
        mac_cmd: false,
        command: false,
    };

    /// The Mac ⌘ Command key
    pub const MAC_CMD: Self = Self {
        alt: false,
        ctrl: false,
        shift: false,
        mac_cmd: true,
        command: false,
    };

    /// On Mac: ⌘ Command key, elsewhere: Ctrl key
    pub const COMMAND: Self = Self {
        alt: false,
        ctrl: false,
        shift: false,
        mac_cmd: false,
        command: true,
    };

    /// ```
    /// # use egui::Modifiers;
    /// assert_eq!(
    ///     Modifiers::CTRL | Modifiers::ALT,
    ///     Modifiers { ctrl: true, alt: true, ..Default::default() }
    /// );
    /// assert_eq!(
    ///     Modifiers::ALT.plus(Modifiers::CTRL),
    ///     Modifiers::CTRL.plus(Modifiers::ALT),
    /// );
    /// assert_eq!(
    ///     Modifiers::CTRL | Modifiers::ALT,
    ///     Modifiers::CTRL.plus(Modifiers::ALT),
    /// );
    /// ```
    #[inline]
    pub const fn plus(self, rhs: Self) -> Self {
        Self {
            alt: self.alt | rhs.alt,
            ctrl: self.ctrl | rhs.ctrl,
            shift: self.shift | rhs.shift,
            mac_cmd: self.mac_cmd | rhs.mac_cmd,
            command: self.command | rhs.command,
        }
    }

    #[inline]
    pub fn is_none(&self) -> bool {
        self == &Self::default()
    }

    #[inline]
    pub fn any(&self) -> bool {
        !self.is_none()
    }

    #[inline]
    pub fn all(&self) -> bool {
        self.alt && self.ctrl && self.shift && self.command
    }

    /// Is shift the only pressed button?
    #[inline]
    pub fn shift_only(&self) -> bool {
        self.shift && !(self.alt || self.command)
    }

    /// true if only [`Self::ctrl`] or only [`Self::mac_cmd`] is pressed.
    #[inline]
    pub fn command_only(&self) -> bool {
        !self.alt && !self.shift && self.command
    }

    /// Checks that the `ctrl/cmd` matches, and that the `shift/alt` of the argument is a subset
    /// of the pressed key (`self`).
    ///
    /// This means that if the pattern has not set `shift`, then `self` can have `shift` set or not.
    ///
    /// The reason is that many logical keys require `shift` or `alt` on some keyboard layouts.
    /// For instance, in order to press `+` on an English keyboard, you need to press `shift` and `=`,
    /// but a Swedish keyboard has dedicated `+` key.
    /// So if you want to make a [`KeyboardShortcut`](crate::KeyboardShortcut) looking for `Cmd` + `+`, it makes sense
    /// to ignore the shift key.
    /// Similarly, the `Alt` key is sometimes used to type special characters.
    ///
    /// However, if the pattern (the argument) explicitly requires the `shift` or `alt` keys
    /// to be pressed, then they must be pressed.
    ///
    /// # Example:
    /// ```
    /// # use egui::Modifiers;
    /// # let pressed_modifiers = Modifiers::default();
    /// if pressed_modifiers.matches_logically(Modifiers::ALT | Modifiers::SHIFT) {
    ///     // Alt and Shift are pressed, but not ctrl/command
    /// }
    /// ```
    ///
    /// ## Behavior:
    /// ```
    /// # use egui::Modifiers;
    /// assert!(Modifiers::CTRL.matches_logically(Modifiers::CTRL));
    /// assert!(!Modifiers::CTRL.matches_logically(Modifiers::CTRL | Modifiers::SHIFT));
    /// assert!((Modifiers::CTRL | Modifiers::SHIFT).matches_logically(Modifiers::CTRL));
    /// assert!((Modifiers::CTRL | Modifiers::COMMAND).matches_logically(Modifiers::CTRL));
    /// assert!((Modifiers::CTRL | Modifiers::COMMAND).matches_logically(Modifiers::COMMAND));
    /// assert!((Modifiers::MAC_CMD | Modifiers::COMMAND).matches_logically(Modifiers::COMMAND));
    /// assert!(!Modifiers::COMMAND.matches_logically(Modifiers::MAC_CMD));
    /// ```
    pub fn matches_logically(&self, pattern: Self) -> bool {
        if pattern.alt && !self.alt {
            return false;
        }
        if pattern.shift && !self.shift {
            return false;
        }

        self.cmd_ctrl_matches(pattern)
    }

    /// Check for equality but with proper handling of [`Self::command`].
    ///
    /// `self` here are the currently pressed modifiers,
    /// and the argument the pattern we are testing for.
    ///
    /// Note that this will require the `shift` and `alt` keys match, even though
    /// these modifiers are sometimes required to produce some logical keys.
    /// For instance, to press `+` on an English keyboard, you need to press `shift` and `=`,
    /// but on a Swedish keyboard you can press the dedicated `+` key.
    /// Therefore, you often want to use [`Self::matches_logically`] instead.
    ///
    /// # Example:
    /// ```
    /// # use egui::Modifiers;
    /// # let pressed_modifiers = Modifiers::default();
    /// if pressed_modifiers.matches_exact(Modifiers::ALT | Modifiers::SHIFT) {
    ///     // Alt and Shift are pressed, and nothing else
    /// }
    /// ```
    ///
    /// ## Behavior:
    /// ```
    /// # use egui::Modifiers;
    /// assert!(Modifiers::CTRL.matches_exact(Modifiers::CTRL));
    /// assert!(!Modifiers::CTRL.matches_exact(Modifiers::CTRL | Modifiers::SHIFT));
    /// assert!(!(Modifiers::CTRL | Modifiers::SHIFT).matches_exact(Modifiers::CTRL));
    /// assert!((Modifiers::CTRL | Modifiers::COMMAND).matches_exact(Modifiers::CTRL));
    /// assert!((Modifiers::CTRL | Modifiers::COMMAND).matches_exact(Modifiers::COMMAND));
    /// assert!((Modifiers::MAC_CMD | Modifiers::COMMAND).matches_exact(Modifiers::COMMAND));
    /// assert!(!Modifiers::COMMAND.matches_exact(Modifiers::MAC_CMD));
    /// ```
    pub fn matches_exact(&self, pattern: Self) -> bool {
        // alt and shift must always match the pattern:
        if pattern.alt != self.alt || pattern.shift != self.shift {
            return false;
        }

        self.cmd_ctrl_matches(pattern)
    }

    /// Check if any of the modifiers match exactly.
    ///
    /// Returns true if the same modifier is pressed in `self` as in `pattern`,
    /// for at least one modifier.
    ///
    /// ## Behavior:
    /// ```
    /// # use egui::Modifiers;
    /// assert!(Modifiers::CTRL.matches_any(Modifiers::CTRL));
    /// assert!(Modifiers::CTRL.matches_any(Modifiers::CTRL | Modifiers::SHIFT));
    /// assert!((Modifiers::CTRL | Modifiers::SHIFT).matches_any(Modifiers::CTRL));
    /// ```
    pub fn matches_any(&self, pattern: Self) -> bool {
        if self.alt && pattern.alt {
            return true;
        }
        if self.shift && pattern.shift {
            return true;
        }
        if self.ctrl && pattern.ctrl {
            return true;
        }
        if self.mac_cmd && pattern.mac_cmd {
            return true;
        }
        if (self.mac_cmd || self.command || self.ctrl) && pattern.command {
            return true;
        }
        false
    }

    /// Checks only cmd/ctrl, not alt/shift.
    ///
    /// `self` here are the currently pressed modifiers,
    /// and the argument the pattern we are testing for.
    ///
    /// This takes care to properly handle the difference between
    /// [`Self::ctrl`], [`Self::command`] and [`Self::mac_cmd`].
    pub fn cmd_ctrl_matches(&self, pattern: Self) -> bool {
        if pattern.mac_cmd {
            // Mac-specific match:
            if !self.mac_cmd {
                return false;
            }
            if pattern.ctrl != self.ctrl {
                return false;
            }
            return true;
        }

        if !pattern.ctrl && !pattern.command {
            // the pattern explicitly doesn't want any ctrl/command:
            return !self.ctrl && !self.command;
        }

        // if the pattern is looking for command, then `ctrl` may or may not be set depending on platform.
        // if the pattern is looking for `ctrl`, then `command` may or may not be set depending on platform.

        if pattern.ctrl && !self.ctrl {
            return false;
        }
        if pattern.command && !self.command {
            return false;
        }

        true
    }

    /// Whether another set of modifiers is contained in this set of modifiers with proper handling of [`Self::command`].
    ///
    /// ```
    /// # use egui::Modifiers;
    /// assert!(Modifiers::default().contains(Modifiers::default()));
    /// assert!(Modifiers::CTRL.contains(Modifiers::default()));
    /// assert!(Modifiers::CTRL.contains(Modifiers::CTRL));
    /// assert!(Modifiers::CTRL.contains(Modifiers::COMMAND));
    /// assert!(Modifiers::MAC_CMD.contains(Modifiers::COMMAND));
    /// assert!(Modifiers::COMMAND.contains(Modifiers::MAC_CMD));
    /// assert!(Modifiers::COMMAND.contains(Modifiers::CTRL));
    /// assert!(!(Modifiers::ALT | Modifiers::CTRL).contains(Modifiers::SHIFT));
    /// assert!((Modifiers::CTRL | Modifiers::SHIFT).contains(Modifiers::CTRL));
    /// assert!(!Modifiers::CTRL.contains(Modifiers::CTRL | Modifiers::SHIFT));
    /// ```
    pub fn contains(&self, query: Self) -> bool {
        if query == Self::default() {
            return true;
        }

        let Self {
            alt,
            ctrl,
            shift,
            mac_cmd,
            command,
        } = *self;

        if alt && query.alt {
            return self.contains(Self {
                alt: false,
                ..query
            });
        }
        if shift && query.shift {
            return self.contains(Self {
                shift: false,
                ..query
            });
        }

        if (ctrl || command) && (query.ctrl || query.command) {
            return self.contains(Self {
                command: false,
                ctrl: false,
                ..query
            });
        }
        if (mac_cmd || command) && (query.mac_cmd || query.command) {
            return self.contains(Self {
                mac_cmd: false,
                command: false,
                ..query
            });
        }

        false
    }
}

impl std::ops::BitOr for Modifiers {
    type Output = Self;

    #[inline]
    fn bitor(self, rhs: Self) -> Self {
        self.plus(rhs)
    }
}

impl std::ops::BitOrAssign for Modifiers {
    #[inline]
    fn bitor_assign(&mut self, rhs: Self) {
        *self = *self | rhs;
    }
}

impl Modifiers {
    pub fn ui(&self, ui: &mut crate::Ui) {
        ui.label(ModifierNames::NAMES.format(self, ui.ctx().os().is_mac()));
    }
}
