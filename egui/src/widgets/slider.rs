use std::ops::RangeInclusive;

use crate::{paint::*, widgets::Label, *};

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

/// Control a number by a horizontal slider.
pub struct Slider<'a> {
    get_set_value: GetSetValue<'a>,
    range: RangeInclusive<f64>,
    // TODO: label: Option<Label>
    text: Option<String>,
    precision: Option<usize>,
    text_color: Option<Srgba>,
    id: Option<Id>,
}

impl<'a> Slider<'a> {
    fn from_get_set(
        range: RangeInclusive<f64>,
        get_set_value: impl 'a + FnMut(Option<f64>) -> f64,
    ) -> Self {
        Self {
            get_set_value: Box::new(get_set_value),
            range,
            text: None,
            precision: None,
            text_color: None,
            id: None,
        }
    }

    pub fn f32(value: &'a mut f32, range: RangeInclusive<f32>) -> Self {
        Self {
            ..Self::from_get_set(to_f64_range(range), move |v: Option<f64>| {
                if let Some(v) = v {
                    *value = v as f32
                }
                *value as f64
            })
        }
    }

    pub fn f64(value: &'a mut f64, range: RangeInclusive<f64>) -> Self {
        Self {
            ..Self::from_get_set(to_f64_range(range), move |v: Option<f64>| {
                if let Some(v) = v {
                    *value = v
                }
                *value
            })
        }
    }

    pub fn u8(value: &'a mut u8, range: RangeInclusive<u8>) -> Self {
        Self {
            precision: Some(0),
            ..Self::from_get_set(to_f64_range(range), move |v: Option<f64>| {
                if let Some(v) = v {
                    *value = v.round() as u8
                }
                *value as f64
            })
        }
    }

    pub fn i32(value: &'a mut i32, range: RangeInclusive<i32>) -> Self {
        Self {
            precision: Some(0),
            ..Self::from_get_set(to_f64_range(range), move |v: Option<f64>| {
                if let Some(v) = v {
                    *value = v.round() as i32
                }
                *value as f64
            })
        }
    }

