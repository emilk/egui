use epaint::{emath::lerp, vec2, Pos2, Shape, Stroke};

use crate::{Response, Sense, Ui, Widget};

/// A spinner widget used to indicate loading.
#[must_use = "You should put this widget in an ui with `ui.add(widget);`"]
pub struct Spinner {
    enabled: bool,
    /// Uses the style's `interact_size` if `None`.
    size: Option<f32>,
}

impl Spinner {
    /// A new spinner that is shown if `enabled` is true.
    /// Uses the style's `interact_size` unless changed.
    /// A disabled spinner still takes up it's usual space in order to prevent inconsistent
    /// alignment.
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            size: None,
        }
    }

    /// Sets the spinner's size. The size sets both the height and width, as the spinner is always
    /// square.
    pub fn size(mut self, size: f32) -> Self {
        self.size = Some(size);
        self
    }
}

impl Widget for Spinner {
    fn ui(self, ui: &mut Ui) -> Response {
        let size = self
            .size
            .unwrap_or_else(|| ui.style().spacing.interact_size.y);
        let (rect, response) = ui.allocate_exact_size(vec2(size, size), Sense::hover());

        if self.enabled {
            ui.ctx().request_repaint();

            let corner_radius = rect.height() / 2.0;
            let n_points = 20;
            let start_angle = ui.input().time as f64 * 360f64.to_radians();
            let end_angle = start_angle + 240f64.to_radians() * ui.input().time.sin();
            let circle_radius = corner_radius - 2.0;
            let points: Vec<Pos2> = (0..n_points)
                .map(|i| {
                    let angle = lerp(start_angle..=end_angle, i as f64 / n_points as f64);
                    let (sin, cos) = angle.sin_cos();
                    rect.right_center()
                        + circle_radius * vec2(cos as f32, sin as f32)
                        + vec2(-corner_radius, 0.0)
                })
                .collect();
            ui.painter().add(Shape::line(
                points,
                Stroke::new(3.0, ui.visuals().strong_text_color()),
            ));
        }

        response
    }
}
