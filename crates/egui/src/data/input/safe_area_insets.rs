use epaint::MarginF32;

use crate::emath::Rect;

/// The 'safe area' insets of the screen
///
/// This represents the area taken up by the status bar, navigation controls, notches,
/// or any other items that obscure parts of the screen.
#[derive(Debug, PartialEq, Copy, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct SafeAreaInsets(pub MarginF32);

impl std::ops::Sub<SafeAreaInsets> for Rect {
    type Output = Self;

    fn sub(self, rhs: SafeAreaInsets) -> Self::Output {
        self - rhs.0
    }
}
