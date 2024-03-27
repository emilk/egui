use std::{fmt::Debug, ops::RangeInclusive, sync::Arc};

use egui::{
    emath::{remap_clamp, round_to_decimals, Rot2},
    epaint::TextShape,
    Pos2, Rangef, Rect, Response, Sense, TextStyle, Ui, Vec2, WidgetText,
};

use super::{transform::PlotTransform, GridMark};

pub(super) type AxisFormatterFn = dyn Fn(GridMark, usize, &RangeInclusive<f64>) -> String;

/// X or Y axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    /// Horizontal X-Axis
    X = 0,

    /// Vertical Y-axis
    Y = 1,
}

impl From<Axis> for usize {
    #[inline]
    fn from(value: Axis) -> Self {
        match value {
            Axis::X => 0,
            Axis::Y => 1,
        }
    }
}

/// Placement of the horizontal X-Axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VPlacement {
    Top,
    Bottom,
}

/// Placement of the vertical Y-Axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HPlacement {
    Left,
    Right,
}

/// Placement of an axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Placement {
    /// Bottom for X-axis, or left for Y-axis.
    LeftBottom,

    /// Top for x-axis and right for y-axis.
    RightTop,
}

impl From<HPlacement> for Placement {
    #[inline]
    fn from(placement: HPlacement) -> Self {
        match placement {
            HPlacement::Left => Self::LeftBottom,
            HPlacement::Right => Self::RightTop,
        }
    }
}

impl From<Placement> for HPlacement {
    #[inline]
    fn from(placement: Placement) -> Self {
        match placement {
            Placement::LeftBottom => Self::Left,
            Placement::RightTop => Self::Right,
        }
    }
}

impl From<VPlacement> for Placement {
    #[inline]
    fn from(placement: VPlacement) -> Self {
        match placement {
            VPlacement::Top => Self::RightTop,
            VPlacement::Bottom => Self::LeftBottom,
        }
    }
}

impl From<Placement> for VPlacement {
    #[inline]
    fn from(placement: Placement) -> Self {
        match placement {
            Placement::LeftBottom => Self::Bottom,
            Placement::RightTop => Self::Top,
        }
    }
}

/// Axis configuration.
///
/// Used to configure axis label and ticks.
#[derive(Clone)]
pub struct AxisHints {
    pub(super) label: WidgetText,
    pub(super) formatter: Arc<AxisFormatterFn>,
    pub(super) digits: usize,
    pub(super) placement: Placement,
    pub(super) label_spacing: Rangef,
}

// TODO(JohannesProgrammiert): this just a guess. It might cease to work if a user changes font size.
const LINE_HEIGHT: f32 = 12.0;

impl AxisHints {
    /// Initializes a default axis configuration for the X axis.
    pub fn new_x() -> Self {
        Self::new(Axis::X)
    }

    /// Initializes a default axis configuration for the X axis.
    pub fn new_y() -> Self {
        Self::new(Axis::Y)
    }

    /// Initializes a default axis configuration for the specified axis.
    ///
    /// `label` is empty.
    /// `formatter` is default float to string formatter.
    /// maximum `digits` on tick label is 5.
    pub fn new(axis: Axis) -> Self {
        Self {
            label: Default::default(),
            formatter: Arc::new(Self::default_formatter),
            digits: 5,
            placement: Placement::LeftBottom,
            label_spacing: match axis {
                Axis::X => Rangef::new(60.0, 80.0), // labels can get pretty wide
                Axis::Y => Rangef::new(20.0, 30.0), // text isn't very high
            },
        }
    }

    /// Specify custom formatter for ticks.
    ///
    /// The first parameter of `formatter` is the raw tick value as `f64`.
    /// The second parameter is the maximum number of characters that fit into y-labels.
    /// The second parameter of `formatter` is the currently shown range on this axis.
    pub fn formatter(
        mut self,
        fmt: impl Fn(GridMark, usize, &RangeInclusive<f64>) -> String + 'static,
    ) -> Self {
        self.formatter = Arc::new(fmt);
        self
    }

