use std::{fmt::Debug, sync::Arc};

use ecolor::Color32;
use emath::{Pos2, Rect};

/// How paths will be colored.
#[derive(Clone)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum ColorMode {
    /// The entire path is one solid color, this is the default.
    Solid(Color32),

    /// Provide a callback which takes in the path's bounding box and a position and converts it to a color.
    /// When used with a path, the bounding box will have a margin of [`TessellationOptions::feathering_size_in_pixels`](`crate::tessellator::TessellationOptions::feathering_size_in_pixels`)
    ///
    /// **This cannot be serialized**
    #[cfg_attr(feature = "serde", serde(skip))]
    UV(Arc<dyn Fn(Rect, Pos2) -> Color32 + Send + Sync>),
}

impl Default for ColorMode {
    fn default() -> Self {
        Self::Solid(Color32::TRANSPARENT)
    }
}

impl Debug for ColorMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Solid(arg0) => f.debug_tuple("Solid").field(arg0).finish(),
            Self::UV(_arg0) => f.debug_tuple("UV").field(&"<closure>").finish(),
        }
    }
}

impl PartialEq for ColorMode {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Solid(l0), Self::Solid(r0)) => l0 == r0,
            (Self::UV(_l0), Self::UV(_r0)) => false,
            _ => false,
        }
    }
}

impl ColorMode {
    pub const TRANSPARENT: Self = Self::Solid(Color32::TRANSPARENT);
}
