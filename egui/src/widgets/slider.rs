#![allow(clippy::needless_pass_by_value)] // False positives with `impl ToString`
#![allow(clippy::float_cmp)]

use crate::{widgets::Label, *};
use std::ops::RangeInclusive;

// ----------------------------------------------------------------------------

/// Combined into one function (rather than two) to make it easier
/// for the borrow checker.
type GetSetValue<'a> = Box<dyn 'a + FnMut(Option<f64>) -> f64>;

fn get(get_set_value: &mut GetSetValue<'_>) -> f64 {
    (get_set_value)(None)
}

fn set(get_set_value: &mut GetSetValue<'_>, value: f64) {
    (get_set_value)(Some(value));
}

// ----------------------------------------------------------------------------

#[derive(Clone)]
struct SliderSpec {
    logarithmic: bool,
    /// For logarithmic sliders, the smallest positive value we are interested in.
    /// 1 for integer sliders, maybe 1e-6 for others.
    smallest_positive: f64,
    /// For logarithmic sliders, the largest positive value we are interested in
    /// before the slider switches to `INFINITY`, if that is the higher end.
    /// Default: INFINITY.
    largest_finite: f64,
}

/// Control a number by a horizontal slider.
///
/// The slider range defines the values you get when pulling the slider to the far edges.
/// By default, the slider can still show values outside this range,
/// and still allows users to enter values outside the range by clicking the slider value and editing it.
/// If you want to clamp incoming and outgoing values, use [`Slider::clamp_to_range`].
///
/// The range can include any numbers, and go from low-to-high or from high-to-low.
///
/// The slider consists of three parts: a horizontal slider, a value display, and an optional text.
/// The user can click the value display to edit its value. It can be turned off with `.show_value(false)`.
///
/// ```
/// # let ui = &mut egui::Ui::__test();
/// # let mut my_f32: f32 = 0.0;
/// ui.add(egui::Slider::f32(&mut my_f32, 0.0..=100.0).text("My value"));
/// ```
#[must_use = "You should put this widget in an ui with `ui.add(widget);`"]
pub struct Slider<'a> {
    get_set_value: GetSetValue<'a>,
    range: RangeInclusive<f64>,
    spec: SliderSpec,
    clamp_to_range: bool,
    smart_aim: bool,
    show_value: bool,
    prefix: String,
    suffix: String,
    text: String,
    text_color: Option<Color32>,
    min_decimals: usize,
    max_decimals: Option<usize>,
}

macro_rules! impl_integer_constructor {
    ($int:ident) => {
        pub fn $int(value: &'a mut $int, range: RangeInclusive<$int>) -> Self {
            let range_f64 = (*range.start() as f64)..=(*range.end() as f64);
            Self::from_get_set(range_f64, move |v: Option<f64>| {
                if let Some(v) = v {
                    *value = v.round() as $int
                }
                *value as f64
            })
            .integer()
        }
    };
}

impl<'a> Slider<'a> {
    pub fn f32(value: &'a mut f32, range: RangeInclusive<f32>) -> Self {
        let range_f64 = (*range.start() as f64)..=(*range.end() as f64);
        Self::from_get_set(range_f64, move |v: Option<f64>| {
            if let Some(v) = v {
                *value = v as f32
            }
            *value as f64
        })
    }

    pub fn f64(value: &'a mut f64, range: RangeInclusive<f64>) -> Self {
        Self::from_get_set(range, move |v: Option<f64>| {
            if let Some(v) = v {
                *value = v
            }
            *value
        })
    }

