use crate::{Key, NamedKey, PhysicalKey};

/// A key's logical and, if known, physical identity.
///
/// Comparing a [`FullKey`] with a [`Key`] or [`NamedKey`] compares only the logical key.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct FullKey {
    /// The logical key, heeding the active keymap -- for instance, if the user has a Dvorak
    /// keyboard layout, it will be taken into account.
    ///
    /// Character keys may also depend on modifiers, depending on the platform and integration.
    /// See [`Key`] for details.
    ///
    /// [`NamedKey::Unidentified`] if the integration couldn't determine the logical key
    /// (but it may still know the [`Self::physical`] key).
    pub logical: Key,

    /// The physical key, corresponding to the actual position on the keyboard.
    ///
    /// This ignores keymaps, so it is generally not recommended for text or shortcuts.
    /// It can be useful for things like games, where e.g. the physical location of WASD on
    /// QWERTY should always map to movement, even if the user is using Dvorak or AZERTY.
    pub physical: Option<PhysicalKey>,
}

impl FullKey {
    #[inline]
    pub fn new(logical: Key, physical: Option<PhysicalKey>) -> Self {
        Self { logical, physical }
    }

    #[inline]
    pub fn with_physical(mut self, physical: PhysicalKey) -> Self {
        self.physical = Some(physical);
        self
    }

    /// Is anything known about this key, i.e. is either the logical key not
    /// [`NamedKey::Unidentified`], or the physical key known?
    ///
    /// Integrations should only send key events for identified keys.
    #[inline]
    pub fn is_identified(&self) -> bool {
        self.logical != NamedKey::Unidentified || self.physical.is_some()
    }

    /// Whether these keys refer to the same keyboard key.
    ///
    /// Compares physical keys if both are known; otherwise compares logical keys.
    /// egui uses this to pair releases with presses when the logical key changes in between,
    /// for example when Shift changes a number key into a symbol on the web.
    /// If a physical key is unavailable and the logical key changes, the pair cannot be matched.
    #[inline]
    pub fn is_same_key(&self, other: &Self) -> bool {
        match (self.physical, other.physical) {
            (Some(a), Some(b)) => a == b,
            _ => self.logical == other.logical,
        }
    }
}

impl PartialEq<Key> for FullKey {
    fn eq(&self, other: &Key) -> bool {
        self.logical == *other
    }
}

impl PartialEq<NamedKey> for FullKey {
    fn eq(&self, other: &NamedKey) -> bool {
        self.logical == *other
    }
}

impl From<Key> for FullKey {
    fn from(logical: Key) -> Self {
        Self {
            logical,
            physical: None,
        }
    }
}

impl From<NamedKey> for FullKey {
    fn from(logical: NamedKey) -> Self {
        Key::Named(logical).into()
    }
}
