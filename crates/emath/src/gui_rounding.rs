/// We (sometimes) round sizes and coordinates to an even multiple of this value.
///
/// This is only used for rounding _logical UI points_, used for widget coordinates and sizes.
/// When rendering, you may want to round to an integer multiple of the physical _pixels_ instead,
/// using [`GuiRounding::round_to_pixels`].
///
/// See [`GuiRounding::round_ui`] for more information.
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
/// See [`GuiRounding::round_ui`] for more information.
pub trait GuiRounding {
    /// Rounds floating point numbers to an even multiple of the GUI rounding factor, [`crate::GUI_ROUNDING`].
    ///
    /// Use this for widget coordinates and sizes.
    ///
    /// Rounding sizes and positions prevent rounding errors when doing sizing calculations.
    /// We don't round to integers, because that would be too coarse (causing visible juddering when scrolling, for instance).
    /// Instead we round to an even multiple of [`GUI_ROUNDING`].
    fn round_ui(self) -> Self;

    /// Like [`Self::round_ui`], but always rounds towards negative infinity.
    fn floor_ui(self) -> Self;

    /// Round a size or position to an even multiple of the physical pixel size.
    ///
    /// This can be useful for crisp rendering.
    ///
    /// The `self` should be in coordinates of _logical UI points_.
    /// The argument `pixels_per_point` is the number of _physical pixels_ per logical UI point.
    /// For instance, on a high-DPI screen, `pixels_per_point` could be `2.0`.
    fn round_to_pixels(self, pixels_per_point: f32) -> Self;

    /// Will round the position to be in the center of a pixel.
    ///
    /// The pixel size is `1.0 / pixels_per_point`.
    ///
    /// So if `pixels_per_point = 2` (i.e. `pixel size = 0.5`),
    /// then the position will be rounded to the closest of `…, 0.25, 0.75, 1.25, …`.
    ///
    /// This is useful, for instance, when picking the center of a line that is one pixel wide.
    fn round_to_pixel_center(self, pixels_per_point: f32) -> Self;
}

impl GuiRounding for f32 {
    #[inline]
    fn round_ui(self) -> Self {
        (self / GUI_ROUNDING).round() * GUI_ROUNDING
    }

    #[inline]
    fn floor_ui(self) -> Self {
        (self / GUI_ROUNDING).floor() * GUI_ROUNDING
    }

    #[inline]
    fn round_to_pixels(self, pixels_per_point: f32) -> Self {
        (self * pixels_per_point).round() / pixels_per_point
    }

    #[inline]
    fn round_to_pixel_center(self, pixels_per_point: f32) -> Self {
        ((self * pixels_per_point - 0.5).round() + 0.5) / pixels_per_point
    }
}

impl GuiRounding for f64 {
    #[inline]
    fn round_ui(self) -> Self {
        (self / GUI_ROUNDING as Self).round() * GUI_ROUNDING as Self
    }

    #[inline]
    fn floor_ui(self) -> Self {
        (self / GUI_ROUNDING as Self).floor() * GUI_ROUNDING as Self
    }

    #[inline]
    fn round_to_pixels(self, pixels_per_point: f32) -> Self {
        (self * pixels_per_point as Self).round() / pixels_per_point as Self
    }

    #[inline]
    fn round_to_pixel_center(self, pixels_per_point: f32) -> Self {
        ((self * pixels_per_point as Self - 0.5).round() + 0.5) / pixels_per_point as Self
    }
}

impl GuiRounding for crate::Vec2 {
    #[inline]
    fn round_ui(self) -> Self {
        Self::new(self.x.round_ui(), self.y.round_ui())
    }

    #[inline]
    fn floor_ui(self) -> Self {
        Self::new(self.x.floor_ui(), self.y.floor_ui())
    }

    #[inline]
    fn round_to_pixels(self, pixels_per_point: f32) -> Self {
        Self::new(
            self.x.round_to_pixels(pixels_per_point),
            self.y.round_to_pixels(pixels_per_point),
        )
    }

    // This doesn't really make sense for a Vec2, but 🤷‍♂️
    #[inline]
    fn round_to_pixel_center(self, pixels_per_point: f32) -> Self {
        Self::new(
            self.x.round_to_pixel_center(pixels_per_point),
            self.y.round_to_pixel_center(pixels_per_point),
        )
    }
}

impl GuiRounding for crate::Pos2 {
    #[inline]
    fn round_ui(self) -> Self {
        Self::new(self.x.round_ui(), self.y.round_ui())
    }

    #[inline]
    fn floor_ui(self) -> Self {
        Self::new(self.x.floor_ui(), self.y.floor_ui())
    }

    #[inline]
    fn round_to_pixels(self, pixels_per_point: f32) -> Self {
        Self::new(
            self.x.round_to_pixels(pixels_per_point),
            self.y.round_to_pixels(pixels_per_point),
        )
    }

    #[inline]
    fn round_to_pixel_center(self, pixels_per_point: f32) -> Self {
        Self::new(
            self.x.round_to_pixel_center(pixels_per_point),
            self.y.round_to_pixel_center(pixels_per_point),
        )
    }
}

impl GuiRounding for crate::Rect {
    /// Rounded so that two adjacent rects that tile perfectly
    /// will continue to tile perfectly.
    #[inline]
    fn round_ui(self) -> Self {
        Self::from_min_max(self.min.round_ui(), self.max.round_ui())
    }

    /// Rounded so that two adjacent rects that tile perfectly
    /// will continue to tile perfectly.
    #[inline]
    fn floor_ui(self) -> Self {
        Self::from_min_max(self.min.floor_ui(), self.max.floor_ui())
    }

    /// Rounded so that two adjacent rects that tile perfectly
    /// will continue to tile perfectly.
    #[inline]
    fn round_to_pixels(self, pixels_per_point: f32) -> Self {
        Self::from_min_max(
            self.min.round_to_pixels(pixels_per_point),
            self.max.round_to_pixels(pixels_per_point),
        )
    }

    /// Rounded so that two adjacent rects that tile perfectly
    /// will continue to tile perfectly.
    #[inline]
    fn round_to_pixel_center(self, pixels_per_point: f32) -> Self {
        Self::from_min_max(
            self.min.round_to_pixel_center(pixels_per_point),
            self.max.round_to_pixel_center(pixels_per_point),
        )
    }
}

#[test]
fn test_gui_rounding() {
    assert_eq!(0.0_f32.round_ui(), 0.0);
    assert_eq!((GUI_ROUNDING * 1.11).round_ui(), GUI_ROUNDING);
    assert_eq!((-GUI_ROUNDING * 1.11).round_ui(), -GUI_ROUNDING);
    assert_eq!(f32::NEG_INFINITY.round_ui(), f32::NEG_INFINITY);
    assert_eq!(f32::INFINITY.round_ui(), f32::INFINITY);

    assert_eq!(0.17_f32.round_to_pixel_center(2.0), 0.25);
}