    impl_integer_constructor!(i8);
    impl_integer_constructor!(u8);
    impl_integer_constructor!(i16);
    impl_integer_constructor!(u16);
    impl_integer_constructor!(i32);
    impl_integer_constructor!(u32);
    impl_integer_constructor!(i64);
    impl_integer_constructor!(u64);
    impl_integer_constructor!(isize);
    impl_integer_constructor!(usize);

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
                largest_finite: f64::INFINITY,
            },
            clamp_to_range: false,
            smart_aim: true,
            show_value: true,
            prefix: Default::default(),
            suffix: Default::default(),
            text: Default::default(),
            text_color: None,
            min_decimals: 0,
            max_decimals: None,
        }
    }

    /// Control wether or not the slider shows the current value.
    /// Default: `true`.
    pub fn show_value(mut self, show_value: bool) -> Self {
        self.show_value = show_value;
        self
    }

    /// Show a prefix before the number, e.g. "x: "
    pub fn prefix(mut self, prefix: impl ToString) -> Self {
        self.prefix = prefix.to_string();
        self
    }

    /// Add a suffix to the number, this can be e.g. a unit ("°" or " m")
    pub fn suffix(mut self, suffix: impl ToString) -> Self {
        self.suffix = suffix.to_string();
        self
    }

    /// Show a text next to the slider (e.g. explaining what the slider controls).
    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.text = text.into();
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

    /// For logarithmic sliders, the largest positive value we are interested in
    /// before the slider switches to `INFINITY`, if that is the higher end.
    /// Default: INFINITY.
    pub fn largest_finite(mut self, largest_finite: f64) -> Self {
        self.spec.largest_finite = largest_finite;
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
            let start = *self.range.start();
            let end = *self.range.end();
            value.clamp(start.min(end), start.max(end))
        } else {
            value
        }
    }

    fn set_value(&mut self, mut value: f64) {
        if self.clamp_to_range {
            let start = *self.range.start();
            let end = *self.range.end();
            value = value.clamp(start.min(end), start.max(end));
        }
        if let Some(max_decimals) = self.max_decimals {
            value = emath::round_to_decimals(value, max_decimals);
        }
        set(&mut self.get_set_value, value);
    }

    fn clamp_range(&self) -> RangeInclusive<f64> {
        if self.clamp_to_range {
            self.range()
        } else {
            f64::NEG_INFINITY..=f64::INFINITY
        }
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
    #[allow(clippy::unused_self)]
    fn allocate_slider_space(&self, ui: &mut Ui, height: f32) -> Response {
        let desired_size = vec2(ui.spacing().slider_width, height);
        ui.allocate_response(desired_size, Sense::click_and_drag())
    }

    /// Just the slider, no text
    fn slider_ui(&mut self, ui: &mut Ui, response: &Response) {
        let rect = &response.rect;
        let x_range = x_range(rect);

        if let Some(pointer_pos) = response.interact_pointer_pos() {
            let new_value = if self.smart_aim {
                let aim_radius = ui.input().aim_radius();
                emath::smart_aim::best_in_range_f64(
                    self.value_from_x(pointer_pos.x - aim_radius, x_range.clone()),
                    self.value_from_x(pointer_pos.x + aim_radius, x_range.clone()),
                )
            } else {
                self.value_from_x(pointer_pos.x, x_range.clone())
            };
            self.set_value(new_value);
        }

        let value = self.get_value();
        response.widget_info(|| WidgetInfo::slider(value, &self.text));

        if response.has_focus() {
            let kb_step = ui.input().num_presses(Key::ArrowRight) as f32
                - ui.input().num_presses(Key::ArrowLeft) as f32;

            if kb_step != 0.0 {
                let prev_value = self.get_value();
                let prev_x = self.x_from_value(prev_value, x_range.clone());
                let new_x = prev_x + kb_step;
                let new_value = if self.smart_aim {
                    let aim_radius = ui.input().aim_radius();
                    emath::smart_aim::best_in_range_f64(
                        self.value_from_x(new_x - aim_radius, x_range.clone()),
                        self.value_from_x(new_x + aim_radius, x_range.clone()),
                    )
                } else {
                    self.value_from_x(new_x, x_range.clone())
                };
                self.set_value(new_value);
            }
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

                fill: ui.visuals().widgets.inactive.bg_fill,
                // fill: visuals.bg_fill,
                // fill: ui.visuals().extreme_bg_color,
                stroke: Default::default(),
                // stroke: visuals.bg_stroke,
                // stroke: ui.visuals().widgets.inactive.bg_stroke,
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
        if !self.text.is_empty() {
            let text_color = self.text_color.unwrap_or_else(|| ui.visuals().text_color());
            ui.add(Label::new(&self.text).wrap(false).text_color(text_color));
        }
    }

    fn value_ui(&mut self, ui: &mut Ui, x_range: RangeInclusive<f32>) {
        let mut value = self.get_value();
        ui.add(
            DragValue::f64(&mut value)
                .speed(self.current_gradient(&x_range))
                .clamp_range_f64(self.clamp_range())
                .min_decimals(self.min_decimals)
                .max_decimals_opt(self.max_decimals)
                .suffix(self.suffix.clone())
                .prefix(self.prefix.clone()),
        );
        if value != self.get_value() {
            self.set_value(value);
        }
    }

    /// delta(value) / delta(points)
    fn current_gradient(&mut self, x_range: &RangeInclusive<f32>) -> f64 {
        // TODO: handle clamping
        let value = self.get_value();
        let value_from_x = |x: f32| self.value_from_x(x, x_range.clone());
        let x_from_value = |value: f64| self.x_from_value(value, x_range.clone());
        let left_value = value_from_x(x_from_value(value) - 0.5);
        let right_value = value_from_x(x_from_value(value) + 0.5);
        right_value - left_value
    }
}

impl<'a> Widget for Slider<'a> {
    fn ui(mut self, ui: &mut Ui) -> Response {
        let text_style = TextStyle::Button;
        let font = &ui.fonts()[text_style];
        let height = font.row_height().at_least(ui.spacing().interact_size.y);

        let old_value = self.get_value();

        let inner_response = ui.horizontal(|ui| {
            let slider_response = self.allocate_slider_space(ui, height);
            self.slider_ui(ui, &slider_response);

            if self.show_value {
                let x_range = x_range(&slider_response.rect);
                self.value_ui(ui, x_range);
            }

            if !self.text.is_empty() {
                self.label_ui(ui);
            }
            slider_response
        });

        let mut response = inner_response.inner | inner_response.response;
        response.changed = self.get_value() != old_value;
        response
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
        lerp(range, normalized.clamp(0.0, 1.0))
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
        if min < spec.largest_finite {
            (min.log10(), spec.largest_finite.log10())
        } else {
            (min.log10(), min.log10() + INF_RANGE_MAGNITUDE)
        }
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
