use crate::*;

/// A visual separator. A horizontal or vertical line (depending on [`Layout`]).
#[must_use = "You should put this widget in an ui with `ui.add(widget);`"]
pub struct Separator {
    spacing: f32,
}

impl Separator {
    pub fn new() -> Self {
        Self { spacing: 6.0 }
    }

    /// How much space we take up. The line is painted in the middle of this.
    pub fn spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing;
        self
    }
}

impl Widget for Separator {
    fn ui(self, ui: &mut Ui) -> Response {
        let Separator { spacing } = self;

        let available_space = ui.available_size_before_wrap_finite();

        let size = if ui.layout().main_dir().is_horizontal() {
            vec2(spacing, available_space.y)
        } else {
            vec2(available_space.x, spacing)
        };

        let (rect, response) = ui.allocate_at_least(size, Sense::hover());
        let points = if ui.layout().main_dir().is_horizontal() {
            [
                pos2(rect.center().x, rect.top()),
                pos2(rect.center().x, rect.bottom()),
            ]
        } else {
            [
                pos2(rect.left(), rect.center().y),
                pos2(rect.right(), rect.center().y),
            ]
        };
        let stroke = ui.style().visuals.widgets.noninteractive.bg_stroke;
        ui.painter().line_segment(points, stroke);
        response
    }
}
