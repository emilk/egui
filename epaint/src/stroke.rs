use super::*;

/// Describes the width and color of a line.
///
/// The default stroke is the same as [`Stroke::none`].
#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
pub struct Stroke {
    pub width: f32,
    pub color: Color32,
}

impl Stroke {
    /// Same as [`Stroke::default`].
    #[inline(always)]
    pub fn none() -> Self {
        Self::new(0.0, Color32::TRANSPARENT)
    }

    #[inline]
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
    #[inline(always)]
    fn from((width, color): (f32, Color)) -> Stroke {
        Stroke::new(width, color)
    }
}

impl std::hash::Hash for Stroke {
    #[inline(always)]
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        let Self { width, color } = *self;
        crate::f32_hash(state, width);
        color.hash(state);
    }
}

impl PartialEq for Stroke {
    #[inline(always)]
    fn eq(&self, other: &Self) -> bool {
        self.color == other.color && crate::f32_eq(self.width, other.width)
    }
}

impl std::cmp::Eq for Stroke {}
