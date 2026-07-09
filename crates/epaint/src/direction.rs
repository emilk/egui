/// A cardinal direction, one of [`LeftToRight`](Direction::LeftToRight), [`RightToLeft`](Direction::RightToLeft), [`TopDown`](Direction::TopDown), [`BottomUp`](Direction::BottomUp).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum Direction {
    LeftToRight,
    RightToLeft,
    TopDown,
    BottomUp,
}

impl Direction {
    #[inline(always)]
    pub fn is_horizontal(self) -> bool {
        match self {
            Self::LeftToRight | Self::RightToLeft => true,
            Self::TopDown | Self::BottomUp => false,
        }
    }

    #[inline(always)]
    pub fn is_vertical(self) -> bool {
        match self {
            Self::LeftToRight | Self::RightToLeft => false,
            Self::TopDown | Self::BottomUp => true,
        }
    }
}
