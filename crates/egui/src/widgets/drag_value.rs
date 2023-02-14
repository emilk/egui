#![allow(clippy::needless_pass_by_value)] // False positives with `impl ToString`

use std::{cmp::Ordering, ops::RangeInclusive};

use crate::*;

// ----------------------------------------------------------------------------

/// Same state for all [`DragValue`]s.
#[derive(Clone, Debug, Default)]
pub(crate) struct MonoState {
    last_dragged_id: Option<Id>,
    last_dragged_value: Option<f64>,
    /// For temporary edit of a [`DragValue`] value.
    /// Couples with the current focus id.
    edit_string: Option<String>,
}

impl MonoState {
    pub(crate) fn end_frame(&mut self, input: &InputState) {
        if input.pointer.any_pressed() || input.pointer.any_released() {
            self.last_dragged_id = None;
            self.last_dragged_value = None;
        }
    }
}

// ----------------------------------------------------------------------------

type NumFormatter<'a> = Box<dyn 'a + Fn(f64, RangeInclusive<usize>) -> String>;
type NumParser<'a> = Box<dyn 'a + Fn(&str) -> Option<f64>>;

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

/// A numeric value that you can change by dragging the number. More compact than a [`Slider`].
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// # let mut my_f32: f32 = 0.0;
/// ui.add(egui::DragValue::new(&mut my_f32).speed(0.1));
/// # });
/// ```
#[must_use = "You should put this widget in an ui with `ui.add(widget);`"]
pub struct DragValue<'a> {
    get_set_value: GetSetValue<'a>,
    speed: f64,
    prefix: String,
    suffix: String,
    clamp_range: RangeInclusive<f64>,
    min_decimals: usize,
    max_decimals: Option<usize>,
    custom_formatter: Option<NumFormatter<'a>>,
    custom_parser: Option<NumParser<'a>>,
}

impl<'a> DragValue<'a> {
    pub fn new<Num: emath::Numeric>(value: &'a mut Num) -> Self {
        let slf = Self::from_get_set(move |v: Option<f64>| {
            if let Some(v) = v {
                *value = Num::from_f64(v);
            }
            value.to_f64()
        });

        if Num::INTEGRAL {
            slf.max_decimals(0)
                .clamp_range(Num::MIN..=Num::MAX)
                .speed(0.25)
        } else {
            slf
        }
    }

    pub fn from_get_set(get_set_value: impl 'a + FnMut(Option<f64>) -> f64) -> Self {
        Self {
            get_set_value: Box::new(get_set_value),
            speed: 1.0,
            prefix: Default::default(),
            suffix: Default::default(),
            clamp_range: f64::NEG_INFINITY..=f64::INFINITY,
            min_decimals: 0,
            max_decimals: None,
            custom_formatter: None,
            custom_parser: None,
        }
    }

    /// How much the value changes when dragged one point (logical pixel).
    pub fn speed(mut self, speed: impl Into<f64>) -> Self {
        self.speed = speed.into();
        self
    }

