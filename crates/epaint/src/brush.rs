use crate::{Rect, TextureId};

/// Controls texturing of a [`crate::RectShape`].
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Brush {
    /// If the rect should be filled with a texture, which one?
    ///
    /// The texture is multiplied with [`crate::RectShape::fill`].
    pub fill_texture_id: TextureId,

    /// What UV coordinates to use for the texture?
    ///
    /// To display a texture, set [`Self::fill_texture_id`],
    /// and set this to `Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0))`.
    ///
    /// Use [`Rect::ZERO`] to turn off texturing.
    pub uv: Rect,
}
