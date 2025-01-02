use crate::*;

/// A path which can be stroked and/or filled (if closed).
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct PathShape {
    /// Filled paths should prefer clockwise order.
    pub points: Vec<Pos2>,

    /// If true, connect the first and last of the points together.
    /// This is required if `fill != TRANSPARENT`.
    pub closed: bool,

    /// Fill is only supported for convex polygons.
    pub fill: Color32,

    /// Color and thickness of the line.
    pub stroke: PathStroke,
    // TODO(emilk): Add texture support either by supplying uv for each point,
    // or by some transform from points to uv (e.g. a callback or a linear transform matrix).
}

impl PathShape {
    /// A line through many points.
    ///
    /// Use [`Shape::line_segment`] instead if your line only connects two points.
    #[inline]
    pub fn line(points: Vec<Pos2>, stroke: impl Into<PathStroke>) -> Self {
        Self {
            points,
            closed: false,
            fill: Default::default(),
            stroke: stroke.into(),
        }
    }

    /// A line that closes back to the start point again.
    #[inline]
    pub fn closed_line(points: Vec<Pos2>, stroke: impl Into<PathStroke>) -> Self {
        Self {
            points,
            closed: true,
            fill: Default::default(),
            stroke: stroke.into(),
        }
    }

    /// A convex polygon with a fill and optional stroke.
    ///
    /// The most performant winding order is clockwise.
    #[inline]
    pub fn convex_polygon(
        points: Vec<Pos2>,
        fill: impl Into<Color32>,
        stroke: impl Into<PathStroke>,
    ) -> Self {
        Self {
            points,
            closed: true,
            fill: fill.into(),
            stroke: stroke.into(),
        }
    }

    /// The visual bounding rectangle (includes stroke width)
    #[inline]
    pub fn visual_bounding_rect(&self) -> Rect {
        if self.fill == Color32::TRANSPARENT && self.stroke.is_empty() {
            Rect::NOTHING
        } else {
            Rect::from_points(&self.points).expand(self.stroke.width / 2.0)
        }
    }
}

impl From<PathShape> for Shape {
    #[inline(always)]
    fn from(shape: PathShape) -> Self {
        Self::Path(shape)
    }
}
