#![allow(clippy::float_cmp)]

use std::ops::RangeInclusive;

use crate::{widgets::Label, *};

// ----------------------------------------------------------------------------

/// Combined into one function (rather than two) to make it easier
/// for the borrow checker.
type GetSetValue<'a> = Box<dyn 'a + FnMut(Option<f64>) -> f64>;

fn get(value_function: &mut GetSetValue<'_>) -> f64 {
    (value_function)(None)
}

fn set(value_function: &mut GetSetValue<'_>, value: f64) {
    (value_function)(Some(value));
}

fn to_f64_range<T: Copy>(r: RangeInclusive<T>) -> RangeInclusive<f64>
where
    f64: From<T>,
{
    f64::from(*r.start())..=f64::from(*r.end())
}

// ----------------------------------------------------------------------------

#[derive(Clone)]
struct SliderSpec {
    logarithmic: bool,
    /// For logarithmic sliders, the smallest positive value we are interested in.
    /// 1 for integer sliders, maybe 1e-6 for others.
    smallest_positive: f64,
}

/// Control a number by a horizontal slider.
///
/// The slider range defines the values you get when pulling the slider to the far edges.
/// By default, the slider can still show values outside this range,
/// and still allows users to enter values outside the range by clicking the slider value and editing it.
/// If you want to clamp incoming and outgoing values, use [`Slider::clamp_to_range`].
///
/// The range can include any numbers, and go from low-to-high or from high-to-low.
#[must_use = "You should put this widget in an ui with `ui.add(widget);`"]
pub struct Slider<'a> {
    get_set_value: GetSetValue<'a>,
    range: RangeInclusive<f64>,
    spec: SliderSpec,
    clamp_to_range: bool,
    smart_aim: bool,
    // TODO: label: Option<Label>
    text: Option<String>,
    text_color: Option<Color32>,
    min_decimals: usize,
    max_decimals: Option<usize>,
}

impl<'a> Slider<'a> {
    pub fn from_get_set(
        range: RangeInclusive<f64>,
        get_set_value: impl 'a + FnMut(Option<f64>) -> f64,
    ) -> Self {
        Self {
            get_set_value: Box::new(get_set_value),
            range,
            spec: SliderSpec {
                logarithmic: false,
                smallest_positive: 1e-6,
            },
            clamp_to_range: false,
            smart_aim: true,
            text: None,
            text_color: None,
            min_decimals: 0,
            max_decimals: None,
        }
    }

    pub fn f32(value: &'a mut f32, range: RangeInclusive<f32>) -> Self {
        Self::from_get_set(to_f64_range(range), move |v: Option<f64>| {
            if let Some(v) = v {
                *value = v as f32
            }
            *value as f64
        })
    }

    pub fn f64(value: &'a mut f64, range: RangeInclusive<f64>) -> Self {
        Self::from_get_set(to_f64_range(range), move |v: Option<f64>| {
            if let Some(v) = v {
                *value = v
            }
            *value
        })
    }

    pub fn u8(value: &'a mut u8, range: RangeInclusive<u8>) -> Self {
        Self::from_get_set(to_f64_range(range), move |v: Option<f64>| {
            if let Some(v) = v {
                *value = v.round() as u8
            }
            *value as f64
        })
        .integer()
    }

    pub fn i32(value: &'a mut i32, range: RangeInclusive<i32>) -> Self {
        Self::from_get_set(to_f64_range(range), move |v: Option<f64>| {
            if let Some(v) = v {
                *value = v.round() as i32
            }
            *value as f64
        })
        .integer()
    }

    pub fn u32(value: &'a mut u32, range: RangeInclusive<u32>) -> Self {
        Self::from_get_set(to_f64_range(range), move |v: Option<f64>| {
            if let Some(v) = v {
                *value = v.round() as u32
            }
            *value as f64
        })
        .integer()
    }

