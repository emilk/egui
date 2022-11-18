use std::{
    fmt::{Debug, Formatter},
    ops::RangeInclusive,
};

use epaint::{
    emath::{remap_clamp, round_to_decimals, NumExt},
    Color32, Pos2, Rect, Rgba, Shape, Stroke, TextShape,
};

use crate::{Response, Sense, TextStyle, Ui, Widget, WidgetText};

use super::{transform::PlotTransform, GridMark, MIN_LINE_SPACING_IN_POINTS};

pub(super) type AxisFormatterFn = fn(f64, usize, &RangeInclusive<f64>) -> String;

/// Generic constant for x-Axis
pub(super) const X_AXIS: usize = 0;
/// Generic constant for y-Axis
pub(super) const Y_AXIS: usize = 1;

/// Placement of an Axis.
///
/// `Default` means bottom for x-axis and left for y-axis.
/// `Opposite` means top for x-axis and right for y-axis.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Placement {
    Default,
    Opposite,
}

// shorthand types for AxisHints, public API
/// Configuration for x-axis
pub type XAxisHints = AxisHints<X_AXIS>;
/// Configuration for y-axis
pub type YAxisHints = AxisHints<Y_AXIS>;

// shorthand types for AxisWidget
pub(super) type XAxisWidget = AxisWidget<X_AXIS>;
pub(super) type YAxisWidget = AxisWidget<Y_AXIS>;
/// Axis configuration.
///
/// Used to configure axis label and ticks.
#[derive(Clone)]
pub struct AxisHints<const AXIS: usize> {
    pub(super) label: String,
    pub(super) formatter: AxisFormatterFn,
    digits: usize,
    pub(super) placement: Placement,
}

impl<const AXIS: usize> Debug for AxisHints<AXIS> {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        let axis_str = match AXIS {
            X_AXIS => "x-axis",
            Y_AXIS => "y-axis",
            _ => unreachable!(),
        };
        write!(
            fmt,
            "Axis ( placement: {:?}, label: {}, formatter: ???, axis: {} )",
            self.placement, self.label, axis_str
        )
    }
}

// TODO: this just a guess. It might cease to work if a user changes font size.
const LINE_HEIGHT: f32 = 12.0;

impl<const AXIS: usize> Default for AxisHints<AXIS> {
    /// Initializes a default axis configuration for the specified [`Axis`].
    ///
    /// `label` is 'x' or 'y'
    /// `formatter` is default float to string formatter
    /// maximum `digits` on tick label is 5
    fn default() -> Self {
        let label = match AXIS {
            X_AXIS => "x".to_string(),
            Y_AXIS => "y".to_string(),
            _ => unreachable!(),
        };
        Self {
            label,
            formatter: Self::default_formatter,
            digits: 5,
            placement: Placement::Default,
        }
    }
}

impl<const AXIS: usize> AxisHints<AXIS> {
    /// Specify custom formatter for ticks.
    ///
    /// The first parameter of `formatter` is the raw tick value as `f64`.
    /// The second paramter is the maximum number of characters that fit into y-labels.
    /// The second paramter of `formatter` is the currently shown range on this axis.
    pub fn formatter(mut self, fmt: fn(f64, usize, &RangeInclusive<f64>) -> String) -> Self {
        self.formatter = fmt;
        self
    }

    fn default_formatter(tick: f64, max_digits: usize, _range: &RangeInclusive<f64>) -> String {
        if tick.abs() > 10.0_f64.powf(max_digits as f64) {
            let tick_rounded = tick as isize;
            return format!("{:+e}", tick_rounded);
        }
        let tick_rounded = round_to_decimals(tick, max_digits);
        if tick.abs() < 10.0_f64.powf(-(max_digits as f64)) && tick != 0.0 {
            return format!("{:+e}", tick_rounded);
        }
        format!("{}", tick_rounded)
    }

    /// Specify axis label.
    ///
    /// The default is 'x' for x-axes and 'y' for y-axes.
    pub fn label(mut self, label: String) -> Self {
        self.label = label;
        self
    }

    /// Specify maximum number of digits for ticks.
    ///
    /// This is considered by the default tick formatter
    /// and affects the width of the internal y-axis widget
    pub fn max_digits(mut self, digits: usize) -> Self {
        self.digits = digits;
        self
    }

    /// Specify the placement of the axis.
    pub fn placement(mut self, placement: Placement) -> Self {
        self.placement = placement;
        self
    }

    pub(super) fn thickness(&self) -> f32 {
        match AXIS {
            X_AXIS => {
                if self.label.is_empty() {
                    1.0 * LINE_HEIGHT
                } else {
                    3.0 * LINE_HEIGHT
                }
            }
            Y_AXIS => {
                if self.label.is_empty() {
                    (self.digits as f32) * LINE_HEIGHT
                } else {
                    (self.digits as f32 + 1.0) * LINE_HEIGHT
                }
            }
            _ => unreachable!(),
        }
    }
}

