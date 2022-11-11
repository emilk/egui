use std::{
    fmt::{Debug, Formatter},
    ops::RangeInclusive,
};

use epaint::{Pos2, Rect, Stroke, TextShape};

use crate::{Response, Sense, TextStyle, Ui, Widget, WidgetText};

pub(super) type AxisFormatterFn = fn(f64, &RangeInclusive<f64>) -> String;

/// Axis specifier.
///
/// Used to specify which kind of axis an [`AxisConfig`] refers to.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Axis {
    X = 0,
    Y = 1,
}

/// Placement configuration for an axis.
///
/// `Default` means bottom for x, left for y.
/// `Opposite` means top for x, right for y.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Placement {
    Default,
    Opposite,
}

/// Axis configuration.
///
/// Used to configure axis label and ticks.
#[derive(Clone)]
pub struct AxisConfig {
    pub(super) placement: Placement,
    label: String,
    pub(super) formatter: AxisFormatterFn,
    pub(super) axis: Axis,
}

impl Debug for AxisConfig {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(
            fmt,
            "AxisConfig ( placement: {:?}, label: {}, formatter: ???, axis: {:?} )",
            self.placement, self.label, self.axis
        )
    }
}
const LINE_HEIGHT: f32 = 12.0;

impl AxisConfig {
    /// Initializes a default axis configuration for the specified [`Axis`].
    ///
    /// `placement` is bottom for x-axes and left for y-axes
    /// `label` is empty
    /// `formatter` is default float to string formatter
    pub const fn default(axis: Axis) -> Self {
        Self {
            placement: Placement::Default,
            label: String::new(),
            formatter: Self::default_formatter,
            axis,
        }
    }

    /// Specify axis label
    pub fn label(mut self, label: String) -> Self {
        self.label = label;
        self
    }

    /// Specify custom formatter for ticks.
    ///
    /// The first parameter of `formatter` is the raw tick value as `f64`.
    /// The second paramter of `formatter` is the currently shown range on this axis.
    pub fn tick_formatter(mut self, formatter: fn(f64, &RangeInclusive<f64>) -> String) -> Self {
        self.formatter = formatter;
        self
    }

    /// Specify the placement for this axis.
    pub fn placement(mut self, placement: Placement) -> Self {
        self.placement = placement;
        self
    }

    fn default_formatter(tick: f64, _range: &RangeInclusive<f64>) -> String {
        tick.to_string()
    }

    pub(super) fn thickness(&self) -> f32 {
        if self.label.is_empty() {
            LINE_HEIGHT
        } else {
            2.0 * LINE_HEIGHT
        }
    }
}

#[derive(Clone)]
pub(super) struct AxisWidget {
    config: AxisConfig,
    pub(super) rect: Rect,
}

impl AxisWidget {
    /// if `rect` as width or height == 0, is will be automatically calculated from ticks and text.
    pub(super) fn new(config: AxisConfig, rect: Rect) -> Self {
        Self { config, rect }
    }

    // fn calc_size(&mut self) {
    //     if self.rect.height() == 0.0 {
    //         if self.config.axis == Axis::X {
    //             // calculate height of x-axis label: ticks + label
    //             let y = self.rect.min.y;
    //             if self.config.label.is_empty() {
    //                 self.rect.extend_with_y(y + LINE_HEIGHT);
    //             }
    //             else {
    //                 self.rect.extend_with_y(y + 2.0*LINE_HEIGHT)
    //             }
    //         }
    //     }
    //     if self.rect.width() == 0.0 {
    //         if self.config.axis == Axis::Y {
    //             // calculate width of y-axis label: ticks + label
    //             if self.config.label.is_empty() {
    //                 self.rect.extend_with_x(50.0);
    //             }
    //             else {
    //                 self.rect.extend_with_x(100.0)
    //             }
    //         }
    //     }
    // }
    // pub(super) fn exact_size(mut self, rect: Rect) -> Self  {
    //     self.rect = rect;
    //     self
    // }
}

impl Widget for AxisWidget {
    fn ui(self, ui: &mut Ui) -> Response {
        let response = ui.allocate_rect(self.rect, Sense::click_and_drag());
        if ui.is_rect_visible(response.rect) {
            let visuals = ui.style().visuals.clone();
            let text: WidgetText = self.config.label.into();
            let galley = text.into_galley(ui, Some(false), f32::INFINITY, TextStyle::Body);
            let text_color = visuals
                .override_text_color
                .unwrap_or(ui.visuals().text_color());
            let angle: f32 = match self.config.axis {
                Axis::X => 0.0,
                Axis::Y => -std::f32::consts::PI * 0.5,
            };
            // select text_pos and angle depending on placement and orientation of widget
            let text_pos = match self.config.placement {
                Placement::Default => match self.config.axis {
                    Axis::X => {
                        let pos = response.rect.center_bottom();
                        Pos2 {
                            x: pos.x - galley.size().x / 2.0,
                            y: pos.y - galley.size().y,
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
                Placement::Opposite => match self.config.axis {
                    Axis::X => {
                        let pos = response.rect.center_top();
                        Pos2 {
                            x: pos.x - galley.size().x / 2.0,
                            y: pos.y + galley.size().y / 2.0,
                        }
                    }
                    Axis::Y => {
                        let pos = response.rect.right_center();
                        Pos2 {
                            x: pos.x - galley.size().y,
                            y: pos.y + galley.size().x / 2.0,
                        }
                    }
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
        }
        response
    }
}