    pub fn usize(value: &'a mut usize, range: RangeInclusive<usize>) -> Self {
        let range = (*range.start() as f64)..=(*range.end() as f64);
        Self::from_get_set(range, move |v: Option<f64>| {
            if let Some(v) = v {
                *value = v.round() as usize
            }
            *value as f64
        })
        .integer()
    }

    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }

    pub fn text_color(mut self, text_color: Color32) -> Self {
        self.text_color = Some(text_color);
        self
    }

    /// Make this a logarithmic slider.
    /// This is great for when the slider spans a huge range,
    /// e.g. from one to a million.
    /// The default is OFF.
    pub fn logarithmic(mut self, logarithmic: bool) -> Self {
        self.spec.logarithmic = logarithmic;
        self
    }

    /// For logarithmic sliders that includes zero:
    /// what is the smallest positive value you want to be able to select?
    /// The default is `1` for integer sliders and `1e-6` for real sliders.
    pub fn smallest_positive(mut self, smallest_positive: f64) -> Self {
        self.spec.smallest_positive = smallest_positive;
        self
    }

    /// If set to `true`, all incoming and outgoing values will be clamped to the slider range.
    /// Default: `false`.
    pub fn clamp_to_range(mut self, clamp_to_range: bool) -> Self {
        self.clamp_to_range = clamp_to_range;
        self
    }

    /// Turn smart aim on/off. Default is ON.
    /// There is almost no point in turning this off.
    pub fn smart_aim(mut self, smart_aim: bool) -> Self {
        self.smart_aim = smart_aim;
        self
    }

    #[deprecated = "Use fixed_decimals instead"]
    pub fn precision(self, precision: usize) -> Self {
        self.max_decimals(precision)
    }

    // TODO: we should also have a "min precision".
    /// Set a minimum number of decimals to display.
    /// Normally you don't need to pick a precision, as the slider will intelligently pick a precision for you.
    /// Regardless of precision the slider will use "smart aim" to help the user select nice, round values.
    pub fn min_decimals(mut self, min_decimals: usize) -> Self {
        self.min_decimals = min_decimals;
        self
    }

    // TODO: we should also have a "max precision".
    /// Set a maximum number of decimals to display.
    /// Values will also be rounded to this number of decimals.
    /// Normally you don't need to pick a precision, as the slider will intelligently pick a precision for you.
    /// Regardless of precision the slider will use "smart aim" to help the user select nice, round values.
    pub fn max_decimals(mut self, max_decimals: usize) -> Self {
        self.max_decimals = Some(max_decimals);
        self
    }

    /// Set an exact number of decimals to display.
    /// Values will also be rounded to this number of decimals.
    /// Normally you don't need to pick a precision, as the slider will intelligently pick a precision for you.
    /// Regardless of precision the slider will use "smart aim" to help the user select nice, round values.
    pub fn fixed_decimals(mut self, num_decimals: usize) -> Self {
        self.min_decimals = num_decimals;
        self.max_decimals = Some(num_decimals);
        self
    }

    /// Helper: equivalent to `self.precision(0).smallest_positive(1.0)`.
    /// If you use one of the integer constructors (e.g. `Slider::i32`) this is called for you,
    /// but if you want to have a slider for picking integer values in an `Slider::f64`, use this.
    pub fn integer(self) -> Self {
        self.fixed_decimals(0).smallest_positive(1.0)
    }

    fn get_value(&mut self) -> f64 {
        let value = get(&mut self.get_set_value);
        if self.clamp_to_range {
            clamp(value, self.range.clone())
        } else {
            value
        }
    }

    fn set_value(&mut self, mut value: f64) {
        if self.clamp_to_range {
            value = clamp(value, self.range.clone());
        }
        if let Some(max_decimals) = self.max_decimals {
            value = math::round_to_decimals(value, max_decimals);
        }
        set(&mut self.get_set_value, value);
    }

    fn range(&self) -> RangeInclusive<f64> {
        self.range.clone()
    }

    /// For instance, `x` is the mouse position and `x_range` is the physical location of the slider on the screen.
    fn value_from_x(&self, x: f32, x_range: RangeInclusive<f32>) -> f64 {
        let normalized = remap_clamp(x, x_range, 0.0..=1.0) as f64;
        value_from_normalized(normalized, self.range(), &self.spec)
    }

    fn x_from_value(&self, value: f64, x_range: RangeInclusive<f32>) -> f32 {
        let normalized = normalized_from_value(value, self.range(), &self.spec);
        lerp(x_range, normalized as f32)
    }
}

