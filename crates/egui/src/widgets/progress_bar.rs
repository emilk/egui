use crate::{
    Color32, CornerRadius, NumExt as _, Pos2, Rect, Response, Rgba, Sense, Shape, Stroke,
    TextStyle, TextWrapMode, Ui, Vec2, Widget, WidgetInfo, WidgetText, WidgetType, lerp, vec2,
};

enum ProgressBarText {
    Custom(WidgetText),
    Percentage,
}

/// A simple progress bar.
///
/// See also: [`crate::Spinner`].
#[must_use = "You should put this widget in a ui with `ui.add(widget);`"]
pub struct ProgressBar {
    progress: f32,
    desired_width: Option<f32>,
    desired_height: Option<f32>,
    text: Option<ProgressBarText>,
    fill: Option<Color32>,
    animate: bool,
    corner_radius: Option<CornerRadius>,
}

impl ProgressBar {
    /// Progress in the `[0, 1]` range, where `1` means "completed".
    pub fn new(progress: f32) -> Self {
        Self {
            progress: progress.clamp(0.0, 1.0),
            desired_width: None,
            desired_height: None,
            text: None,
            fill: None,
            animate: false,
            corner_radius: None,
        }
    }

    /// The desired width of the bar. Will use all horizontal space if not set.
    #[inline]
    pub fn desired_width(mut self, desired_width: f32) -> Self {
        self.desired_width = Some(desired_width);
        self
    }

    /// The desired height of the bar. Will use the default interaction size if not set.
    #[inline]
    pub fn desired_height(mut self, desired_height: f32) -> Self {
        self.desired_height = Some(desired_height);
        self
    }

    /// The fill color of the bar.
    #[inline]
    pub fn fill(mut self, color: Color32) -> Self {
        self.fill = Some(color);
        self
    }

    /// A custom text to display on the progress bar.
    #[inline]
    pub fn text(mut self, text: impl Into<WidgetText>) -> Self {
        self.text = Some(ProgressBarText::Custom(text.into()));
        self
    }

    /// Show the progress in percent on the progress bar.
    #[inline]
    pub fn show_percentage(mut self) -> Self {
        self.text = Some(ProgressBarText::Percentage);
        self
    }

    /// Whether to display a loading animation when progress `< 1`.
    /// Note that this will cause the UI to be redrawn.
    /// Defaults to `false`.
    ///
    /// If [`Self::corner_radius`] and [`Self::animate`] are used simultaneously, the animation is not
    /// rendered, since it requires a perfect circle to render correctly. However, the UI is still
    /// redrawn.
    #[inline]
    pub fn animate(mut self, animate: bool) -> Self {
        self.animate = animate;
        self
    }

    /// Set the rounding of the progress bar.
    ///
    /// If [`Self::corner_radius`] and [`Self::animate`] are used simultaneously, the animation is not
    /// rendered, since it requires a perfect circle to render correctly. However, the UI is still
    /// redrawn.
    #[inline]
    pub fn corner_radius(mut self, corner_radius: impl Into<CornerRadius>) -> Self {
        self.corner_radius = Some(corner_radius.into());
        self
    }

    #[inline]
    #[deprecated = "Renamed to `corner_radius`"]
    pub fn rounding(self, corner_radius: impl Into<CornerRadius>) -> Self {
        self.corner_radius(corner_radius)
    }
}

impl Widget for ProgressBar {
    fn ui(self, ui: &mut Ui) -> Response {
        let Self {
            progress,
            desired_width,
            desired_height,
            text,
            fill,
            animate,
            corner_radius,
        } = self;

        let animate = animate && progress < 1.0;

        let desired_width =
            desired_width.unwrap_or_else(|| ui.available_size_before_wrap().x.at_least(96.0));
        let height = desired_height.unwrap_or_else(|| ui.spacing().interact_size.y);
        let (outer_rect, response) =
            ui.allocate_exact_size(vec2(desired_width, height), Sense::hover());

        response.widget_info(|| {
            let mut info = if let Some(ProgressBarText::Custom(text)) = &text {
                WidgetInfo::labeled(WidgetType::ProgressIndicator, ui.is_enabled(), text.text())
            } else {
                WidgetInfo::new(WidgetType::ProgressIndicator)
            };
            info.value = Some((progress as f64 * 100.0).floor());

            info
        });

        if ui.is_rect_visible(response.rect) {
            if animate {
                ui.request_repaint();
            }

            let visuals = ui.style().visuals.clone();
            let has_custom_cr = corner_radius.is_some();
            let half_height = outer_rect.height() / 2.0;
            let corner_radius = corner_radius.unwrap_or_else(|| half_height.into());
            ui.painter()
                .rect_filled(outer_rect, corner_radius, visuals.extreme_bg_color);
            let min_width =
                2.0 * f32::max(corner_radius.sw as _, corner_radius.nw as _).at_most(half_height);
            let filled_width = (outer_rect.width() * progress).at_least(min_width);
            let inner_rect =
                Rect::from_min_size(outer_rect.min, vec2(filled_width, outer_rect.height()));

            let (dark, bright) = (0.7, 1.0);
            let color_factor = if animate {
                let time = ui.input(|i| i.time);
                lerp(dark..=bright, time.cos().abs())
            } else {
                bright
            };

            ui.painter().rect_filled(
                inner_rect,
                corner_radius,
                Color32::from(
                    Rgba::from(fill.unwrap_or(visuals.selection.bg_fill)) * color_factor as f32,
                ),
            );

            if animate && !has_custom_cr {
                let n_points = 20;
                let time = ui.input(|i| i.time);
                let start_angle = time * std::f64::consts::TAU;
                let end_angle = start_angle + 240f64.to_radians() * time.sin();
                let circle_radius = half_height - 2.0;
                let points: Vec<Pos2> = (0..n_points)
                    .map(|i| {
                        let angle = lerp(start_angle..=end_angle, i as f64 / n_points as f64);
                        let (sin, cos) = angle.sin_cos();
                        inner_rect.right_center()
                            + circle_radius * vec2(cos as f32, sin as f32)
                            + vec2(-half_height, 0.0)
                    })
                    .collect();
                ui.painter()
                    .add(Shape::line(points, Stroke::new(2.0, visuals.text_color())));
            }

            if let Some(text_kind) = text {
                let text = match text_kind {
                    ProgressBarText::Custom(text) => text,
                    ProgressBarText::Percentage => {
                        format!("{}%", (progress * 100.0) as usize).into()
                    }
                };
                let galley = text.into_galley(
                    ui,
                    Some(TextWrapMode::Extend),
                    f32::INFINITY,
                    TextStyle::Button,
                );
                let text_pos = outer_rect.left_center() - Vec2::new(0.0, galley.size().y / 2.0)
                    + vec2(ui.spacing().item_spacing.x, 0.0);
                let text_color = visuals
                    .override_text_color
                    .unwrap_or(visuals.selection.stroke.color);
                ui.painter()
                    .with_clip_rect(outer_rect)
                    .galley(text_pos, galley, text_color);
            }
        }

        response
    }
}
