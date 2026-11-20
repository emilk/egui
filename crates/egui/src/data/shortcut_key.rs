use core::ops::{Deref, DerefMut};

use smol_str::StrExt as _;

use crate::{Key, NamedKey, PhysicalKey, data::FullKey};

/// Key wrapper type, representing a key that is considered safe to use to check for keyboard shortcuts. It is always lowercase.
///
/// Typically obtained by calling [`FullKey::shortcut_key`] or [`Key::shortcut_key`], which uses the logical key when it's
/// ASCII/named, and the physical key's US QWERTY equivalent otherwise.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[repr(transparent)]
pub struct ShortcutKey(Key);

impl ShortcutKey {
    /// Naively creates a new shortcut-safe key. Lowercase's any [`Key::Character`]'s.
    ///
    /// Unless you want custom logical/physical key -> shortcut key mapping, you should use  [`FullKey::shortcut_key`] or [`Key::shortcut_key`].
    pub fn new(key: Key) -> Self {
        Self(match key {
            Key::Character(c) => Key::Character(c.to_lowercase_smolstr()),
            named @ Key::Named(_) => named,
        })
    }

    pub const fn simple(key: &'static str) -> Self {
        assert!(
            key.is_ascii(),
            "simple shortcut keys must be lowercase ASCII"
        );
        let bytes = key.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            let c = char::from_u32(bytes[i] as _)
                .expect("simple shortcut keys must be lowercase ASCII");
            assert!(
                !c.is_ascii_uppercase(),
                "simple shortcut keys must be lowercase ASCII"
            );
            i += 1;
        }
        Self(Key::character(key))
    }
}

pub trait ShortcutKeyExt {
    /// The key to use when matching keyboard shortcuts.
    ///
    /// See [`Key::shortcut_key`] for more detail.
    fn shortcut_key(&self) -> ShortcutKey;
}

impl ShortcutKeyExt for NamedKey {
    fn shortcut_key(&self) -> ShortcutKey {
        ShortcutKey(Key::Named(*self))
    }
}

impl FullKey {
    /// The key to use when matching keyboard shortcuts.
    ///
    /// See [`Key::shortcut_key`] for more detail.
    pub fn shortcut_key(&self) -> ShortcutKey {
        self.logical.shortcut_key(self.physical)
    }
}

impl Key {
    /// The key to use when matching keyboard shortcuts.
    ///
    /// Named keys and ASCII characters use the logical key (`self`),
    /// so shortcuts follow the user's layout (e.g. `Cmd+Z` on AZERTY or Dvorak).
    ///
    /// Non-ASCII characters (e.g. `"я"` on a Russian layout) use the character the physical key
    /// produces on a US layout instead (see [`PhysicalKey::us_layout_character`]),
    /// so that `Ctrl+Z` works on non-Latin layouts too.
    /// If the physical key is unknown, the logical key is used.
    ///
    /// ```
    /// # use egui::{Key, KeyCode, PhysicalKey, ShortcutKey};
    /// let physical_key = Some(PhysicalKey::Code(KeyCode::KeyZ));
    /// assert_eq!(Key::character("я").shortcut_key(physical_key), ShortcutKey::simple("z"));
    /// assert_eq!(Key::character("Z").shortcut_key(physical_key), ShortcutKey::simple("z"));
    ///
    /// // AZERTY: the physical `KeyW` key types "z", so this is `Cmd+Z`.
    /// let physical_key = Some(PhysicalKey::Code(KeyCode::KeyW));
    /// assert_eq!(Key::character("z").shortcut_key(physical_key), ShortcutKey::simple("z"));
    /// ```
    pub fn shortcut_key(&self, physical_key: Option<PhysicalKey>) -> ShortcutKey {
        match self {
            Self::Character(character)
                if !character.is_ascii()
                    && let Some(us_character) =
                        physical_key.and_then(PhysicalKey::us_layout_character) =>
            {
                // Safe to directly construct, since us_layout_character always returns lowercase.
                ShortcutKey(Self::character(us_character))
            }
            key => ShortcutKey::new(key.clone()),
        }
    }
}

impl DerefMut for ShortcutKey {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Deref for ShortcutKey {
    type Target = Key;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