fn handle_radius(rect: &Rect) -> f32 {
    rect.height() / 2.5
}

fn x_range(rect: &Rect) -> RangeInclusive<f32> {
    let handle_radius = handle_radius(rect);
    (rect.left() + handle_radius)..=(rect.right() - handle_radius)
}

impl<'a> Slider<'a> {
    /// Just the slider, no text
    fn allocate_slider_space(&self, ui: &mut Ui, height: f32) -> Response {
        let desired_size = vec2(ui.style().spacing.slider_width, height);
        ui.allocate_response(desired_size, Sense::click_and_drag())
    }

    /// Just the slider, no text
    fn slider_ui(&mut self, ui: &mut Ui, response: &Response) {
        let rect = &response.rect;
        let x_range = x_range(rect);

        if let Some(pointer_pos) = response.interact_pointer_pos() {
            let new_value = if self.smart_aim {
                let aim_radius = ui.input().aim_radius();
                crate::math::smart_aim::best_in_range_f64(
                    self.value_from_x(pointer_pos.x - aim_radius, x_range.clone()),
                    self.value_from_x(pointer_pos.x + aim_radius, x_range.clone()),
                )
            } else {
                self.value_from_x(pointer_pos.x, x_range.clone())
            };
            self.set_value(new_value);
        }

        // Paint it:
        {
            let value = self.get_value();

            let rail_radius = ui.painter().round_to_pixel((rect.height() / 8.0).max(2.0));
            let rail_rect = Rect::from_min_max(
                pos2(rect.left(), rect.center().y - rail_radius),
                pos2(rect.right(), rect.center().y + rail_radius),
            );
            let marker_center_x = self.x_from_value(value, x_range);

            let visuals = ui.style().interact(response);
            ui.painter().add(Shape::Rect {
                rect: rail_rect,
                corner_radius: rail_radius,

                fill: ui.style().visuals.widgets.inactive.bg_fill,
                // fill: visuals.bg_fill,
                // fill: ui.style().visuals.dark_bg_color,
                stroke: Default::default(),
                // stroke: visuals.bg_stroke,
                // stroke: ui.style().visuals.widgets.inactive.bg_stroke,
            });

            ui.painter().add(Shape::Circle {
                center: pos2(marker_center_x, rail_rect.center().y),
                radius: handle_radius(rect) + visuals.expansion,
                fill: visuals.bg_fill,
                stroke: visuals.fg_stroke,
            });
        }
    }

    fn label_ui(&mut self, ui: &mut Ui) {
        if let Some(label_text) = self.text.as_deref() {
            let text_color = self
                .text_color
                .unwrap_or_else(|| ui.style().visuals.text_color());

            ui.add(
                Label::new(label_text)
                    .multiline(false)
                    .text_color(text_color),
            );
        }
    }

