use crate::*;

/// A visual separator. A horizontal or vertical line (depending on [`Layout`]).
///
/// Usually you'd use the shorter version [`Ui::separator`].
///
/// ```
/// # let ui = &mut egui::Ui::__test();
/// // These are equivalent:
/// ui.separator();
/// ui.add(egui::Separator::new());
/// ```
#[must_use = "You should put this widget in an ui with `ui.add(widget);`"]
pub struct Separator {
    spacing: f32,
    is_horizontal_line: Option<bool>,
}

impl Separator {
    pub fn new() -> Self {
        Self {
            spacing: 6.0,
            is_horizontal_line: None,
        }
    }

    /// How much space we take up. The line is painted in the middle of this.
    pub fn spacing(mut self, spacing: f32) -> Self {
        self.spacing = spacing;
        self
    }

    /// Explicitly ask for a horizontal line.
    /// By default you will get a horizontal line in vertical layouts,
    /// and a vertical line in horizontal layouts.
    pub fn horizontal(mut self) -> Self {
        self.is_horizontal_line = Some(true);
        self
    }

    /// Explicitly ask for a vertical line.
    /// By default you will get a horizontal line in vertical layouts,
    /// and a vertical line in horizontal layouts.
    pub fn vertical(mut self) -> Self {
        self.is_horizontal_line = Some(false);
        self
    }
}

impl Widget for Separator {
    fn ui(self, ui: &mut Ui) -> Response {
        let Separator {
            spacing,
            is_horizontal_line,
        } = self;

        let is_horizontal_line = is_horizontal_line
            .unwrap_or_else(|| ui.is_grid() || !ui.layout().main_dir().is_horizontal());

        let available_space = ui.available_size_before_wrap_finite();

        let size = if is_horizontal_line {
            vec2(available_space.x, spacing)
        } else {
            vec2(spacing, available_space.y)
        };

        let (rect, response) = ui.allocate_at_least(size, Sense::hover());
        let points = if is_horizontal_line {
            [
                pos2(rect.left(), rect.center().y),
                pos2(rect.right(), rect.center().y),
            ]
        } else {
            [
                pos2(rect.center().x, rect.top()),
                pos2(rect.center().x, rect.bottom()),
            ]
        };
        let stroke = ui.visuals().widgets.noninteractive.bg_stroke;
        ui.painter().line_segment(points, stroke);
        response
    }
}