#[derive(Clone)]
pub(super) struct AxisWidget<const AXIS: usize> {
    pub(super) range: RangeInclusive<f64>,
    pub(super) hints: AxisHints<AXIS>,
    pub(super) rect: Rect,
    pub(super) transform: Option<PlotTransform>,
    pub(super) steps: Vec<GridMark>,
}

impl<const AXIS: usize> AxisWidget<AXIS> {
    /// if `rect` as width or height == 0, is will be automatically calculated from ticks and text.
    pub(super) fn new(hints: AxisHints<AXIS>, rect: Rect) -> Self {
        Self {
            range: (0.0..=0.0),
            hints,
            rect,
            transform: None,
            steps: Vec::new(),
        }
    }
}

impl<const AXIS: usize> Widget for AxisWidget<AXIS> {
    fn ui(self, ui: &mut Ui) -> Response {
        // --- add label ---
        let response = ui.allocate_rect(self.rect, Sense::click_and_drag());
        if ui.is_rect_visible(response.rect) {
            let visuals = ui.style().visuals.clone();
            let text: WidgetText = self.hints.label.into();
            let galley = text.into_galley(ui, Some(false), f32::INFINITY, TextStyle::Body);
            let text_color = visuals
                .override_text_color
                .unwrap_or_else(|| ui.visuals().text_color());
            let angle: f32 = match AXIS {
                X_AXIS => 0.0,
                Y_AXIS => -std::f32::consts::PI * 0.5,
                _ => unreachable!(),
            };
            // select text_pos and angle depending on placement and orientation of widget
            let text_pos = match self.hints.placement {
                Placement::Default => match AXIS {
                    X_AXIS => {
                        let pos = response.rect.center_bottom();
                        Pos2 {
                            x: pos.x - galley.size().x / 2.0,
                            y: pos.y - galley.size().y * 1.25,
                        }
                    }
                    Y_AXIS => {
                        let pos = response.rect.left_center();
                        Pos2 {
                            x: pos.x,
                            y: pos.y + galley.size().x / 2.0,
                        }
                    }
                    _ => unreachable!(),
                },
                Placement::Opposite => match AXIS {
                    X_AXIS => {
                        let pos = response.rect.center_top();
                        Pos2 {
                            x: pos.x - galley.size().x / 2.0,
                            y: pos.y + galley.size().y * 0.25,
                        }
                    }
                    Y_AXIS => {
                        let pos = response.rect.right_center();
                        Pos2 {
                            x: pos.x - galley.size().y * 1.5,
                            y: pos.y + galley.size().x / 2.0,
                        }
                    }
                    _ => unreachable!(),
                },
            };
            let shape = TextShape {
                pos: text_pos,
                galley: galley.galley,
                underline: Stroke::NONE,
                override_text_color: Some(text_color),
                angle,
            };
            ui.painter().add(shape);

            // --- add ticks ---
            let font_id = TextStyle::Body.resolve(ui.style());
            let transform = match self.transform {
                Some(t) => t,
                None => return response,
            };

            for step in self.steps {
                let text = (self.hints.formatter)(step.value, self.hints.digits, &self.range);
                if !text.is_empty() {
                    let spacing_in_points =
                        (transform.dpos_dvalue()[AXIS] * step.step_size).abs() as f32;

                    let line_alpha = remap_clamp(
                        spacing_in_points,
                        (MIN_LINE_SPACING_IN_POINTS as f32 * 2.0)..=300.0,
                        0.0..=0.15,
                    );

                    if line_alpha > 0.0 {
                        let line_color = color_from_alpha(ui, line_alpha);
                        let galley = ui
                            .painter()
                            .layout_no_wrap(text, font_id.clone(), line_color);
                        let text_pos = match AXIS {
                            X_AXIS => {
                                let y = match self.hints.placement {
                                    Placement::Default => self.rect.min.y,
                                    Placement::Opposite => self.rect.max.y - galley.size().y,
                                };
                                let projected_point = super::PlotPoint::new(step.value, 0.0);
                                Pos2 {
                                    x: transform.position_from_point(&projected_point).x
                                        - galley.size().x / 2.0,
                                    y,
                                }
                            }
                            Y_AXIS => {
                                let x = match self.hints.placement {
                                    Placement::Default => self.rect.max.x - galley.size().x,
                                    Placement::Opposite => self.rect.min.x,
                                };
                                let projected_point = super::PlotPoint::new(0.0, step.value);
                                Pos2 {
                                    x,
                                    y: transform.position_from_point(&projected_point).y
                                        - galley.size().y / 2.0,
                                }
                            }
                            _ => unreachable!(),
                        };

                        ui.painter().add(Shape::galley(text_pos, galley));
                    }
                }
            }
        }
        response
    }
}

fn color_from_alpha(ui: &Ui, alpha: f32) -> Color32 {
    if ui.visuals().dark_mode {
        Rgba::from_white_alpha(alpha).into()
    } else {
        Rgba::from_black_alpha((4.0 * alpha).at_most(1.0)).into()
    }
}