    fn value_ui(&mut self, ui: &mut Ui, x_range: RangeInclusive<f32>) {
        let kb_edit_id = ui.auto_id_with("edit");
        let is_kb_editing = ui.memory().has_kb_focus(kb_edit_id);

        let aim_radius = ui.input().aim_radius();
        let value_text = self.format_value(aim_radius, x_range);

        if is_kb_editing {
            let button_width = ui.style().spacing.interact_size.x;
            let mut value_text = ui.memory().temp_edit_string.take().unwrap_or(value_text);
            ui.add(
                TextEdit::singleline(&mut value_text)
                    .id(kb_edit_id)
                    .desired_width(button_width)
                    .text_color_opt(self.text_color)
                    .text_style(TextStyle::Monospace),
            );
            if let Ok(value) = value_text.parse() {
                self.set_value(value);
            }
            if ui.input().key_pressed(Key::Enter) {
                ui.memory().surrender_kb_focus(kb_edit_id);
            } else {
                ui.memory().temp_edit_string = Some(value_text);
            }
        } else {
            let response = ui.add(
                Button::new(value_text)
                    .text_style(TextStyle::Monospace)
                    .text_color_opt(self.text_color),
            );
            let response = response.on_hover_text(format!(
                "{}\nClick to enter a value.",
                self.get_value() as f32 // Show full precision value on-hover. TODO: figure out f64 vs f32
            ));
            // let response = ui.interact(response.rect, kb_edit_id, Sense::click());
            if response.clicked() {
                ui.memory().request_kb_focus(kb_edit_id);
                ui.memory().temp_edit_string = None; // Filled in next frame
            }
        }
    }

    fn format_value(&mut self, aim_radius: f32, x_range: RangeInclusive<f32>) -> String {
        let value = self.get_value();

        // pick precision based upon how much moving the slider would change the value:
        let value_from_x = |x: f32| self.value_from_x(x, x_range.clone());
        let x_from_value = |value: f64| self.x_from_value(value, x_range.clone());
        let left_value = value_from_x(x_from_value(value) - aim_radius);
        let right_value = value_from_x(x_from_value(value) + aim_radius);
        let range = (left_value - right_value).abs();
        let auto_decimals = ((-range.log10()).ceil().at_least(0.0) as usize).at_most(16);
        let min_decimals = self.min_decimals;
        let max_decimals = self.max_decimals.unwrap_or(auto_decimals + 2);

        let auto_decimals = clamp(auto_decimals, min_decimals..=max_decimals);

        if min_decimals == max_decimals {
            math::format_with_minimum_decimals(value, max_decimals)
        } else if value == 0.0 {
            "0".to_owned()
        } else if range == 0.0 {
            value.to_string()
        } else {
            math::format_with_decimals_in_range(value, auto_decimals..=max_decimals)
        }
    }
}

impl<'a> Widget for Slider<'a> {
    fn ui(mut self, ui: &mut Ui) -> Response {
        let text_style = TextStyle::Button;
        let font = &ui.fonts()[text_style];
        let height = font
            .row_height()
            .at_least(ui.style().spacing.interact_size.y);

        if self.text.is_some() {
            ui.horizontal(|ui| {
                let slider_response = self.allocate_slider_space(ui, height);
                self.slider_ui(ui, &slider_response);
                let x_range = x_range(&slider_response.rect);
                self.value_ui(ui, x_range);
                self.label_ui(ui);
                slider_response
            })
            .0
        } else {
            let response = self.allocate_slider_space(ui, height);
            self.slider_ui(ui, &response);
            response
        }
    }
}

// ----------------------------------------------------------------------------
// Helpers for converting slider range to/from normalized [0-1] range.
// Always clamps.
// Logarithmic sliders are allowed to include zero and infinity,
// even though mathematically it doesn't make sense.

use std::f64::INFINITY;

/// When the user asks for an infinitely large range (e.g. logarithmic from zero),
/// give a scale that this many orders of magnitude in size.
const INF_RANGE_MAGNITUDE: f64 = 10.0;

