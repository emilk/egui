/// Two bools, one for each axis (X and Y).
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Vec2b {
    pub x: bool,
    pub y: bool,
}

impl Vec2b {
    pub const FALSE: Self = Self::new(false, false);
    pub const TRUE: Self = Self::new(true, true);

    #[inline]
    pub const fn new(x: bool, y: bool) -> Self {
        Self { x, y }
    }

    #[inline]
    pub const fn any(&self) -> bool {
        self.x || self.y
    }

    /// Are both `x` and `y` true?
    #[inline]
    pub const fn all(&self) -> bool {
        self.x && self.y
    }

    #[inline]
    pub const fn const_and(&self, other: Self) -> Self {
        Self::new(self.x && other.x, self.y && other.y)
    }

    #[inline]
    pub fn and(&self, other: impl Into<Self>) -> Self {
        let other = other.into();
        self.const_and(other)
    }

    #[inline]
    pub const fn const_or(&self, other: Self) -> Self {
        Self::new(self.x || other.x, self.y || other.y)
    }

    #[inline]
    pub fn or(&self, other: impl Into<Self>) -> Self {
        let other = other.into();
        self.const_or(other)
    }
}

impl From<bool> for Vec2b {
    #[inline]
    fn from(val: bool) -> Self {
        Self::new(val, val)
    }
}

impl From<[bool; 2]> for Vec2b {
    #[inline]
    fn from([x, y]: [bool; 2]) -> Self {
        Self::new(x, y)
    }
}

impl std::ops::Index<usize> for Vec2b {
    type Output = bool;

    #[inline(always)]
    fn index(&self, index: usize) -> &bool {
        match index {
            0 => &self.x,
            1 => &self.y,
            _ => panic!("Vec2b index out of bounds: {index}"),
        }
    }
}

impl std::ops::IndexMut<usize> for Vec2b {
    #[inline(always)]
    fn index_mut(&mut self, index: usize) -> &mut bool {
        match index {
            0 => &mut self.x,
            1 => &mut self.y,
            _ => panic!("Vec2b index out of bounds: {index}"),
        }
    }
}

impl std::ops::Not for Vec2b {
    type Output = Self;

    #[inline]
    fn not(self) -> Self::Output {
        Self::new(!self.x, !self.y)
    }
}
