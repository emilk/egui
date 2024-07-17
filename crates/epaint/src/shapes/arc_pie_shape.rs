use crate::*;

/// A arc or pie slice with a given start and end angle.
#[derive(Clone, Debug, PartialEq)]
pub struct ArcPieShape {
    pub center: Pos2,
    pub radius: f32,
    pub start_angle: f32,
    pub end_angle: f32,
    pub closed: bool,
    pub fill: Color32,
    pub stroke: PathStroke,
}

impl ArcPieShape {
    /// Create a new arc or pie shape.
    ///
    /// # Arguments
    ///
    /// * `center` - The center of the arc or pie.
    /// * `radius` - The radius of the arc or pie.
    /// * `start_angle` - The start angle of the arc or pie, in radians.
    /// * `end_angle` - The end angle of the arc or pie, in radians.
    /// * `closed` - If true, connect the center with the start and end points.
    /// * `fill` - The fill color of the arc or pie.
    /// * `stroke` - The stroke of the arc or pie.
    pub fn new(
        center: Pos2,
        radius: f32,
        start_angle: f32,
        end_angle: f32,
        closed: bool,
        fill: impl Into<Color32>,
        stroke: impl Into<PathStroke>,
    ) -> Self {
        Self {
            center,
            radius,
            start_angle,
            end_angle,
            closed,
            fill: fill.into(),
            stroke: stroke.into(),
        }
    }

    /// Create a new arc shape.
    ///
    /// # Arguments
    ///
    /// * `center` - The center of the arc.
    /// * `radius` - The radius of the arc.
    /// * `start_angle` - The start angle of the arc, in radians.
    /// * `end_angle` - The end angle of the arc, in radians.
    /// * `stroke` - The stroke of the arc.
    pub fn arc(
        center: Pos2,
        radius: f32,
        start_angle: f32,
        end_angle: f32,
        stroke: impl Into<PathStroke>,
    ) -> Self {
        Self::new(
            center,
            radius,
            start_angle,
            end_angle,
            false,
            Color32::TRANSPARENT,
            stroke,
        )
    }

    /// Create a new pie shape.
    ///
    /// # Arguments
    ///
    /// * `center` - The center of the pie.
    /// * `radius` - The radius of the pie.
    /// * `start_angle` - The start angle of the pie, in radians.
    /// * `end_angle` - The end angle of the pie, in radians.
    /// * `fill` - The fill color of the pie.
    /// * `stroke` - The stroke of the pie.
    pub fn pie(
        center: Pos2,
        radius: f32,
        start_angle: f32,
        end_angle: f32,
        fill: impl Into<Color32>,
        stroke: impl Into<PathStroke>,
    ) -> Self {
        Self::new(center, radius, start_angle, end_angle, true, fill, stroke)
    }

    /// The visual bounding rectangle (includes stroke width)
    pub fn visual_bounding_rect(&self) -> Rect {
        if self.fill == Color32::TRANSPARENT && self.stroke.is_empty() {
            Rect::NOTHING
        } else {
            let rect = Rect::from_center_size(self.center, vec2(self.radius, self.radius));
            let start =
                self.center + vec2(self.start_angle.cos(), self.start_angle.sin()) * self.radius;
            let end = self.center + vec2(self.end_angle.cos(), self.end_angle.sin()) * self.radius;
            rect.union(Rect::from_two_pos(start, end))
        }
    }
}
