use emath::{Rect, TSTransform};

use super::Shape;

/// A [`Shape`] that is transformed _after_ it has been tessellated.
///
/// Everything epaint snaps to the pixel grid — text, rectangles, line segments — is snapped in the
/// coordinates of the wrapped shape, and the transform then applies to the finished rendering.
///
/// Use this for a transform that animates towards [`TSTransform::IDENTITY`], such as a popup
/// scaling into place. The rendering converges on the untransformed one, so the animation doesn't
/// end with a jump of up to a pixel when the snapping lands somewhere else.
/// The cost is that the rendering is resampled while the transform is not the identity, so it is
/// slightly blurrier during the animation.
///
/// For a lasting transform, such as a pan/zoom canvas, transform the shapes instead
/// ([`Shape::transform`]): the contents then stay snapped to the pixel grid at any transform.
#[derive(Clone, Debug, PartialEq)]
pub struct TransformedShape {
    /// Applied to the tessellated rendering of `shape`.
    pub transform: TSTransform,

    pub shape: Shape,
}

impl TransformedShape {
    #[inline]
    pub fn new(transform: TSTransform, shape: Shape) -> Self {
        Self { transform, shape }
    }

    /// The visual bounding rectangle in the coordinate space the transform maps to.
    pub fn visual_bounding_rect(&self) -> Rect {
        let bounds = self.shape.visual_bounding_rect();
        if bounds.is_positive() {
            self.transform * bounds
        } else {
            bounds
        }
    }
}
