/// The unit associated with the numeric value of a mouse wheel event
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum MouseWheelUnit {
    /// Number of ui points (logical pixels)
    Point,

    /// Number of lines
    Line,

    /// Number of pages
    Page,
}
