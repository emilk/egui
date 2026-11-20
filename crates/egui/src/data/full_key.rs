use crate::{Key, NamedKey, PhysicalKey};

/// The full definition of a key - both logical & physical, if known.
///
/// ## Hash/Eq implementation
/// Two [`FullKey`]'s are considered identical if their logical keys match. Physical keys are never
/// considered.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct FullKey {
    /// Most of the time, it's the logical key, heeding the active keymap -- for instance, if the user has Dvorak
    /// keyboard layout, it will be taken into account.
    pub logical: Key,

    /// The physical key, corresponding to the actual position on the keyboard.
    ///
    /// This ignores keymaps, so it is not recommended to use this.
    /// The only thing it makes sense for is things like games,
    /// where e.g. the physical location of WSAD on QWERTY should always map to movement,
    /// even if the user is using Dvorak or AZERTY.
    ///
    // FIXME: does the below comment still apply?
    /// `eframe` does not (yet) implement this on web.
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
}

impl core::hash::Hash for FullKey {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.logical.hash(state);
    }
}

impl PartialEq for FullKey {
    fn eq(&self, other: &Self) -> bool {
        self.logical == other.logical
    }
}

impl Eq for FullKey {}

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
