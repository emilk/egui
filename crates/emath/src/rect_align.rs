use crate::{Align2, Pos2, Rect, Vec2};

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
            child: align.flip(),
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
    pub fn flip_x(self) -> Self {
        Self {
            parent: self.parent.flip_x(),
            child: self.child.flip_x(),
        }
    }

    /// Flip the alignment on the y-axis.
    pub fn flip_y(self) -> Self {
        Self {
            parent: self.parent.flip_y(),
            child: self.child.flip_y(),
        }
    }

    /// Flip the alignment on both axes.
    pub fn flip(self) -> Self {
        Self {
            parent: self.parent.flip(),
            child: self.child.flip(),
        }
    }

    /// Returns the 3 alternative [`RectAlign`]s that are flipped in various ways, for use
    /// with [`RectAlign::find_best_align`].
    pub fn symmetries(self) -> [Self; 3] {
        [self.flip_x(), self.flip_y(), self.flip()]
    }

    /// Look for the [`RectAlign`] that fits best in the available space.
    ///
    /// See also:
    /// - [`RectAlign::symmetries`] to calculate alternatives
    /// - [`RectAlign::MENU_ALIGNS`] for the 12 common menu positions
    pub fn find_best_align(
        mut values_to_try: impl Iterator<Item = Self>,
        available_space: Rect,
        parent_rect: Rect,
        gap: f32,
        size: Vec2,
    ) -> Self {
        let area = size.x * size.y;

        let blocked_area = |pos: Self| {
            let rect = pos.align_rect(&parent_rect, size, gap);
            area - available_space.intersect(rect).area()
        };

        let first = values_to_try.next().unwrap_or_default();

        if blocked_area(first) == 0.0 {
            return first;
        }

        let mut best_area = blocked_area(first);
        let mut best = first;

        for align in values_to_try {
            let blocked = blocked_area(align);
            if blocked == 0.0 {
                return align;
            }
            if blocked < best_area {
                best = align;
                best_area = blocked;
            }
        }

        best
    }
}