fn value_from_normalized(normalized: f64, range: RangeInclusive<f64>, spec: &SliderSpec) -> f64 {
    let (min, max) = (*range.start(), *range.end());

    if min.is_nan() || max.is_nan() {
        f64::NAN
    } else if min == max {
        min
    } else if min > max {
        value_from_normalized(1.0 - normalized, max..=min, spec)
    } else if normalized <= 0.0 {
        min
    } else if normalized >= 1.0 {
        max
    } else if spec.logarithmic {
        if max <= 0.0 {
            // non-positive range
            -value_from_normalized(normalized, -min..=-max, spec)
        } else if 0.0 <= min {
            let (min_log, max_log) = range_log10(min, max, spec);
            let log = lerp(min_log..=max_log, normalized);
            10.0_f64.powf(log)
        } else {
            assert!(min < 0.0 && 0.0 < max);
            let zero_cutoff = logaritmic_zero_cutoff(min, max);
            if normalized < zero_cutoff {
                // negative
                value_from_normalized(
                    remap(normalized, 0.0..=zero_cutoff, 0.0..=1.0),
                    min..=0.0,
                    spec,
                )
            } else {
                // positive
                value_from_normalized(
                    remap(normalized, zero_cutoff..=1.0, 0.0..=1.0),
                    0.0..=max,
                    spec,
                )
            }
        }
    } else {
        debug_assert!(
            min.is_finite() && max.is_finite(),
            "You should use a logarithmic range"
        );
        lerp(range, clamp(normalized, 0.0..=1.0))
    }
}

fn normalized_from_value(value: f64, range: RangeInclusive<f64>, spec: &SliderSpec) -> f64 {
    let (min, max) = (*range.start(), *range.end());

    if min.is_nan() || max.is_nan() {
        f64::NAN
    } else if min == max {
        0.5 // empty range, show center of slider
    } else if min > max {
        1.0 - normalized_from_value(value, max..=min, spec)
    } else if value <= min {
        0.0
    } else if value >= max {
        1.0
    } else if spec.logarithmic {
        if max <= 0.0 {
            // non-positive range
            normalized_from_value(-value, -min..=-max, spec)
        } else if 0.0 <= min {
            let (min_log, max_log) = range_log10(min, max, spec);
            let value_log = value.log10();
            remap_clamp(value_log, min_log..=max_log, 0.0..=1.0)
        } else {
            assert!(min < 0.0 && 0.0 < max);
            let zero_cutoff = logaritmic_zero_cutoff(min, max);
            if value < 0.0 {
                // negative
                remap(
                    normalized_from_value(value, min..=0.0, spec),
                    0.0..=1.0,
                    0.0..=zero_cutoff,
                )
            } else {
                // positive side
                remap(
                    normalized_from_value(value, 0.0..=max, spec),
                    0.0..=1.0,
                    zero_cutoff..=1.0,
                )
            }
        }
    } else {
        debug_assert!(
            min.is_finite() && max.is_finite(),
            "You should use a logarithmic range"
        );
        remap_clamp(value, range, 0.0..=1.0)
    }
}

fn range_log10(min: f64, max: f64, spec: &SliderSpec) -> (f64, f64) {
    assert!(spec.logarithmic);
    assert!(min <= max);

    if min == 0.0 && max == INFINITY {
        (spec.smallest_positive.log10(), INF_RANGE_MAGNITUDE)
    } else if min == 0.0 {
        if spec.smallest_positive < max {
            (spec.smallest_positive.log10(), max.log10())
        } else {
            (max.log10() - INF_RANGE_MAGNITUDE, max.log10())
        }
    } else if max == INFINITY {
        (min.log10(), min.log10() + INF_RANGE_MAGNITUDE)
    } else {
        (min.log10(), max.log10())
    }
}

/// where to put the zero cutoff for logarithmic sliders
/// that crosses zero ?
fn logaritmic_zero_cutoff(min: f64, max: f64) -> f64 {
    assert!(min < 0.0 && 0.0 < max);

    let min_magnitude = if min == -INFINITY {
        INF_RANGE_MAGNITUDE
    } else {
        min.abs().log10().abs()
    };
    let max_magnitude = if max == INFINITY {
        INF_RANGE_MAGNITUDE
    } else {
        max.log10().abs()
    };

    let cutoff = min_magnitude / (min_magnitude + max_magnitude);
    debug_assert!(0.0 <= cutoff && cutoff <= 1.0);
    cutoff
}