    fn default_formatter(
        mark: GridMark,
        max_digits: usize,
        _range: &RangeInclusive<f64>,
    ) -> String {
        let tick = mark.value;

        if tick.abs() > 10.0_f64.powf(max_digits as f64) {
            let tick_rounded = tick as isize;
            return format!("{tick_rounded:+e}");
        }
        let tick_rounded = round_to_decimals(tick, max_digits);
        if tick.abs() < 10.0_f64.powf(-(max_digits as f64)) && tick != 0.0 {
            return format!("{tick_rounded:+e}");
        }
        tick_rounded.to_string()
    }

    /// Specify axis label.
    ///
    /// The default is 'x' for x-axes and 'y' for y-axes.
    #[inline]
    pub fn label(mut self, label: impl Into<WidgetText>) -> Self {
        self.label = label.into();
        self
    }

    /// Specify maximum number of digits for ticks.
    ///
    /// This is considered by the default tick formatter and affects the width of the y-axis
    #[inline]
    pub fn max_digits(mut self, digits: usize) -> Self {
        self.digits = digits;
        self
    }

    /// Specify the placement of the axis.
    ///
    /// For X-axis, use [`VPlacement`].
    /// For Y-axis, use [`HPlacement`].
    #[inline]
    pub fn placement(mut self, placement: impl Into<Placement>) -> Self {
        self.placement = placement.into();
        self
    }

    /// Set the minimum spacing between labels
    ///
    /// When labels get closer together than the given minimum, then they become invisible.
    /// When they get further apart than the max, they are at full opacity.
    ///
    /// Labels can never be closer together than the [`crate::Plot::grid_spacing`] setting.
    #[inline]
    pub fn label_spacing(mut self, range: impl Into<Rangef>) -> Self {
        self.label_spacing = range.into();
        self
    }

    pub(super) fn thickness(&self, axis: Axis) -> f32 {
        match axis {
            Axis::X => {
                if self.label.is_empty() {
                    1.0 * LINE_HEIGHT
                } else {
                    3.0 * LINE_HEIGHT
                }
            }
            Axis::Y => {
                if self.label.is_empty() {
                    (self.digits as f32) * LINE_HEIGHT
                } else {
                    (self.digits as f32 + 1.0) * LINE_HEIGHT
                }
            }
        }
    }
}

#[derive(Clone)]
pub(super) struct AxisWidget {
    pub range: RangeInclusive<f64>,
    pub hints: AxisHints,

    /// The region where we draw the axis labels.
    pub rect: Rect,
    pub transform: Option<PlotTransform>,
    pub steps: Arc<Vec<GridMark>>,
}

impl AxisWidget {
    /// if `rect` as width or height == 0, is will be automatically calculated from ticks and text.
    pub fn new(hints: AxisHints, rect: Rect) -> Self {
        Self {
            range: (0.0..=0.0),
            hints,
            rect,
            transform: None,
            steps: Default::default(),
        }
    }

