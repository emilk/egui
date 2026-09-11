/// Mouse button (or similar for touch input)
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum PointerButton {
    /// The primary mouse button is usually the left one.
    Primary = 0,

    /// The secondary mouse button is usually the right one,
    /// and most often used for context menus or other optional things.
    Secondary = 1,

    /// The tertiary mouse button is usually the middle mouse button (e.g. clicking the scroll wheel).
    Middle = 2,

    /// The first extra mouse button on some mice. In web typically corresponds to the Browser back button.
    Extra1 = 3,

    /// The second extra mouse button on some mice. In web typically corresponds to the Browser forward button.
    Extra2 = 4,
}

/// Number of pointer buttons supported by egui, i.e. the number of possible states of [`PointerButton`].
pub const NUM_POINTER_BUTTONS: usize = 5;
