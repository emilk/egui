use crate::*;

/// A visual separator. A horizontal or vertical line (depending on [`Layout`]).
///
/// Usually you'd use the shorter version [`Ui::separator`].
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// // These are equivalent:
/// ui.separator();
/// ui.add(egui::Separator::default());
/// # });
/// ```
#[must_use = "You should put this widget in an ui with `ui.add(widget);`"]
pub struct Separator {
    spacing: f32,
    is_horizontal_line: Option<bool>,
}

impl Default for Separator {
    fn default() -> Self {
        Self {
            spacing: 6.0,
            is_horizontal_line: None,
        }
    }
}

impl Separator {
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

        let available_space = ui.available_size_before_wrap();

        let size = if is_horizontal_line {
            vec2(available_space.x, spacing)
        } else {
            vec2(spacing, available_space.y)
        };

        let (rect, response) = ui.allocate_at_least(size, Sense::hover());

        if ui.is_rect_visible(response.rect) {
            let stroke = ui.visuals().widgets.noninteractive.bg_stroke;
            if is_horizontal_line {
                ui.painter().hline(rect.x_range(), rect.center().y, stroke);
            } else {
                ui.painter().vline(rect.center().x, rect.y_range(), stroke);
            }
        }

        response
    }
}