    /// Returns the actual thickness of the axis.
    pub fn ui(self, ui: &mut Ui, axis: Axis) -> (Response, f32) {
        let response = ui.allocate_rect(self.rect, Sense::hover());

        if !ui.is_rect_visible(response.rect) {
            return (response, 0.0);
        }

        let visuals = ui.style().visuals.clone();

        {
            let text = self.hints.label;
            let galley = text.into_galley(ui, Some(false), f32::INFINITY, TextStyle::Body);
            let text_color = visuals
                .override_text_color
                .unwrap_or_else(|| ui.visuals().text_color());
            let angle: f32 = match axis {
                Axis::X => 0.0,
                Axis::Y => -std::f32::consts::TAU * 0.25,
            };
            // select text_pos and angle depending on placement and orientation of widget
            let text_pos = match self.hints.placement {
                Placement::LeftBottom => match axis {
                    Axis::X => {
                        let pos = response.rect.center_bottom();
                        Pos2 {
                            x: pos.x - galley.size().x / 2.0,
                            y: pos.y - galley.size().y * 1.25,
                        }
                    }
                    Axis::Y => {
                        let pos = response.rect.left_center();
                        Pos2 {
                            x: pos.x,
                            y: pos.y + galley.size().x / 2.0,
                        }
                    }
                },
                Placement::RightTop => match axis {
                    Axis::X => {
                        let pos = response.rect.center_top();
                        Pos2 {
                            x: pos.x - galley.size().x / 2.0,
                            y: pos.y + galley.size().y * 0.25,
                        }
                    }
                    Axis::Y => {
                        let pos = response.rect.right_center();
                        Pos2 {
                            x: pos.x - galley.size().y * 1.5,
                            y: pos.y + galley.size().x / 2.0,
                        }
                    }
                },
            };

            ui.painter()
                .add(TextShape::new(text_pos, galley, text_color).with_angle(angle));
        }

        let font_id = TextStyle::Body.resolve(ui.style());
        let Some(transform) = self.transform else {
            return (response, 0.0);
        };

        let label_spacing = self.hints.label_spacing;

        let mut thickness: f32 = 0.0;

        // Add tick labels:
        for step in self.steps.iter() {
            let text = (self.hints.formatter)(*step, self.hints.digits, &self.range);
            if !text.is_empty() {
                let spacing_in_points =
                    (transform.dpos_dvalue()[usize::from(axis)] * step.step_size).abs() as f32;

                if spacing_in_points <= label_spacing.min {
                    // Labels are too close together - don't paint them.
                    continue;
                }

                // Fade in labels as they get further apart:
                let strength = remap_clamp(spacing_in_points, label_spacing, 0.0..=1.0);

                let text_color = super::color_from_strength(ui, strength);
                let galley = ui
                    .painter()
                    .layout_no_wrap(text, font_id.clone(), text_color);

                if spacing_in_points < galley.size()[axis as usize] {
                    continue; // the galley won't fit (likely too wide on the X axis).
                }

                match axis {
                    Axis::X => {
                        thickness = thickness.max(galley.size().y);

                        let projected_point = super::PlotPoint::new(step.value, 0.0);
                        let center_x = transform.position_from_point(&projected_point).x;
                        let y = match VPlacement::from(self.hints.placement) {
                            VPlacement::Bottom => self.rect.min.y,
                            VPlacement::Top => self.rect.max.y - galley.size().y,
                        };
                        let pos = Pos2::new(center_x - galley.size().x / 2.0, y);
                        ui.painter().add(TextShape::new(pos, galley, text_color));
                    }
                    Axis::Y => {
                        thickness = thickness.max(galley.size().x);

                        let projected_point = super::PlotPoint::new(0.0, step.value);
                        let center_y = transform.position_from_point(&projected_point).y;

                        match HPlacement::from(self.hints.placement) {
                            HPlacement::Left => {
                                let angle = 0.0; // TODO(emilk): allow users to rotate text

                                if angle == 0.0 {
                                    let x = self.rect.max.x - galley.size().x;
                                    let pos = Pos2::new(x, center_y - galley.size().y / 2.0);
                                    ui.painter().add(TextShape::new(pos, galley, text_color));
                                } else {
                                    let right = Pos2::new(
                                        self.rect.max.x,
                                        center_y - galley.size().y / 2.0,
                                    );
                                    let width = galley.size().x;
                                    let left =
                                        right - Rot2::from_angle(angle) * Vec2::new(width, 0.0);

                                    ui.painter().add(
                                        TextShape::new(left, galley, text_color).with_angle(angle),
                                    );
                                }
                            }
                            HPlacement::Right => {
                                let x = self.rect.min.x;
                                let pos = Pos2::new(x, center_y - galley.size().y / 2.0);
                                ui.painter().add(TextShape::new(pos, galley, text_color));
                            }
                        };
                    }
                };
            }
        }

        (response, thickness)
    }
}
