#![allow(clippy::needless_pass_by_value)] // False positives with `impl ToString`

use std::ops::RangeInclusive;

use crate::*;

/// Combined into one function (rather than two) to make it easier
/// for the borrow checker.
type GetSetValue<'a> = Box<dyn 'a + FnMut(Option<f64>) -> f64>;

fn get(value_function: &mut GetSetValue<'_>) -> f64 {
    (value_function)(None)
}

fn set(value_function: &mut GetSetValue<'_>, value: f64) {
    (value_function)(Some(value));
}

/// A numeric value that you can change by dragging the number. More compact than a [`Slider`].
#[must_use = "You should put this widget in an ui with `ui.add(widget);`"]
pub struct DragValue<'a> {
    value_function: GetSetValue<'a>,
    speed: f32,
    prefix: String,
    suffix: String,
    clamp_range: RangeInclusive<f64>,
    min_decimals: usize,
    max_decimals: Option<usize>,
}

impl<'a> DragValue<'a> {
    pub(crate) fn from_get_set(value_function: impl 'a + FnMut(Option<f64>) -> f64) -> Self {
        Self {
            value_function: Box::new(value_function),
            speed: 1.0,
            prefix: Default::default(),
            suffix: Default::default(),
            clamp_range: f64::NEG_INFINITY..=f64::INFINITY,
            min_decimals: 0,
            max_decimals: None,
        }
    }

    pub fn f32(value: &'a mut f32) -> Self {
        Self::from_get_set(move |v: Option<f64>| {
            if let Some(v) = v {
                *value = v as f32
            }
            *value as f64
        })
    }

    pub fn f64(value: &'a mut f64) -> Self {
        Self::from_get_set(move |v: Option<f64>| {
            if let Some(v) = v {
                *value = v
            }
            *value
        })
    }

    pub fn u8(value: &'a mut u8) -> Self {
        Self::from_get_set(move |v: Option<f64>| {
            if let Some(v) = v {
                *value = v.round() as u8;
            }
            *value as f64
        })
        .max_decimals(0)
    }

    pub fn i32(value: &'a mut i32) -> Self {
        Self::from_get_set(move |v: Option<f64>| {
            if let Some(v) = v {
                *value = v.round() as i32;
            }
            *value as f64
        })
        .max_decimals(0)
    }

    /// How much the value changes when dragged one point (logical pixel).
    pub fn speed(mut self, speed: f32) -> Self {
        self.speed = speed;
        self
    }

    /// Clamp incoming and outgoing values to this range.
    pub fn clamp_range(mut self, clamp_range: RangeInclusive<f32>) -> Self {
        self.clamp_range = *clamp_range.start() as f64..=*clamp_range.end() as f64;
        self
    }

    #[deprecated = "Renamed clamp_range"]
    pub fn range(self, clamp_range: RangeInclusive<f32>) -> Self {
        self.clamp_range(clamp_range)
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
}

impl<'a> Widget for DragValue<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let Self {
            mut value_function,
            speed,
            clamp_range,
            prefix,
            suffix,
            min_decimals,
            max_decimals,
        } = self;

        let value = get(&mut value_function);
        let value = clamp(value, clamp_range.clone());
        let aim_rad = ui.input().physical_pixel_size(); // ui.input().aim_radius(); // TODO
        let auto_decimals = (aim_rad / speed.abs()).log10().ceil().at_least(0.0) as usize;
        let max_decimals = max_decimals.unwrap_or(auto_decimals + 2);
        let auto_decimals = clamp(auto_decimals, min_decimals..=max_decimals);
        let value_text = math::format_with_decimals_in_range(value, auto_decimals..=max_decimals);

        let kb_edit_id = ui.auto_id_with("edit");
        let is_kb_editing = ui.memory().has_kb_focus(kb_edit_id);

        if is_kb_editing {
            let button_width = ui.style().spacing.interact_size.x;
            let mut value_text = ui.memory().temp_edit_string.take().unwrap_or(value_text);
            let response = ui.add(
                TextEdit::singleline(&mut value_text)
                    .id(kb_edit_id)
                    .desired_width(button_width)
                    .text_style(TextStyle::Monospace),
            );
            if let Ok(parsed_value) = value_text.parse() {
                let parsed_value = clamp(parsed_value, clamp_range);
                set(&mut value_function, parsed_value)
            }
            if ui.input().key_pressed(Key::Enter) {
                ui.memory().surrender_kb_focus(kb_edit_id);
            } else {
                ui.memory().temp_edit_string = Some(value_text);
            }
            response
        } else {
            let button = Button::new(format!("{}{}{}", prefix, value_text, suffix))
                .sense(Sense::click_and_drag())
                .text_style(TextStyle::Monospace);
            let response = ui.add(button);
            let response = response.on_hover_text(format!(
                "{}{}{}\nDrag to edit or click to enter a value.",
                prefix,
                value as f32, // Show full precision value on-hover. TODO: figure out f64 vs f32
                suffix
            ));
            if response.clicked() {
                ui.memory().request_kb_focus(kb_edit_id);
                ui.memory().temp_edit_string = None; // Filled in next frame
            } else if response.dragged() {
                let mdelta = ui.input().pointer.delta();
                let delta_points = mdelta.x - mdelta.y; // Increase to the right and up
                let delta_value = speed * delta_points;
                if delta_value != 0.0 {
                    let new_value = value + delta_value as f64;
                    let new_value = math::round_to_decimals(new_value, auto_decimals);
                    let new_value = clamp(new_value, clamp_range);
                    set(&mut value_function, new_value);
                    // TODO: To make use or `smart_aim` for `DragValue` we need to store some state somewhere,
                    // otherwise we will just keep rounding to the same value while moving the mouse.
                }
            }
            response
        }
    }
}
