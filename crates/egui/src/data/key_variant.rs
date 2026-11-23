use crate::{FullKey, Key, NamedKey, PhysicalKey, ShortcutKey};

#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum KeyVariant {
    Logical(Key),
    Physical(PhysicalKey),
    Shortcut(ShortcutKey),
}

impl KeyVariant {
    pub fn equivalent(&self, key: &FullKey) -> bool {
        match self {
            Self::Logical(logical) => *logical == key.logical,
            Self::Physical(a) if let Some(b) = &key.physical => a == b,
            Self::Physical(_) => false,
            Self::Shortcut(shortcut) => *shortcut == key.shortcut_key(),
        }
    }
}

impl From<Key> for KeyVariant {
    fn from(value: Key) -> Self {
        Self::Logical(value)
    }
}

impl From<NamedKey> for KeyVariant {
    fn from(value: NamedKey) -> Self {
        Self::Logical(value.into())
    }
}

impl From<PhysicalKey> for KeyVariant {
    fn from(value: PhysicalKey) -> Self {
        Self::Physical(value)
    }
}

impl From<ShortcutKey> for KeyVariant {
    fn from(value: ShortcutKey) -> Self {
        Self::Shortcut(value)
    }
}
