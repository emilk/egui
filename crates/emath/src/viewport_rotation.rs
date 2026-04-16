use super::{Pos2, Rect, Vec2};

/// Viewport rotation in 90-degree increments (clockwise).
///
/// When applied, the entire UI is rendered rotated and all input coordinates
/// (mouse, touch) are automatically remapped. Application code sees a normal
/// coordinate space — rotation is transparent.
///
/// Use cases: pinball cabinet displays, kiosks, embedded screens,
/// industrial panels mounted in non-standard orientation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum ViewportRotation {
    /// No rotation (0 degrees).
    #[default]
    None,

    /// 90 degrees clockwise.
    CW90,

    /// 180 degrees.
    CW180,

    /// 270 degrees clockwise (= 90 degrees counter-clockwise).
    CW270,
}

impl ViewportRotation {
    /// Returns `true` if width and height are swapped (90 or 270 degrees).
    #[inline]
    pub fn swaps_axes(self) -> bool {
        matches!(self, Self::CW90 | Self::CW270)
    }

    /// Returns `true` if this is [`Self::None`].
    #[inline]
    pub fn is_none(self) -> bool {
        self == Self::None
    }

    /// Transform a point from physical screen space to logical UI space.
    ///
    /// `physical_size` is the physical window size (before rotation).
    #[inline]
    pub fn transform_pos(self, pos: Pos2, physical_size: Vec2) -> Pos2 {
        match self {
            Self::None => pos,
            Self::CW90 => Pos2::new(physical_size.y - pos.y, pos.x),
            Self::CW180 => Pos2::new(
                physical_size.x - pos.x,
                physical_size.y - pos.y,
            ),
            Self::CW270 => Pos2::new(pos.y, physical_size.x - pos.x),
        }
    }

    /// Transform a point from logical UI space back to physical screen space.
    #[inline]
    pub fn inverse_transform_pos(self, pos: Pos2, logical_size: Vec2) -> Pos2 {
        match self {
            Self::None => pos,
            Self::CW90 => Pos2::new(pos.y, logical_size.x - pos.x),
            Self::CW180 => Pos2::new(
                logical_size.x - pos.x,
                logical_size.y - pos.y,
            ),
            Self::CW270 => Pos2::new(logical_size.y - pos.y, pos.x),
        }
    }

    /// Transform a delta/vector (no translation needed).
    #[inline]
    pub fn transform_vec(self, vec: Vec2) -> Vec2 {
        match self {
            Self::None => vec,
            Self::CW90 => Vec2::new(-vec.y, vec.x),
            Self::CW180 => Vec2::new(-vec.x, -vec.y),
            Self::CW270 => Vec2::new(vec.y, -vec.x),
        }
    }

    /// Logical screen rect after rotation.
    ///
    /// For 90/270 degree rotations, width and height are swapped.
    #[inline]
    pub fn transform_screen_rect(self, physical_rect: Rect) -> Rect {
        if self.swaps_axes() {
            Rect::from_min_size(
                Pos2::ZERO,
                Vec2::new(physical_rect.height(), physical_rect.width()),
            )
        } else {
            physical_rect
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_none_is_identity() {
        let pos = Pos2::new(10.0, 20.0);
        let size = Vec2::new(800.0, 600.0);
        assert_eq!(ViewportRotation::None.transform_pos(pos, size), pos);
        assert_eq!(ViewportRotation::None.inverse_transform_pos(pos, size), pos);
    }

    #[test]
    fn test_roundtrip_all_rotations() {
        let physical_size = Vec2::new(800.0, 600.0);
        let pos = Pos2::new(100.0, 200.0);

        for rotation in [
            ViewportRotation::None,
            ViewportRotation::CW90,
            ViewportRotation::CW180,
            ViewportRotation::CW270,
        ] {
            let logical_size = if rotation.swaps_axes() {
                Vec2::new(physical_size.y, physical_size.x)
            } else {
                physical_size
            };

            let transformed = rotation.transform_pos(pos, physical_size);
            let back = rotation.inverse_transform_pos(transformed, logical_size);
            assert!(
                (back.x - pos.x).abs() < 1e-6 && (back.y - pos.y).abs() < 1e-6,
                "Roundtrip failed for {rotation:?}: {pos:?} -> {transformed:?} -> {back:?}"
            );
        }
    }

    #[test]
    fn test_cw90_transform() {
        let physical_size = Vec2::new(800.0, 600.0);
        // Physical top-left (0,0) should map to logical (600, 0) — bottom-left in physical
        let pos = Pos2::new(0.0, 0.0);
        let result = ViewportRotation::CW90.transform_pos(pos, physical_size);
        assert_eq!(result, Pos2::new(600.0, 0.0));
    }

    #[test]
    fn test_cw180_transform() {
        let physical_size = Vec2::new(800.0, 600.0);
        let pos = Pos2::new(0.0, 0.0);
        let result = ViewportRotation::CW180.transform_pos(pos, physical_size);
        assert_eq!(result, Pos2::new(800.0, 600.0));
    }

    #[test]
    fn test_cw270_transform() {
        let physical_size = Vec2::new(800.0, 600.0);
        let pos = Pos2::new(0.0, 0.0);
        let result = ViewportRotation::CW270.transform_pos(pos, physical_size);
        assert_eq!(result, Pos2::new(0.0, 800.0));
    }

    #[test]
    fn test_transform_screen_rect() {
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0));

        assert_eq!(ViewportRotation::None.transform_screen_rect(rect), rect);
        assert_eq!(
            ViewportRotation::CW90.transform_screen_rect(rect),
            Rect::from_min_size(Pos2::ZERO, Vec2::new(600.0, 800.0))
        );
        assert_eq!(ViewportRotation::CW180.transform_screen_rect(rect), rect);
        assert_eq!(
            ViewportRotation::CW270.transform_screen_rect(rect),
            Rect::from_min_size(Pos2::ZERO, Vec2::new(600.0, 800.0))
        );
    }

    #[test]
    fn test_transform_vec() {
        let v = Vec2::new(1.0, 0.0);
        assert_eq!(ViewportRotation::CW90.transform_vec(v), Vec2::new(0.0, 1.0));
        assert_eq!(ViewportRotation::CW180.transform_vec(v), Vec2::new(-1.0, 0.0));
        assert_eq!(ViewportRotation::CW270.transform_vec(v), Vec2::new(0.0, -1.0));
    }

    #[test]
    fn test_swaps_axes() {
        assert!(!ViewportRotation::None.swaps_axes());
        assert!(ViewportRotation::CW90.swaps_axes());
        assert!(!ViewportRotation::CW180.swaps_axes());
        assert!(ViewportRotation::CW270.swaps_axes());
    }
}
