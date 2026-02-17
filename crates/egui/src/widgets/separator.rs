use crate::{Response, Sense, Ui, Vec2, Widget, vec2, widget_style::SeparatorStyle};

/// A visual separator. A horizontal or vertical line (depending on [`crate::Layout`]).
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
#[must_use = "You should put this widget in a ui with `ui.add(widget);`"]
pub struct Separator {
    spacing: Option<f32>,
    grow: f32,
    is_horizontal_line: Option<bool>,
}

impl Default for Separator {
    fn default() -> Self {
        Self {
            spacing: None,
            grow: 0.0,
            is_horizontal_line: None,
        }
    }
}

impl Separator {
    /// How much space we take up. The line is painted in the middle of this.
    ///
    /// In a vertical layout, with a horizontal Separator,
    /// this is the height of the separator widget.
    ///
    /// In a horizontal layout, with a vertical Separator,
    /// this is the width of the separator widget.
    #[inline]
    pub fn spacing(mut self, spacing: f32) -> Self {
        self.spacing = Some(spacing);
        self
    }

    /// Explicitly ask for a horizontal line.
    ///
    /// By default you will get a horizontal line in vertical layouts,
    /// and a vertical line in horizontal layouts.
    #[inline]
    pub fn horizontal(mut self) -> Self {
        self.is_horizontal_line = Some(true);
        self
    }

    /// Explicitly ask for a vertical line.
    ///
    /// By default you will get a horizontal line in vertical layouts,
    /// and a vertical line in horizontal layouts.
    #[inline]
    pub fn vertical(mut self) -> Self {
        self.is_horizontal_line = Some(false);
        self
    }

    /// Extend each end of the separator line by this much.
    ///
    /// The default is to take up the available width/height of the parent.
    ///
    /// This will make the line extend outside the parent ui.
    #[inline]
    pub fn grow(mut self, extra: f32) -> Self {
        self.grow += extra;
        self
    }

    /// Contract each end of the separator line by this much.
    ///
    /// The default is to take up the available width/height of the parent.
    ///
    /// This effectively adds margins to the line.
    #[inline]
    pub fn shrink(mut self, shrink: f32) -> Self {
        self.grow -= shrink;
        self
    }
}

impl Widget for Separator {
    fn ui(self, ui: &mut Ui) -> Response {
        let Self {
            spacing,
            grow,
            is_horizontal_line,
        } = self;

        // Get the widget style by reading the response from the previous pass
        let id = ui.next_auto_id();
        let response: Option<Response> = ui.ctx().read_response(id);
        let state = response.map(|r| r.widget_state()).unwrap_or_default();
        let SeparatorStyle {
            spacing: spacing_style,
            stroke,
        } = ui.style().separator_style(state);

        // override the spacing if not set
        let spacing = spacing.unwrap_or(spacing_style);

        let is_horizontal_line = is_horizontal_line
            .unwrap_or_else(|| ui.is_grid() || !ui.layout().main_dir().is_horizontal());

        let available_space = if ui.is_sizing_pass() {
            Vec2::ZERO
        } else {
            ui.available_size_before_wrap()
        };

        let size = if is_horizontal_line {
            vec2(available_space.x, spacing)
        } else {
            vec2(spacing, available_space.y)
        };

        let (rect, response) = ui.allocate_at_least(size, Sense::hover());

        if ui.is_rect_visible(response.rect) {
            let painter = ui.painter();
            if is_horizontal_line {
                painter.hline(
                    (rect.left() - grow)..=(rect.right() + grow),
                    rect.center().y,
                    stroke,
                );
            } else {
                painter.vline(
                    rect.center().x,
                    (rect.top() - grow)..=(rect.bottom() + grow),
                    stroke,
                );
            }
        }

        response
    }
}