    /// Clamp incoming and outgoing values to this range.
    pub fn clamp_range<Num: emath::Numeric>(mut self, clamp_range: RangeInclusive<Num>) -> Self {
        self.clamp_range = clamp_range.start().to_f64()..=clamp_range.end().to_f64();
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

    // TODO(emilk): we should also have a "min precision".
    /// Set a minimum number of decimals to display.
    /// Normally you don't need to pick a precision, as the slider will intelligently pick a precision for you.
    /// Regardless of precision the slider will use "smart aim" to help the user select nice, round values.
    pub fn min_decimals(mut self, min_decimals: usize) -> Self {
        self.min_decimals = min_decimals;
        self
    }

    // TODO(emilk): we should also have a "max precision".
    /// Set a maximum number of decimals to display.
    /// Values will also be rounded to this number of decimals.
    /// Normally you don't need to pick a precision, as the slider will intelligently pick a precision for you.
    /// Regardless of precision the slider will use "smart aim" to help the user select nice, round values.
    pub fn max_decimals(mut self, max_decimals: usize) -> Self {
        self.max_decimals = Some(max_decimals);
        self
    }

    pub fn max_decimals_opt(mut self, max_decimals: Option<usize>) -> Self {
        self.max_decimals = max_decimals;
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

    /// Set custom formatter defining how numbers are converted into text.
    ///
    /// A custom formatter takes a `f64` for the numeric value and a `RangeInclusive<usize>` representing
    /// the decimal range i.e. minimum and maximum number of decimal places shown.
    ///
    /// See also: [`DragValue::custom_parser`]
    ///
    /// ```
    /// # egui::__run_test_ui(|ui| {
    /// # let mut my_i32: i32 = 0;
    /// ui.add(egui::DragValue::new(&mut my_i32)
    ///     .clamp_range(0..=((60 * 60 * 24) - 1))
    ///     .custom_formatter(|n, _| {
    ///         let n = n as i32;
    ///         let hours = n / (60 * 60);
    ///         let mins = (n / 60) % 60;
    ///         let secs = n % 60;
    ///         format!("{hours:02}:{mins:02}:{secs:02}")
    ///     })
    ///     .custom_parser(|s| {
    ///         let parts: Vec<&str> = s.split(':').collect();
    ///         if parts.len() == 3 {
    ///             parts[0].parse::<i32>().and_then(|h| {
    ///                 parts[1].parse::<i32>().and_then(|m| {
    ///                     parts[2].parse::<i32>().map(|s| {
    ///                         ((h * 60 * 60) + (m * 60) + s) as f64
    ///                     })
    ///                 })
    ///             })
    ///             .ok()
    ///         } else {
    ///             None
    ///         }
    ///     }));
    /// # });
    /// ```
    pub fn custom_formatter(
        mut self,
        formatter: impl 'a + Fn(f64, RangeInclusive<usize>) -> String,
    ) -> Self {
        self.custom_formatter = Some(Box::new(formatter));
        self
    }

    /// Set custom parser defining how the text input is parsed into a number.
    ///
    /// A custom parser takes an `&str` to parse into a number and returns a `f64` if it was successfully parsed
    /// or `None` otherwise.
    ///
    /// See also: [`DragValue::custom_formatter`]
    ///
    /// ```
    /// # egui::__run_test_ui(|ui| {
    /// # let mut my_i32: i32 = 0;
    /// ui.add(egui::DragValue::new(&mut my_i32)
    ///     .clamp_range(0..=((60 * 60 * 24) - 1))
    ///     .custom_formatter(|n, _| {
    ///         let n = n as i32;
    ///         let hours = n / (60 * 60);
    ///         let mins = (n / 60) % 60;
    ///         let secs = n % 60;
    ///         format!("{hours:02}:{mins:02}:{secs:02}")
    ///     })
    ///     .custom_parser(|s| {
    ///         let parts: Vec<&str> = s.split(':').collect();
    ///         if parts.len() == 3 {
    ///             parts[0].parse::<i32>().and_then(|h| {
    ///                 parts[1].parse::<i32>().and_then(|m| {
    ///                     parts[2].parse::<i32>().map(|s| {
    ///                         ((h * 60 * 60) + (m * 60) + s) as f64
    ///                     })
    ///                 })
    ///             })
    ///             .ok()
    ///         } else {
    ///             None
    ///         }
    ///     }));
    /// # });
    /// ```
    pub fn custom_parser(mut self, parser: impl 'a + Fn(&str) -> Option<f64>) -> Self {
        self.custom_parser = Some(Box::new(parser));
        self
    }

    /// Set `custom_formatter` and `custom_parser` to display and parse numbers as binary integers. Floating point
    /// numbers are *not* supported.
    ///
    /// `min_width` specifies the minimum number of displayed digits; if the number is shorter than this, it will be
    /// prefixed with additional 0s to match `min_width`.
    ///
    /// If `twos_complement` is true, negative values will be displayed as the 2's complement representation. Otherwise
    /// they will be prefixed with a '-' sign.
    ///
    /// # Panics
    ///
    /// Panics if `min_width` is 0.
    ///
    /// ```
    /// # egui::__run_test_ui(|ui| {
    /// # let mut my_i32: i32 = 0;
    /// ui.add(egui::DragValue::new(&mut my_i32).binary(64, false));
    /// # });
    /// ```
    pub fn binary(self, min_width: usize, twos_complement: bool) -> Self {
        assert!(
            min_width > 0,
            "DragValue::binary: `min_width` must be greater than 0"
        );
        if twos_complement {
            self.custom_formatter(move |n, _| format!("{:0>min_width$b}", n as i64))
        } else {
            self.custom_formatter(move |n, _| {
                let sign = if n < 0.0 { "-" } else { "" };
                format!("{sign}{:0>min_width$b}", n.abs() as i64)
            })
        }
        .custom_parser(|s| i64::from_str_radix(s, 2).map(|n| n as f64).ok())
    }

    /// Set `custom_formatter` and `custom_parser` to display and parse numbers as octal integers. Floating point
    /// numbers are *not* supported.
    ///
    /// `min_width` specifies the minimum number of displayed digits; if the number is shorter than this, it will be
    /// prefixed with additional 0s to match `min_width`.
    ///
    /// If `twos_complement` is true, negative values will be displayed as the 2's complement representation. Otherwise
    /// they will be prefixed with a '-' sign.
    ///
    /// # Panics
    ///
    /// Panics if `min_width` is 0.
    ///
    /// ```
    /// # egui::__run_test_ui(|ui| {
    /// # let mut my_i32: i32 = 0;
    /// ui.add(egui::DragValue::new(&mut my_i32).octal(22, false));
    /// # });
    /// ```
    pub fn octal(self, min_width: usize, twos_complement: bool) -> Self {
        assert!(
            min_width > 0,
            "DragValue::octal: `min_width` must be greater than 0"
        );
        if twos_complement {
            self.custom_formatter(move |n, _| format!("{:0>min_width$o}", n as i64))
        } else {
            self.custom_formatter(move |n, _| {
                let sign = if n < 0.0 { "-" } else { "" };
                format!("{sign}{:0>min_width$o}", n.abs() as i64)
            })
        }
        .custom_parser(|s| i64::from_str_radix(s, 8).map(|n| n as f64).ok())
    }

    /// Set `custom_formatter` and `custom_parser` to display and parse numbers as hexadecimal integers. Floating point
    /// numbers are *not* supported.
    ///
    /// `min_width` specifies the minimum number of displayed digits; if the number is shorter than this, it will be
    /// prefixed with additional 0s to match `min_width`.
    ///
    /// If `twos_complement` is true, negative values will be displayed as the 2's complement representation. Otherwise
    /// they will be prefixed with a '-' sign.
    ///
    /// # Panics
    ///
    /// Panics if `min_width` is 0.
    ///
    /// ```
    /// # egui::__run_test_ui(|ui| {
    /// # let mut my_i32: i32 = 0;
    /// ui.add(egui::DragValue::new(&mut my_i32).hexadecimal(16, false, true));
    /// # });
    /// ```
    pub fn hexadecimal(self, min_width: usize, twos_complement: bool, upper: bool) -> Self {
        assert!(
            min_width > 0,
            "DragValue::hexadecimal: `min_width` must be greater than 0"
        );
        match (twos_complement, upper) {
            (true, true) => {
                self.custom_formatter(move |n, _| format!("{:0>min_width$X}", n as i64))
            }
            (true, false) => {
                self.custom_formatter(move |n, _| format!("{:0>min_width$x}", n as i64))
            }
            (false, true) => self.custom_formatter(move |n, _| {
                let sign = if n < 0.0 { "-" } else { "" };
                format!("{sign}{:0>min_width$X}", n.abs() as i64)
            }),
            (false, false) => self.custom_formatter(move |n, _| {
                let sign = if n < 0.0 { "-" } else { "" };
                format!("{sign}{:0>min_width$x}", n.abs() as i64)
            }),
        }
        .custom_parser(|s| i64::from_str_radix(s, 16).map(|n| n as f64).ok())
    }
}

impl<'a> Widget for DragValue<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let Self {
            mut get_set_value,
            speed,
            clamp_range,
            prefix,
            suffix,
            min_decimals,
            max_decimals,
            custom_formatter,
            custom_parser,
        } = self;

