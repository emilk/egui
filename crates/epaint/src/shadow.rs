use crate::{Color32, CornerRadius, MarginF32, Rect, RectShape, Vec2};

/// The color and fuzziness of a fuzzy shape.
///
/// Can be used for a rectangular shadow with a soft penumbra.
///
/// Very similar to a box-shadow in CSS.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Shadow {
    /// Move the shadow by this much.
    ///
    /// For instance, a value of `[1.0, 2.0]` will move the shadow 1 point to the right and 2 points down,
    /// causing a drop-shadow effect.
    pub offset: [i8; 2],

    /// The width of the blur, i.e. the width of the fuzzy penumbra.
    ///
    /// A value of 0 means a sharp shadow.
    pub blur: u8,

    /// Expand the shadow in all directions by this much.
    pub spread: u8,

    /// Color of the opaque center of the shadow.
    pub color: Color32,
}

#[test]
fn shadow_size() {
    assert_eq!(
        std::mem::size_of::<Shadow>(), 8,
        "Shadow changed size! If it shrank - good! Update this test. If it grew - bad! Try to find a way to avoid it."
    );
}

impl Shadow {
    /// No shadow at all.
    pub const NONE: Self = Self {
        offset: [0, 0],
        blur: 0,
        spread: 0,
        color: Color32::TRANSPARENT,
    };

    /// The argument is the rectangle of the shadow caster.
    pub fn as_shape(&self, rect: Rect, corner_radius: impl Into<CornerRadius>) -> RectShape {
        // tessellator.clip_rect = clip_rect; // TODO(emilk): culling

        let Self {
            offset,
            blur,
            spread,
            color,
        } = *self;
        let [offset_x, offset_y] = offset;

        let rect = rect
            .translate(Vec2::new(offset_x as _, offset_y as _))
            .expand(spread as _);
        let corner_radius = corner_radius.into() + CornerRadius::from(spread);

        RectShape::filled(rect, corner_radius, color).with_blur_width(blur as _)
    }

    /// How much larger than the parent rect are we in each direction?
    pub fn margin(&self) -> MarginF32 {
        let Self {
            offset,
            blur,
            spread,
            color: _,
        } = *self;
        let spread = spread as f32;
        let blur = blur as f32;
        let [offset_x, offset_y] = offset;
        MarginF32 {
            left: spread + 0.5 * blur - offset_x as f32,
            right: spread + 0.5 * blur + offset_x as f32,
            top: spread + 0.5 * blur - offset_y as f32,
            bottom: spread + 0.5 * blur + offset_y as f32,
        }
    }
}
