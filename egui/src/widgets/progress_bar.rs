use crate::*;

enum ProgressBarText {
    Custom(String),
    Percentage,
}

/// A simple progress bar.
pub struct ProgressBar {
    progress: f32,
    desired_width: Option<f32>,
    text: Option<ProgressBarText>,
    animate: bool,
}

impl ProgressBar {
    /// Progress in the `[0, 1]` range, where `1` means "completed".
    pub fn new(progress: f32) -> Self {
        Self {
            progress: progress.clamp(0.0, 1.0),
            desired_width: None,
            text: None,
            animate: false,
        }
    }

    /// The desired width of the bar. Will use all horizonal space if not set.
    pub fn desired_width(mut self, desired_width: f32) -> Self {
        self.desired_width = Some(desired_width);
        self
    }

    /// A custom text to display on the progress bar.
    #[allow(clippy::needless_pass_by_value)]
    pub fn text(mut self, text: impl ToString) -> Self {
        self.text = Some(ProgressBarText::Custom(text.to_string()));
        self
    }

    /// Show the progress in percent on the progress bar.
    pub fn show_percentage(mut self) -> Self {
        self.text = Some(ProgressBarText::Percentage);
        self
    }

    /// Whether to display a loading animation when progress `< 1`.
    /// Note that this require the UI to be redrawn.
    /// Defaults to `false`.
    pub fn animate(mut self, animate: bool) -> Self {
        self.animate = animate;
        self
    }
}

impl Widget for ProgressBar {
    fn ui(self, ui: &mut Ui) -> Response {
        let ProgressBar {
            progress,
            desired_width,
            text,
            mut animate,
        } = self;

        animate &= progress < 1.0;

        let desired_width = desired_width.unwrap_or(ui.available_size_before_wrap().x);
        let height = ui.spacing().interact_size.y;
        let (outer_rect, response) =
            ui.allocate_exact_size(vec2(desired_width, height), Sense::hover());
        let visuals = ui.style().visuals.clone();
        let corner_radius = outer_rect.height() / 2.0;
        ui.painter().rect(
            outer_rect,
            corner_radius,
            visuals.extreme_bg_color,
            Stroke::none(),
        );
        let inner_rect = Rect::from_min_size(
            outer_rect.min,
            vec2(
                (outer_rect.width() * progress).at_least(outer_rect.height()),
                outer_rect.height(),
            ),
        );

        let (dark, bright) = (0.7, 1.0);
        let color_factor = if animate {
            ui.ctx().request_repaint();
            lerp(dark..=bright, ui.input().time.cos().abs())
        } else {
            bright
        };

        ui.painter().rect(
            inner_rect,
            corner_radius,
            Color32::from(Rgba::from(visuals.selection.bg_fill) * color_factor as f32),
            Stroke::none(),
        );

        if animate {
            let n_points = 20;
            let start_angle = ui.input().time as f64 * 360f64.to_radians();
            let end_angle = start_angle + 240f64.to_radians() * ui.input().time.sin();
            let circle_radius = corner_radius - 2.0;
            let points: Vec<Pos2> = (0..n_points)
                .map(|i| {
                    let angle = lerp(start_angle..=end_angle, i as f64 / n_points as f64);
                    let (sin, cos) = angle.sin_cos();
                    inner_rect.right_center()
                        + circle_radius * vec2(cos as f32, sin as f32)
                        + vec2(-corner_radius, 0.0)
                })
                .collect();
            ui.painter().add(Shape::Path {
                points,
                closed: false,
                fill: Color32::TRANSPARENT,
                stroke: Stroke::new(2.0, visuals.faint_bg_color),
            });
        }

        if let Some(text_kind) = text {
            let text = match text_kind {
                ProgressBarText::Custom(string) => string,
                ProgressBarText::Percentage => format!("{}%", (progress * 100.0) as usize),
            };
            ui.painter().sub_region(outer_rect).text(
                outer_rect.left_center() + vec2(ui.spacing().item_spacing.x, 0.0),
                Align2::LEFT_CENTER,
                text,
                TextStyle::Button,
                visuals
                    .override_text_color
                    .unwrap_or(visuals.selection.stroke.color),
            );
        }

        response
    }
}