        let shift = ui.input(|i| i.modifiers.shift_only());
        // The widget has the same ID whether it's in edit or button mode.
        let id = ui.next_auto_id();
        let is_slow_speed = shift && ui.memory(|mem| mem.is_being_dragged(id));

        // The following ensures that when a `DragValue` receives focus,
        // it is immediately rendered in edit mode, rather than being rendered
        // in button mode for just one frame. This is important for
        // screen readers.
        let is_kb_editing = ui.memory_mut(|mem| {
            mem.interested_in_focus(id);
            let is_kb_editing = mem.has_focus(id);
            if mem.gained_focus(id) {
                mem.drag_value.edit_string = None;
            }
            is_kb_editing
        });

        let old_value = get(&mut get_set_value);
        let mut value = old_value;
        let aim_rad = ui.input(|i| i.aim_radius() as f64);

        let auto_decimals = (aim_rad / speed.abs()).log10().ceil().clamp(0.0, 15.0) as usize;
        let auto_decimals = auto_decimals + is_slow_speed as usize;
        let max_decimals = max_decimals.unwrap_or(auto_decimals + 2);
        let auto_decimals = auto_decimals.clamp(min_decimals, max_decimals);

        let change = ui.input_mut(|input| {
            let mut change = 0.0;

            if is_kb_editing {
                // This deliberately doesn't listen for left and right arrow keys,
                // because when editing, these are used to move the caret.
                // This behavior is consistent with other editable spinner/stepper
                // implementations, such as Chromium's (for HTML5 number input).
                // It is also normal for such controls to go directly into edit mode
                // when they receive keyboard focus, and some screen readers
                // assume this behavior, so having a separate mode for incrementing
                // and decrementing, that supports all arrow keys, would be
                // problematic.
                change += input.count_and_consume_key(Modifiers::NONE, Key::ArrowUp) as f64
                    - input.count_and_consume_key(Modifiers::NONE, Key::ArrowDown) as f64;
            }

            #[cfg(feature = "accesskit")]
            {
                use accesskit::Action;
                change += input.num_accesskit_action_requests(id, Action::Increment) as f64
                    - input.num_accesskit_action_requests(id, Action::Decrement) as f64;
            }

            change
        });

