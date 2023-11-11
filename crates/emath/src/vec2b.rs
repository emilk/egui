/// Two bools, one for each axis (X and Y).
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Vec2b {
    pub x: bool,
    pub y: bool,
}

impl Vec2b {
    #[inline]
    pub fn new(x: bool, y: bool) -> Self {
        Self { x, y }
    }

    #[inline]
    pub fn any(&self) -> bool {
        self.x || self.y
    }
}

impl From<bool> for Vec2b {
    #[inline]
    fn from(val: bool) -> Self {
        Vec2b { x: val, y: val }
    }
}

impl From<[bool; 2]> for Vec2b {
    #[inline]
    fn from([x, y]: [bool; 2]) -> Self {
        Vec2b { x, y }
    }
}
