use crate::{Align2, Pos2, ReadingDirection, Rect, Vec2};

/// Position a child [`Rect`] relative to a parent [`Rect`].
///
/// The corner from [`RectAlign::child`] on the new rect will be aligned to
/// the corner from [`RectAlign::parent`] on the original rect.
///
/// There are helper constants for the 12 common menu positions:
/// ```text
///              ┌───────────┐  ┌────────┐  ┌─────────┐
///              │ TOP_START │  │  TOP   │  │ TOP_END │
///              └───────────┘  └────────┘  └─────────┘
/// ┌──────────┐ ┌────────────────────────────────────┐ ┌───────────┐
/// │LEFT_START│ │                                    │ │RIGHT_START│
/// └──────────┘ │                                    │ └───────────┘
/// ┌──────────┐ │                                    │ ┌───────────┐
/// │   LEFT   │ │             some_rect              │ │   RIGHT   │
/// └──────────┘ │                                    │ └───────────┘
/// ┌──────────┐ │                                    │ ┌───────────┐
/// │ LEFT_END │ │                                    │ │ RIGHT_END │
/// └──────────┘ └────────────────────────────────────┘ └───────────┘
///              ┌────────────┐  ┌──────┐  ┌──────────┐
///              │BOTTOM_START│  │BOTTOM│  │BOTTOM_END│
///              └────────────┘  └──────┘  └──────────┘
/// ```
// There is no `new` function on purpose, since writing out `parent` and `child` is more
// reasonable.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct RectAlign {
    /// The alignment in the parent (original) rect.
    pub parent: Align2,

    /// The alignment in the child (new) rect.
    pub child: Align2,
}

impl Default for RectAlign {
    fn default() -> Self {
        Self::BOTTOM_START
    }
}

impl RectAlign {
    /// Along the top edge, leftmost.
    pub const TOP_START: Self = Self {
        parent: Align2::LEFT_TOP,
        child: Align2::LEFT_BOTTOM,
    };

    /// Along the top edge, centered.
    pub const TOP: Self = Self {
        parent: Align2::CENTER_TOP,
        child: Align2::CENTER_BOTTOM,
    };

    /// Along the top edge, rightmost.
    pub const TOP_END: Self = Self {
        parent: Align2::RIGHT_TOP,
        child: Align2::RIGHT_BOTTOM,
    };

    /// Along the right edge, topmost.
    pub const RIGHT_START: Self = Self {
        parent: Align2::RIGHT_TOP,
        child: Align2::LEFT_TOP,
    };

    /// Along the right edge, centered.
    pub const RIGHT: Self = Self {
        parent: Align2::RIGHT_CENTER,
        child: Align2::LEFT_CENTER,
    };

    /// Along the right edge, bottommost.
    pub const RIGHT_END: Self = Self {
        parent: Align2::RIGHT_BOTTOM,
        child: Align2::LEFT_BOTTOM,
    };

    /// Along the bottom edge, rightmost.
    pub const BOTTOM_END: Self = Self {
        parent: Align2::RIGHT_BOTTOM,
        child: Align2::RIGHT_TOP,
    };

    /// Along the bottom edge, centered.
    pub const BOTTOM: Self = Self {
        parent: Align2::CENTER_BOTTOM,
        child: Align2::CENTER_TOP,
    };

    /// Along the bottom edge, leftmost.
    pub const BOTTOM_START: Self = Self {
        parent: Align2::LEFT_BOTTOM,
        child: Align2::LEFT_TOP,
    };

    /// Along the left edge, bottommost.
    pub const LEFT_END: Self = Self {
        parent: Align2::LEFT_BOTTOM,
        child: Align2::RIGHT_BOTTOM,
    };

    /// Along the left edge, centered.
    pub const LEFT: Self = Self {
        parent: Align2::LEFT_CENTER,
        child: Align2::RIGHT_CENTER,
    };

    /// Along the left edge, topmost.
    pub const LEFT_START: Self = Self {
        parent: Align2::LEFT_TOP,
        child: Align2::RIGHT_TOP,
    };

    /// The 12 most common menu positions as an array, for use with [`RectAlign::find_best_align`].
    ///
    /// These constants name the physical sides under the crate's left-to-right
    /// default. For a right-to-left reading direction, use
    /// [`Self::menu_aligns`] (or [`Self::mirrored`]) instead of renaming
    /// or reordering anything: the constants stay stable for existing code.
    pub const MENU_ALIGNS: [Self; 12] = [
        Self::BOTTOM_START,
        Self::BOTTOM_END,
        Self::TOP_START,
        Self::TOP_END,
        Self::RIGHT_END,
        Self::RIGHT_START,
        Self::LEFT_END,
        Self::LEFT_START,
        // These come last on purpose, we prefer the corner ones
        Self::TOP,
        Self::RIGHT,
        Self::BOTTOM,
        Self::LEFT,
    ];

