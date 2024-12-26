/// We (sometimes) round sizes and coordinates to an even multiple of this value.
///
/// This is only used for rounding _logical UI points_, used for widget coordinates and sizes.
/// When rendering, you may want to round to an integer multiple of the physical _pixels_ instead,
/// using [`GuiRounding::round_to_pixels`].
///
/// See [`GuiRounding::round_point`] for more information.
///
/// This constant has to be a (negative) power of two so that it can be represented exactly
/// by a floating point number.
///
/// If we pick too large a value (e.g. 1 or 1/2), then we get judder during scrolling and animations.
/// If we pick too small a value (e.g. 1/4096), we run the risk of rounding errors again.
///
/// `f32` has 23 bits of mantissa, so if we use e.g. 1/8 as the rounding factor,
/// we can represent all numbers up to 2^20 exactly, which is plenty
/// (to my knowledge there are no displays that are a million pixels wide).
pub const GUI_ROUNDING: f32 = 1.0 / 32.0;

/// Trait for rounding coordinates and sizes to align with either .
///
/// See [`GuiRounding::round_point`] for more information.
pub trait GuiRounding {
    /// Rounds floating point numbers to an even multiple of the GUI rounding factor, [`crate::GUI_ROUNDING`].
    ///
    /// Use this for widget coordinates and sizes.
    ///
    /// Rounding sizes and positions prevent rounding errors when doing sizing calculations.
    /// We don't round to integers, because that would be too coarse (causing visible juddering when scrolling, for instance).
    /// Instead we round to an even multiple of [`GUI_ROUNDING`].
    fn round_point(self) -> Self;

    /// Like [`Self::round_point`], but always rounds towards negative infinity.
    fn floor_point(self) -> Self;

    /// Round a size or position to an even multiple of the physical pixel size.
    ///
    /// This can be useful for crisp rendering.
    ///
    /// The [`self`] should be in coordinates of _logical UI points_.
    /// The argument [`pixels_per_point`] is the number of _physical pixels_ per logical UI point.
    /// For instance, on a high-DPI screen, `pixels_per_point` could be `2.0`.
    fn round_to_pixels(self, pixels_per_point: f32) -> Self;
}

impl GuiRounding for f32 {
    #[inline]
    fn round_point(self) -> Self {
        (self / GUI_ROUNDING).round() * GUI_ROUNDING
    }

    #[inline]
    fn floor_point(self) -> Self {
        (self / GUI_ROUNDING).floor() * GUI_ROUNDING
    }

    #[inline]
    fn round_to_pixels(self, pixels_per_point: f32) -> Self {
        (self * pixels_per_point).round() / pixels_per_point
    }
}

impl GuiRounding for f64 {
    #[inline]
    fn round_point(self) -> Self {
        (self / GUI_ROUNDING as Self).round() * GUI_ROUNDING as Self
    }

    #[inline]
    fn floor_point(self) -> Self {
        (self / GUI_ROUNDING as Self).floor() * GUI_ROUNDING as Self
    }

    #[inline]
    fn round_to_pixels(self, pixels_per_point: f32) -> Self {
        (self * pixels_per_point as Self).round() / pixels_per_point as Self
    }
}

impl GuiRounding for crate::Vec2 {
    #[inline]
    fn round_point(self) -> Self {
        Self::new(self.x.round_point(), self.y.round_point())
    }

    #[inline]
    fn floor_point(self) -> Self {
        Self::new(self.x.floor_point(), self.y.floor_point())
    }

    #[inline]
    fn round_to_pixels(self, pixels_per_point: f32) -> Self {
        Self::new(
            self.x.round_to_pixels(pixels_per_point),
            self.y.round_to_pixels(pixels_per_point),
        )
    }
}

impl GuiRounding for crate::Pos2 {
    #[inline]
    fn round_point(self) -> Self {
        Self::new(self.x.round_point(), self.y.round_point())
    }

    #[inline]
    fn floor_point(self) -> Self {
        Self::new(self.x.floor_point(), self.y.floor_point())
    }

    #[inline]
    fn round_to_pixels(self, pixels_per_point: f32) -> Self {
        Self::new(
            self.x.round_to_pixels(pixels_per_point),
            self.y.round_to_pixels(pixels_per_point),
        )
    }
}

impl GuiRounding for crate::Rect {
    #[inline]
    fn round_point(self) -> Self {
        Self::from_min_size(self.min.round_point(), self.size().round_point())
    }

    #[inline]
    fn floor_point(self) -> Self {
        Self::from_min_size(self.min.floor_point(), self.size().floor_point())
    }

    #[inline]
    fn round_to_pixels(self, pixels_per_point: f32) -> Self {
        Self::from_min_size(
            self.min.round_to_pixels(pixels_per_point),
            self.size().round_to_pixels(pixels_per_point),
        )
    }
}

#[test]
fn test_round_point() {
    assert_eq!(0.0_f32.round_point(), 0.0);
    assert_eq!((GUI_ROUNDING * 1.11).round_point(), GUI_ROUNDING);
    assert_eq!((-GUI_ROUNDING * 1.11).round_point(), -GUI_ROUNDING);
    assert_eq!(f32::NEG_INFINITY.round_point(), f32::NEG_INFINITY);
    assert_eq!(f32::INFINITY.round_point(), f32::INFINITY);
}
