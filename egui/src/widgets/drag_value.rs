use std::ops::RangeInclusive;

use crate::{paint::*, *};

/// Combined into one function (rather than two) to make it easier
/// for the borrow checker.
type GetSetValue<'a> = Box<dyn 'a + FnMut(Option<f64>) -> f64>;

fn get(value_function: &mut GetSetValue<'_>) -> f64 {
    (value_function)(None)
}

fn set(value_function: &mut GetSetValue<'_>, value: f64) {
    (value_function)(Some(value));
}

/// A floating point value that you can change by dragging the number. More compact than a slider.
pub struct DragValue<'a> {
    value_function: GetSetValue<'a>,
    speed: f32,
    prefix: String,
    suffix: String,
    range: RangeInclusive<f64>,
}

impl<'a> DragValue<'a> {
    pub(crate) fn from_get_set(value_function: impl 'a + FnMut(Option<f64>) -> f64) -> Self {
        Self {
            value_function: Box::new(value_function),
            speed: 1.0,
            prefix: Default::default(),
            suffix: Default::default(),
            range: f64::NEG_INFINITY..=f64::INFINITY,
        }
    }

    pub fn f32(value: &'a mut f32) -> Self {
        Self {
            ..Self::from_get_set(move |v: Option<f64>| {
                if let Some(v) = v {
                    *value = v as f32
                }
                *value as f64
            })
        }
    }

    pub fn f64(value: &'a mut f64) -> Self {
        Self {
            ..Self::from_get_set(move |v: Option<f64>| {
                if let Some(v) = v {
                    *value = v
                }
                *value
            })
        }
    }

    pub fn u8(value: &'a mut u8) -> Self {
        Self {
            ..Self::from_get_set(move |v: Option<f64>| {
                if let Some(v) = v {
                    *value = v.round() as u8;
                }
                *value as f64
            })
        }
    }

    pub fn i32(value: &'a mut i32) -> Self {
        Self {
            ..Self::from_get_set(move |v: Option<f64>| {
                if let Some(v) = v {
                    *value = v.round() as i32;
                }
                *value as f64
            })
        }
    }

    /// How much the value changes when dragged one point (logical pixel).
    pub fn speed(mut self, speed: f32) -> Self {
        self.speed = speed;
        self
    }

    /// Clamp the value to this range
    pub fn range(mut self, range: RangeInclusive<f32>) -> Self {
        self.range = *range.start() as f64..=*range.end() as f64;
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
}

impl<'a> Widget for DragValue<'a> {
    fn ui(self, ui: &mut Ui) -> Response {
        let Self {
            mut value_function,
            speed,
            range,
            prefix,
            suffix,
        } = self;
        let value = get(&mut value_function);
        let aim_rad = ui.input().physical_pixel_size(); // ui.input().aim_radius(); // TODO
        let precision = (aim_rad / speed.abs()).log10().ceil().at_least(0.0) as usize;
        let value_text = format_with_minimum_precision(value as f32, precision); //  TODO: full precision

        let kb_edit_id = ui.make_position_id().with("edit");
        let is_kb_editing = ui.memory().has_kb_focus(kb_edit_id);

        if is_kb_editing {
            let button_width = ui.style().spacing.interact_size.x;
            let mut value_text = ui
                .memory()
                .temp_edit_string
                .take()
                .unwrap_or_else(|| value_text);
            let response = ui.add(
                TextEdit::singleline(&mut value_text)
                    .id(kb_edit_id)
                    .desired_width(button_width)
                    .text_style(TextStyle::Monospace),
            );
            if let Ok(parsed_value) = value_text.parse() {
                let parsed_value = clamp(parsed_value, range);
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
            // response.on_hover_text("Drag to edit, click to enter a value"); // TODO: may clash with users own tooltips
            if response.clicked {
                ui.memory().request_kb_focus(kb_edit_id);
                ui.memory().temp_edit_string = None; // Filled in next frame
            } else if response.active {
                let mdelta = ui.input().mouse.delta;
                let delta_points = mdelta.x - mdelta.y; // Increase to the right and up
                let delta_value = speed * delta_points;
                if delta_value != 0.0 {
                    let new_value = value + delta_value as f64;
                    let new_value = round_to_precision(new_value, precision);
                    let new_value = clamp(new_value, range);
                    set(&mut value_function, new_value);
                    // TODO: To make use or `smart_aim` for `DragValue` we need to store some state somewhere,
                    // otherwise we will just keep rounding to the same value while moving the mouse.
                }
            }
            response
        }
    }
}