        #[cfg(feature = "accesskit")]
        {
            use accesskit::{Action, ActionData};
            ui.input(|input| {
                for request in input.accesskit_action_requests(id, Action::SetValue) {
                    if let Some(ActionData::NumericValue(new_value)) = request.data {
                        value = new_value;
                    }
                }
            });
        }

        if change != 0.0 {
            value += speed * change;
            value = emath::round_to_decimals(value, auto_decimals);
        }

        value = clamp_to_range(value, clamp_range.clone());
        if old_value != value {
            set(&mut get_set_value, value);
            ui.memory_mut(|mem| mem.drag_value.edit_string = None);
        }

        let value_text = match custom_formatter {
            Some(custom_formatter) => custom_formatter(value, auto_decimals..=max_decimals),
            None => {
                if value == 0.0 {
                    "0".to_owned()
                } else {
                    emath::format_with_decimals_in_range(value, auto_decimals..=max_decimals)
                }
            }
        };

        let text_style = ui.style().drag_value_text_style.clone();

        // some clones below are redundant if AccessKit is disabled
        #[allow(clippy::redundant_clone)]
        let mut response = if is_kb_editing {
            let mut value_text = ui
                .memory_mut(|mem| mem.drag_value.edit_string.take())
                .unwrap_or_else(|| value_text.clone());
            let response = ui.add(
                TextEdit::singleline(&mut value_text)
                    .clip_text(false)
                    .horizontal_align(ui.layout().horizontal_align())
                    .vertical_align(ui.layout().vertical_align())
                    .margin(ui.spacing().button_padding)
                    .min_size(ui.spacing().interact_size)
                    .id(id)
                    .desired_width(ui.spacing().interact_size.x)
                    .font(text_style),
            );
            // Only update the value when the user presses enter, or clicks elsewhere. NOT every frame.
            // See https://github.com/emilk/egui/issues/2687
            if response.lost_focus() {
                let parsed_value = match custom_parser {
                    Some(parser) => parser(&value_text),
                    None => value_text.parse().ok(),
                };
                if let Some(parsed_value) = parsed_value {
                    let parsed_value = clamp_to_range(parsed_value, clamp_range.clone());
                    set(&mut get_set_value, parsed_value);
                }
            }
            ui.memory_mut(|mem| mem.drag_value.edit_string = Some(value_text));
            response
        } else {
            let button = Button::new(
                RichText::new(format!("{}{}{}", prefix, value_text.clone(), suffix))
                    .text_style(text_style),
            )
            .wrap(false)
            .sense(Sense::click_and_drag())
            .min_size(ui.spacing().interact_size); // TODO(emilk): find some more generic solution to `min_size`

            let response = ui.add(button);
            let mut response = response.on_hover_cursor(CursorIcon::ResizeHorizontal);

            if ui.style().explanation_tooltips {
                response = response .on_hover_text(format!(
                    "{}{}{}\nDrag to edit or click to enter a value.\nPress 'Shift' while dragging for better control.",
                    prefix,
                    value as f32, // Show full precision value on-hover. TODO(emilk): figure out f64 vs f32
                    suffix
                ));
            }

            if response.clicked() {
                ui.memory_mut(|mem| {
                    mem.drag_value.edit_string = None;
                    mem.request_focus(id);
                });
                let mut state = TextEdit::load_state(ui.ctx(), id).unwrap_or_default();
                state.set_ccursor_range(Some(text::CCursorRange::two(
                    epaint::text::cursor::CCursor::default(),
                    epaint::text::cursor::CCursor::new(value_text.chars().count()),
                )));
                state.store(ui.ctx(), response.id);
            } else if response.dragged() {
                ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal);

                let mdelta = response.drag_delta();
                let delta_points = mdelta.x - mdelta.y; // Increase to the right and up

                let speed = if is_slow_speed { speed / 10.0 } else { speed };

                let delta_value = delta_points as f64 * speed;

                if delta_value != 0.0 {
                    let mut drag_state = ui.memory_mut(|mem| std::mem::take(&mut mem.drag_value));

                    // Since we round the value being dragged, we need to store the full precision value in memory:
                    let stored_value = (drag_state.last_dragged_id == Some(response.id))
                        .then_some(drag_state.last_dragged_value)
                        .flatten();
                    let stored_value = stored_value.unwrap_or(value);
                    let stored_value = stored_value + delta_value;

                    let aim_delta = aim_rad * speed;
                    let rounded_new_value = emath::smart_aim::best_in_range_f64(
                        stored_value - aim_delta,
                        stored_value + aim_delta,
                    );
                    let rounded_new_value =
                        emath::round_to_decimals(rounded_new_value, auto_decimals);
                    let rounded_new_value = clamp_to_range(rounded_new_value, clamp_range.clone());
                    set(&mut get_set_value, rounded_new_value);

                    drag_state.last_dragged_id = Some(response.id);
                    drag_state.last_dragged_value = Some(stored_value);
                    ui.memory_mut(|mem| mem.drag_value = drag_state);
                }
            }

