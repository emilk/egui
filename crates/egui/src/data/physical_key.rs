pub use keyboard_types::Code as KeyCode;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum PhysicalKey {
    /// A known key code
    Code(KeyCode),

    /// This variant is used when the key cannot be translated to a [`KeyCode`]
    ///
    /// The native keycode is provided (if available) so you're able to more reliably match
    /// key-press and key-release events by hashing the [`PhysicalKey`]. It is also possible to use
    /// this for keybinds for non-standard keys, but such keybinds are tied to a given platform.
    Unidentified(u32),
}
