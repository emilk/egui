use crate::{Key, NamedKey, PhysicalKey};

/// The full definition of a key - both logical & physical, if known.
///
/// Comparing a [`FullKey`] with a [`Key`] or [`NamedKey`] compares only the logical key.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct FullKey {
    /// Most of the time, it's the logical key, heeding the active keymap -- for instance, if the user has Dvorak
    /// keyboard layout, it will be taken into account.
    ///
    /// [`NamedKey::Unidentified`] if the integration couldn't determine the logical key
    /// (but it may still know the [`Self::physical`] key).
    pub logical: Key,

    /// The physical key, corresponding to the actual position on the keyboard.
    ///
    /// This ignores keymaps, so it is not recommended to use this.
    /// The only thing it makes sense for is things like games,
    /// where e.g. the physical location of WSAD on QWERTY should always map to movement,
    /// even if the user is using Dvorak or AZERTY.
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

    /// Do `self` and `other` refer to the same key on the keyboard?
    ///
    /// Compares the physical keys if both are known, otherwise the logical keys.
    /// Used to pair a key release with its press, even if the logical key changed in between
    /// (e.g. on web, pressing `a`, then `Shift`, then releasing `a` reports the release as `"A"`).
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