            response
        };

        response.changed = get(&mut get_set_value) != old_value;

        response.widget_info(|| WidgetInfo::drag_value(value));

        #[cfg(feature = "accesskit")]
        ui.ctx().accesskit_node_builder(response.id, |builder| {
            use accesskit::Action;
            // If either end of the range is unbounded, it's better
            // to leave the corresponding AccessKit field set to None,
            // to allow for platform-specific default behavior.
            if clamp_range.start().is_finite() {
                builder.set_min_numeric_value(*clamp_range.start());
            }
            if clamp_range.end().is_finite() {
                builder.set_max_numeric_value(*clamp_range.end());
            }
            builder.set_numeric_value_step(speed);
            builder.add_action(Action::SetValue);
            if value < *clamp_range.end() {
                builder.add_action(Action::Increment);
            }
            if value > *clamp_range.start() {
                builder.add_action(Action::Decrement);
            }
            // The name field is set to the current value by the button,
            // but we don't want it set that way on this widget type.
            builder.clear_name();
            // Always expose the value as a string. This makes the widget
            // more stable to accessibility users as it switches
            // between edit and button modes. This is particularly important
            // for VoiceOver on macOS; if the value is not exposed as a string
            // when the widget is in button mode, then VoiceOver speaks
            // the value (or a percentage if the widget has a clamp range)
            // when the widget loses focus, overriding the announcement
            // of the newly focused widget. This is certainly a VoiceOver bug,
            // but it's good to make our software work as well as possible
            // with existing assistive technology. However, if the widget
            // has a prefix and/or suffix, expose those when in button mode,
            // just as they're exposed on the screen. This triggers the
            // VoiceOver bug just described, but exposing all information
            // is more important, and at least we can avoid the bug
            // for instances of the widget with no prefix or suffix.
            //
            // The value is exposed as a string by the text edit widget
            // when in edit mode.
            if !is_kb_editing {
                let value_text = format!("{}{}{}", prefix, value_text, suffix);
                builder.set_value(value_text);
            }
        });

        response
    }
}

