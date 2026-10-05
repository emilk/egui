/// What is driving an [`crate::Event::MouseWheel`] event.
///
/// This lets a [`crate::ScrollArea`] treat the different kinds of scrolling differently,
/// e.g. rubber-band past the edge while the fingers are on the trackpad,
/// but bounce during the system-driven momentum phase.
///
/// This is about mice and trackpads. Touch screens don't produce [`crate::Event::MouseWheel`]
/// events at all; they scroll by dragging with the pointer (see [`crate::Event::Touch`]).
///
/// ## Platform-specific
/// * **macOS**: [`Self::Wheel`], [`Self::Trackpad`] or [`Self::Momentum`].
/// * **Wayland, Windows, X11, web**: always [`Self::Unknown`], until winit reports the source.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum MouseWheelSource {
    /// The platform doesn't say.
    Unknown,

    /// A physical wheel, usually on a mouse.
    Wheel,

    /// Fingers moving on a trackpad (or similar touch surface, like a Magic Mouse).
    ///
    /// Not a touch screen: those scroll by dragging with the pointer, see [`crate::Event::Touch`].
    ///
    /// On platforms that report a [`crate::TouchPhase`], the fingers are on the trackpad
    /// from [`crate::TouchPhase::Start`] until [`crate::TouchPhase::End`].
    Trackpad,

    /// The system is continuing a finger scroll after the fingers were lifted,
    /// with decaying speed, such as the "momentum phase" on macOS.
    ///
    /// On platforms that report a [`crate::TouchPhase`], this gets its own
    /// [`crate::TouchPhase::Start`] … [`crate::TouchPhase::End`] cycle after the finger one.
    Momentum,
}
