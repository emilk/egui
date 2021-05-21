use super::*;

/// Describes the width and color of a line.
///
/// The default stroke is the same as [`Stroke::none`].
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
pub struct Stroke {
    pub width: f32,
    pub color: Color32,
}

impl Stroke {
    /// Same as [`Stroke::default`].
    pub fn none() -> Self {
        Self::new(0.0, Color32::TRANSPARENT)
    }

    pub fn new(width: impl Into<f32>, color: impl Into<Color32>) -> Self {
        Self {
            width: width.into(),
            color: color.into(),
        }
    }
}

impl<Color> From<(f32, Color)> for Stroke
where
    Color: Into<Color32>,
{
    fn from((width, color): (f32, Color)) -> Stroke {
        Stroke::new(width, color)
    }
}