    /// Align in the parent rect.
    pub fn parent(&self) -> Align2 {
        self.parent
    }

    /// Align in the child rect.
    pub fn child(&self) -> Align2 {
        self.child
    }

    /// Convert an [`Align2`] to an [`RectAlign`], positioning the child rect inside the parent.
    pub fn from_align2(align: Align2) -> Self {
        Self {
            parent: align,
            child: align,
        }
    }

    /// The center of the child rect will be aligned to a corner of the parent rect.
    pub fn over_corner(align: Align2) -> Self {
        Self {
            parent: align,
            child: Align2::CENTER_CENTER,
        }
    }

    /// Position the child rect outside the parent rect.
    pub fn outside(align: Align2) -> Self {
        Self {
            parent: align,
            child: align.flipped(),
        }
    }

    /// Calculate the child rect based on a size and some optional gap.
    pub fn align_rect(&self, parent_rect: &Rect, size: Vec2, gap: f32) -> Rect {
        let (pivot, anchor) = self.pivot_pos(parent_rect, gap);
        pivot.anchor_size(anchor, size)
    }

    /// Returns a [`Align2`] and a [`Pos2`] that you can e.g. use with `Area::fixed_pos`
    /// and `Area::pivot` to align an `Area` to some rect.
    pub fn pivot_pos(&self, parent_rect: &Rect, gap: f32) -> (Align2, Pos2) {
        (self.child(), self.anchor(parent_rect, gap))
    }

    /// Returns a sign vector (-1, 0 or 1 in each direction) that can be used as an offset to the
    /// child rect, creating a gap between the rects while keeping the edges aligned.
    pub fn gap_vector(&self) -> Vec2 {
        let mut gap = -self.child.to_sign();

        // Align the edges in these cases
        match *self {
            Self::TOP_START | Self::TOP_END | Self::BOTTOM_START | Self::BOTTOM_END => {
                gap.x = 0.0;
            }
            Self::LEFT_START | Self::LEFT_END | Self::RIGHT_START | Self::RIGHT_END => {
                gap.y = 0.0;
            }
            _ => {}
        }

        gap
    }

    /// Calculator the anchor point for the child rect, based on the parent rect and an optional gap.
    pub fn anchor(&self, parent_rect: &Rect, gap: f32) -> Pos2 {
        let pos = self.parent.pos_in_rect(parent_rect);

        let offset = self.gap_vector() * gap;

        pos + offset
    }

    /// Flip the alignment on the x-axis.
    #[must_use]
    pub fn flipped_x(self) -> Self {
        Self {
            parent: self.parent.flipped_x(),
            child: self.child.flipped_x(),
        }
    }

    /// Flip the alignment on the x-axis.
    #[must_use]
    #[deprecated = "Renamed to `flipped_x`"]
    pub fn flip_x(self) -> Self {
        self.flipped_x()
    }

    /// Swap the reading-order start and end sides of this alignment,
    /// keeping the top/bottom position.
    ///
    /// `TOP_START` becomes `TOP_END`, `LEFT_START` becomes `RIGHT_START`,
    /// `BOTTOM_END` becomes `BOTTOM_START`, and so on. The vertical position
    /// is never touched: reading direction does not mirror the vertical axis.
    ///
    /// The `*_START`/`*_END` constants name the physical sides under the
    /// crate's left-to-right default; [`Self::mirrored`] turns a
    /// left-to-right placement into the right-to-left placement without
    /// changing any constant. Mirroring twice is the identity.
    ///
    /// See also [`Self::menu_aligns`].
    #[must_use]
    #[inline]
    pub fn mirrored(self) -> Self {
        Self {
            parent: self.parent.flipped_x(),
            child: self.child.flipped_x(),
        }
    }

    /// The 12 most common menu positions for the given [`ReadingDirection`],
    /// for use with [`RectAlign::find_best_align`].
    ///
    /// This is [`Self::MENU_ALIGNS`] (the left-to-right default table) with
    /// the reading-order start and end sides swapped for right-to-left
    /// reading, so popup/menu/combo placement code can resolve the reading
    /// direction once, centrally, instead of flipping each placement by hand.
    ///
    /// ```
    /// # use emath::{ReadingDirection, RectAlign};
    /// // The left-to-right table is byte-identical to the default:
    /// assert_eq!(RectAlign::menu_aligns(ReadingDirection::Ltr), RectAlign::MENU_ALIGNS);
    /// // The right-to-left table mirrors every entry of it:
    /// assert_eq!(
    ///     RectAlign::menu_aligns(ReadingDirection::Rtl),
    ///     RectAlign::MENU_ALIGNS.map(RectAlign::mirrored)
    /// );
    /// ```
    #[must_use]
    #[inline]
    pub fn menu_aligns(dir: ReadingDirection) -> [Self; 12] {
        match dir {
            ReadingDirection::Ltr => Self::MENU_ALIGNS,
            ReadingDirection::Rtl => Self::MENU_ALIGNS.map(Self::mirrored),
        }
    }