fn clamp_to_range(x: f64, range: RangeInclusive<f64>) -> f64 {
    let (mut min, mut max) = (*range.start(), *range.end());

    if min.total_cmp(&max) == Ordering::Greater {
        (min, max) = (max, min);
    }

    match x.total_cmp(&min) {
        Ordering::Less | Ordering::Equal => min,
        Ordering::Greater => match x.total_cmp(&max) {
            Ordering::Greater | Ordering::Equal => max,
            Ordering::Less => x,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::clamp_to_range;

    macro_rules! total_assert_eq {
        ($a:expr, $b:expr) => {
            assert!(
                matches!($a.total_cmp(&$b), std::cmp::Ordering::Equal),
                "{} != {}",
                $a,
                $b
            );
        };
    }

    #[test]
    fn test_total_cmp_clamp_to_range() {
        total_assert_eq!(0.0_f64, clamp_to_range(-0.0, 0.0..=f64::MAX));
        total_assert_eq!(-0.0_f64, clamp_to_range(0.0, -1.0..=-0.0));
        total_assert_eq!(-1.0_f64, clamp_to_range(-25.0, -1.0..=1.0));
        total_assert_eq!(5.0_f64, clamp_to_range(5.0, -1.0..=10.0));
        total_assert_eq!(15.0_f64, clamp_to_range(25.0, -1.0..=15.0));
        total_assert_eq!(1.0_f64, clamp_to_range(1.0, 1.0..=10.0));
        total_assert_eq!(10.0_f64, clamp_to_range(10.0, 1.0..=10.0));
        total_assert_eq!(5.0_f64, clamp_to_range(5.0, 10.0..=1.0));
        total_assert_eq!(5.0_f64, clamp_to_range(15.0, 5.0..=1.0));
        total_assert_eq!(1.0_f64, clamp_to_range(-5.0, 5.0..=1.0));
    }
}
