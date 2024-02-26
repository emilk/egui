use crate::*;

pub struct ProgressCircle {
    progress: f32,
    text: Option<WidgetText>,
    desired_radius: Option<f32>,
    stroke: Option<Stroke>,
    segments: i32,
    clockwise: bool,
}

impl ProgressCircle {
    /// Progress in the `[0, 1]` range, where `1` is a closed circle.
    pub fn new(progress: f32) -> Self {
        Self {
            progress: progress.clamp(0.0, 1.0),
            text: None,
            desired_radius: None,
            stroke: None,
            segments: 36,
            clockwise: true,
        }
    }

    /// Text for the center of the circle.
    #[inline]
    pub fn text(mut self, text: impl Into<WidgetText>) -> Self {
        self.text = Some(text.into());
        self
    }

    /// The desired radius of the circle. Will use 1/2 the default interaction size if not set.
    #[inline]
    pub fn desired_radius(mut self, desired_radius: f32) -> Self {
        self.desired_radius = Some(desired_radius);
        self
    }

    /// Override the stroke.
    #[inline]
    pub fn stroke(mut self, stroke: impl Into<Stroke>) -> Self {
        self.stroke = Some(stroke.into());
        self
    }

    /// Number of segments in the circle.
    #[inline]
    pub fn segments(mut self, segments: i32) -> Self {
        self.segments = segments;
        self
    }

    /// Set if moving from 0.0 to 1.0 moves clockwise.
    #[inline]
    pub fn clockwise(mut self, clockwise: bool) -> Self {
        self.clockwise = clockwise;
        self
    }
}

impl Widget for ProgressCircle {
    fn ui(self, ui: &mut Ui) -> Response {
        let available_width: f32 = self
            .desired_radius
            .unwrap_or_else(|| ui.available_size_before_wrap().x);
        let available_height: f32 = self
            .desired_radius
            .unwrap_or_else(|| ui.available_size_before_wrap().y);
        let min = available_width.min(available_height);
        let (outer_rect, response) = ui.allocate_exact_size(vec2(min, min), Sense::hover());
        if ui.is_rect_visible(response.rect) {
            ui.ctx().request_repaint();
            let center = outer_rect.center();
            let r = min / 2.0;
            let visuals = ui.style().interact(&response);

            ui.painter().rect(
                outer_rect,
                Rounding::default(),
                visuals.bg_fill,
                Stroke::NONE,
            );
            // Draw the progress element
            let segment_degrees = 360.0 / self.segments as f32;
            // sin and cos functions take radians.
            let segment_radians = segment_degrees.to_radians();

            let mirror_x = if self.clockwise { -1.0 } else { 1.0 };

            let mut point_one = center;
            point_one.y -= r;
            let mut radians = segment_radians;
            let stroke = self.stroke.unwrap_or(visuals.fg_stroke);

            for _i in 0..(self.segments as f32 * self.progress) as i32 {
                let point_two = Pos2 {
                    x: center.x - (radians.sin() * r * mirror_x),
                    y: center.y - (radians.cos() * r),
                };
                ui.painter().line_segment([point_one, point_two], stroke);
                radians += segment_radians;
                point_one = point_two;
            }

            // If text is set, draw it in the center.
            if let Some(text) = self.text {
                let galley = text.into_galley(ui, Some(false), min, TextStyle::Button);
                let text_pos = center - (galley.size() / 2.0);
                let text_color = ui
                    .style()
                    .visuals
                    .override_text_color
                    .unwrap_or(ui.style().visuals.selection.stroke.color);
                ui.painter()
                    .with_clip_rect(outer_rect)
                    .galley(text_pos, galley, text_color);
            }
        }
        response
    }
}