    /// Flip the alignment on the y-axis.
    #[must_use]
    pub fn flipped_y(self) -> Self {
        Self {
            parent: self.parent.flipped_y(),
            child: self.child.flipped_y(),
        }
    }

    /// Flip the alignment on the y-axis.
    #[must_use]
    #[deprecated = "Renamed to `flipped_y`"]
    pub fn flip_y(self) -> Self {
        self.flipped_y()
    }

    /// Flip the alignment on both axes.
    #[must_use]
    pub fn flipped(self) -> Self {
        Self {
            parent: self.parent.flipped(),
            child: self.child.flipped(),
        }
    }

    /// Flip the alignment on both axes.
    #[must_use]
    #[deprecated = "Renamed to `flipped`"]
    pub fn flip(self) -> Self {
        self.flipped()
    }

    /// Returns the 3 alternative [`RectAlign`]s that are flipped in various ways, for use
    /// with [`RectAlign::find_best_align`].
    #[must_use]
    pub fn symmetries(self) -> [Self; 3] {
        [self.flipped_x(), self.flipped_y(), self.flipped()]
    }

    /// Look for the first alternative [`RectAlign`] that allows the child rect to fit
    /// inside the `content_rect`.
    ///
    /// If no alternative fits, the first is returned.
    /// If no alternatives are given, `None` is returned.
    ///
    /// See also:
    /// - [`RectAlign::symmetries`] to calculate alternatives
    /// - [`RectAlign::MENU_ALIGNS`] for the 12 common menu positions
    pub fn find_best_align(
        values_to_try: impl Iterator<Item = Self>,
        content_rect: Rect,
        parent_rect: Rect,
        gap: f32,
        expected_size: Vec2,
    ) -> Option<Self> {
        let mut first_choice = None;

        for align in values_to_try {
            first_choice = first_choice.or(Some(align)); // Remember the first alternative

            let suggested_popup_rect = align.align_rect(&parent_rect, expected_size, gap);

            if content_rect.contains_rect(suggested_popup_rect) {
                return Some(align);
            }
        }

        first_choice
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mirrored_swaps_start_end_keeps_vertical() {
        assert_eq!(RectAlign::TOP_START.mirrored(), RectAlign::TOP_END);
        assert_eq!(RectAlign::TOP_END.mirrored(), RectAlign::TOP_START);
        assert_eq!(RectAlign::LEFT_START.mirrored(), RectAlign::RIGHT_START);
        assert_eq!(RectAlign::RIGHT_START.mirrored(), RectAlign::LEFT_START);
        assert_eq!(RectAlign::BOTTOM_START.mirrored(), RectAlign::BOTTOM_END);
        assert_eq!(RectAlign::BOTTOM_END.mirrored(), RectAlign::BOTTOM_START);
        // Edge-centered alignments mirror to the opposite edge...
        assert_eq!(RectAlign::LEFT.mirrored(), RectAlign::RIGHT);
        // ...and axis-centered alignments are their own mirror.
        assert_eq!(RectAlign::TOP.mirrored(), RectAlign::TOP);
        assert_eq!(RectAlign::BOTTOM.mirrored(), RectAlign::BOTTOM);
    }

    #[test]
    fn mirrored_twice_is_identity() {
        for &align in &RectAlign::MENU_ALIGNS {
            assert_eq!(align.mirrored().mirrored(), align);
        }
    }

    #[test]
    fn menu_aligns_ltr_matches_default_table() {
        // Regression gate: the left-to-right table must stay byte-identical
        // to the pre-existing physical default.
        assert_eq!(
            RectAlign::menu_aligns(ReadingDirection::Ltr),
            RectAlign::MENU_ALIGNS
        );
    }

    #[test]
    fn menu_aligns_rtl_mirrors_default_table() {
        assert_eq!(
            RectAlign::menu_aligns(ReadingDirection::Rtl),
            RectAlign::MENU_ALIGNS.map(RectAlign::mirrored)
        );
        // And the RTL table is the LTR table, entry for entry.
        assert_eq!(
            RectAlign::menu_aligns(ReadingDirection::Rtl),
            RectAlign::menu_aligns(ReadingDirection::Ltr).map(RectAlign::mirrored)
        );
    }
}