    pub fn usize(value: &'a mut usize, range: RangeInclusive<usize>) -> Self {
        let range = (*range.start() as f64)..=(*range.end() as f64);
        Self {
            precision: Some(0),
            ..Self::from_get_set(range, move |v: Option<f64>| {
                if let Some(v) = v {
                    *value = v.round() as usize
                }
                *value as f64
            })
        }
    }

    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }

    pub fn text_color(mut self, text_color: Srgba) -> Self {
        self.text_color = Some(text_color);
        self
    }

    /// Precision (number of decimals) used when displaying the value.
    /// Values will also be rounded to this precision.
    /// Normally you don't need to pick a precision, as the slider will intelligently pick a precision for you.
    /// Regardless of precision the slider will use "smart aim" to help the user select nice, round values.
    pub fn precision(mut self, precision: usize) -> Self {
        self.precision = Some(precision);
        self
    }

    fn get_value(&mut self) -> f64 {
        get(&mut self.get_set_value)
    }

    fn set_value(&mut self, mut value: f64) {
        if let Some(precision) = self.precision {
            value = round_to_precision(value, precision);
        }
        set(&mut self.get_set_value, value);
    }

    /// For instance, `x` is the mouse position and `x_range` is the physical location of the slider on the screen.
    fn value_from_x_clamped(&self, x: f32, x_range: RangeInclusive<f32>) -> f64 {
        remap_clamp(x as f64, to_f64_range(x_range), self.range.clone())
    }

    fn value_from_x(&self, x: f32, x_range: RangeInclusive<f32>) -> f64 {
        remap(x as f64, to_f64_range(x_range), self.range.clone())
    }

    fn x_from_value(&self, value: f64, x_range: RangeInclusive<f32>) -> f32 {
        remap(value, self.range.clone(), to_f64_range(x_range)) as f32
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
    fn allocate_slide_space(&self, ui: &mut Ui, height: f32) -> Response {
        let id = self.id.unwrap_or_else(|| ui.make_position_id());
        let desired_size = vec2(ui.style().spacing.slider_width, height);
        let rect = ui.allocate_space(desired_size);
        ui.interact(rect, id, Sense::click_and_drag())
    }

    /// Just the slider, no text
    fn slider_ui(&mut self, ui: &mut Ui, response: &Response) {
        let rect = &response.rect;
        let x_range = x_range(rect);

        let range = self.range.clone();
        debug_assert!(range.start() <= range.end());

        if let Some(mouse_pos) = ui.input().mouse.pos {
            if response.active {
                let aim_radius = ui.input().aim_radius();
                let new_value = crate::math::smart_aim::best_in_range_f64(
                    self.value_from_x_clamped(mouse_pos.x - aim_radius, x_range.clone()),
                    self.value_from_x_clamped(mouse_pos.x + aim_radius, x_range.clone()),
                );
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

            ui.painter().add(PaintCmd::Rect {
                rect: rail_rect,
                corner_radius: rail_radius,
                fill: ui.style().visuals.widgets.inactive.bg_fill,
                stroke: ui.style().visuals.widgets.inactive.bg_stroke,
            });

            ui.painter().add(PaintCmd::Circle {
                center: pos2(marker_center_x, rail_rect.center().y),
                radius: handle_radius(rect),
                fill: ui.style().interact(response).fg_fill,
                stroke: ui.style().interact(response).fg_stroke,
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
        let kb_edit_id = self.id.expect("We should have an id by now").with("edit");
        let is_kb_editing = ui.memory().has_kb_focus(kb_edit_id);

        let aim_radius = ui.input().aim_radius();
        let value_text = self.format_value(aim_radius, x_range);

        if is_kb_editing {
            let button_width = ui.style().spacing.interact_size.x;
            let mut value_text = ui
                .memory()
                .temp_edit_string
                .take()
                .unwrap_or_else(|| value_text);
            ui.add(
                TextEdit::new(&mut value_text)
                    .id(kb_edit_id)
                    .multiline(false)
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
            let response = response.on_hover_text("Click to enter a value");
            // let response = ui.interact(response.rect, kb_edit_id, Sense::click());
            if response.clicked {
                ui.memory().request_kb_focus(kb_edit_id);
                ui.memory().temp_edit_string = None; // Filled in next frame
            }
        }
    }

    fn format_value(&mut self, aim_radius: f32, x_range: RangeInclusive<f32>) -> String {
        let value = self.get_value();

        let precision = self.precision.unwrap_or_else(|| {
            // pick precision based upon how much moving the slider would change the value:
            let value_from_x = |x: f32| self.value_from_x(x, x_range.clone());
            let x_from_value = |value: f64| self.x_from_value(value, x_range.clone());
            let left_value = value_from_x(x_from_value(value) - aim_radius);
            let right_value = value_from_x(x_from_value(value) + aim_radius);
            let range = (left_value - right_value).abs();
            (-range.log10()).ceil().max(0.0) as usize
        });

        format_with_minimum_precision(value as f32, precision)
    }
}

impl<'a> Widget for Slider<'a> {
    fn ui(mut self, ui: &mut Ui) -> Response {
        let text_style = TextStyle::Button;
        let font = &ui.fonts()[text_style];
        let height = font
            .line_spacing()
            .at_least(ui.style().spacing.interact_size.y);

        if let Some(text) = &self.text {
            self.id = self.id.or_else(|| Some(ui.make_unique_child_id(text)));

            ui.horizontal(|ui| {
                let slider_response = self.allocate_slide_space(ui, height);
                self.slider_ui(ui, &slider_response);
                let x_range = x_range(&slider_response.rect);
                self.value_ui(ui, x_range);
                self.label_ui(ui);
                slider_response
            })
            .0
        } else {
            let response = self.allocate_slide_space(ui, height);
            self.slider_ui(ui, &response);
            response
        }
    }
}
