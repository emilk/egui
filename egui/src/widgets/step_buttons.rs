use std::ops::RangeInclusive;

use crate::*;

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

/// Decrement [-] and increment [+] buttons for a value.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// # let mut my_f32: f32 = 0.0;
/// ui.add(egui::StepButtons::new(&mut my_f32).step(0.1));
/// # });
/// ```
#[must_use = "You should put this widget in an ui with `ui.add(widget);`"]
pub struct StepButtons<'a> {
    get_set_value: GetSetValue<'a>,
    clamp_range: RangeInclusive<f64>,
    step: f64,
}

impl<'a> StepButtons<'a> {
    pub fn new<Num: emath::Numeric>(value: &'a mut Num) -> Self {
        let slf = Self::from_get_set(move |v: Option<f64>| {
            if let Some(v) = v {
                *value = Num::from_f64(v);
            }
            value.to_f64()
        });

        if Num::INTEGRAL {
            slf.clamp_range(Num::MIN..=Num::MAX)
        } else {
            slf
        }
    }

    pub fn from_get_set(get_set_value: impl 'a + FnMut(Option<f64>) -> f64) -> Self {
        Self {
            get_set_value: Box::new(get_set_value),
            clamp_range: f64::NEG_INFINITY..=f64::INFINITY,
            step: 1.0,
        }
    }

    /// How much the value changes when clicked. Holding `Shift` while clicking makes 10 times smaller steps.
    pub fn step(mut self, step: impl Into<f64>) -> Self {
        self.step = step.into();
        self
    }

    /// Clamp incoming and outgoing values to this range.
    pub fn clamp_range<Num: emath::Numeric>(mut self, clamp_range: RangeInclusive<Num>) -> Self {
        self.clamp_range = clamp_range.start().to_f64()..=clamp_range.end().to_f64();
        self
    }
}

impl<'a> Widget for StepButtons<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let Self {
            mut get_set_value,
            clamp_range,
            step,
        } = self;
        let is_slow_speed = ui.input().modifiers.shift_only();
        let old_value = get(&mut get_set_value);
        let value = clamp_to_range(old_value, &clamp_range);
        let mut response = ui
            .horizontal(|ui| {
                let step = if is_slow_speed { step / 10.0 } else { step };

                let minus_button_response = ui.add_enabled(
                    value > *clamp_range.start() && step > 0.0,
                    Button::new("➖"),
                );
                if minus_button_response.clicked() {
                    let new_value = value - step;
                    let new_value = clamp_to_range(new_value, &clamp_range);
                    set(&mut get_set_value, new_value);
                }
                let plus_button_response =
                    ui.add_enabled(value < *clamp_range.end() && step > 0.0, Button::new("➕"));
                if plus_button_response.clicked() {
                    let new_value = value + step;
                    let new_value = clamp_to_range(new_value, &clamp_range);
                    set(&mut get_set_value, new_value);
                }
            })
            .response;
        response.changed = get(&mut get_set_value) != old_value;
        response.widget_info(|| WidgetInfo::step_buttons(value));
        response
    }
}

fn clamp_to_range(x: f64, range: &RangeInclusive<f64>) -> f64 {
    x.clamp(
        range.start().min(*range.end()),
        range.start().max(*range.end()),
    )
}
