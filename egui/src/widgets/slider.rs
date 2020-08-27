use std::ops::RangeInclusive;

use crate::{paint::*, widgets::Label, *};

/// Combined into one function (rather than two) to make it easier
/// for the borrow checker.
type SliderGetSet<'a> = Box<dyn 'a + FnMut(Option<f32>) -> f32>;

/// Control a number by a horizontal slider.
pub struct Slider<'a> {
    get_set_value: SliderGetSet<'a>,
    range: RangeInclusive<f32>,
    // TODO: label: Option<Label>
    text: Option<String>,
    precision: usize,
    text_color: Option<Color>,
    id: Option<Id>,
}

impl<'a> Slider<'a> {
    fn from_get_set(
        range: RangeInclusive<f32>,
        get_set_value: impl 'a + FnMut(Option<f32>) -> f32,
    ) -> Self {
        Slider {
            get_set_value: Box::new(get_set_value),
            range,
            text: None,
            precision: 3,
            text_color: None,
            id: None,
        }
    }

    pub fn f32(value: &'a mut f32, range: RangeInclusive<f32>) -> Self {
        Slider {
            precision: 3,
            ..Self::from_get_set(range, move |v: Option<f32>| {
                if let Some(v) = v {
                    *value = v
                }
                *value
            })
        }
    }

    pub fn i32(value: &'a mut i32, range: RangeInclusive<i32>) -> Self {
        let range = (*range.start() as f32)..=(*range.end() as f32);
        Slider {
            precision: 0,
            ..Self::from_get_set(range, move |v: Option<f32>| {
                if let Some(v) = v {
                    *value = v.round() as i32
                }
                *value as f32
            })
        }
    }

    pub fn usize(value: &'a mut usize, range: RangeInclusive<usize>) -> Self {
        let range = (*range.start() as f32)..=(*range.end() as f32);
        Slider {
            precision: 0,
            ..Self::from_get_set(range, move |v: Option<f32>| {
                if let Some(v) = v {
                    *value = v.round() as usize
                }
                *value as f32
            })
        }
    }

    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.text = Some(text.into());
        self
    }

    pub fn text_color(mut self, text_color: Color) -> Self {
        self.text_color = Some(text_color);
        self
    }

    /// Precision (number of decimals) used when displaying the value.
    /// Values will also be rounded to this precision.
    /// Regardless of precision the slider will use "smart aim" to help the user select nice, round values.
    pub fn precision(mut self, precision: usize) -> Self {
        self.precision = precision;
        self
    }

    fn get_value_f32(&mut self) -> f32 {
        (self.get_set_value)(None)
    }

    fn set_value_f32(&mut self, mut value: f32) {
        value = round_to_precision(value, self.precision);
        (self.get_set_value)(Some(value));
    }

    /// For instance, `point` is the mouse position and `point_range` is the physical location of the slider on the screen.
    fn value_from_point(&self, point: f32, point_range: RangeInclusive<f32>) -> f32 {
        remap_clamp(point, point_range, self.range.clone())
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
    fn allocate_slide_space(&self, ui: &mut Ui, height: f32) -> InteractInfo {
        let id = self.id.unwrap_or_else(|| ui.make_position_id());
        let desired_size = vec2(ui.available().width(), height);
        let rect = ui.allocate_space(desired_size);
        ui.interact(rect, id, Sense::click_and_drag())
    }

    /// Just the slider, no text
    fn slider_ui(&mut self, ui: &mut Ui, interact: InteractInfo) -> InteractInfo {
        let rect = &interact.rect;
        let x_range = x_range(rect);

        let range = self.range.clone();
        debug_assert!(range.start() <= range.end());

        if let Some(mouse_pos) = ui.input().mouse.pos {
            if interact.active {
                let aim_radius = ui.input().aim_radius();
                let new_value = crate::math::smart_aim::best_in_range_f32(
                    self.value_from_point(mouse_pos.x - aim_radius, x_range.clone()),
                    self.value_from_point(mouse_pos.x + aim_radius, x_range.clone()),
                );
                self.set_value_f32(new_value);
            }
        }

        // Paint it:
        {
            let value = self.get_value_f32();

            let rail_radius = ui.painter().round_to_pixel((rect.height() / 8.0).max(2.0));
            let rail_rect = Rect::from_min_max(
                pos2(rect.left(), rect.center().y - rail_radius),
                pos2(rect.right(), rect.center().y + rail_radius),
            );
            let marker_center_x = remap_clamp(value, range, x_range);

            ui.painter().add(PaintCmd::Rect {
                rect: rail_rect,
                corner_radius: rail_radius,
                fill: Some(ui.style().background_fill),
                outline: Some(LineStyle::new(1.0, color::gray(200, 255))), // TODO
            });

            ui.painter().add(PaintCmd::Circle {
                center: pos2(marker_center_x, rail_rect.center().y),
                radius: handle_radius(rect),
                fill: Some(ui.style().interact(&interact).fill),
                outline: Some(LineStyle::new(
                    ui.style().interact(&interact).stroke_width,
                    ui.style().interact(&interact).stroke_color,
                )),
            });
        }

        interact
    }

    /// Just the text label
    fn text_ui(&mut self, ui: &mut Ui) {
        if let Some(text) = &self.text {
            let text_color = self.text_color.unwrap_or_else(|| ui.style().text_color);
            let value = (self.get_set_value)(None);
            let full_text = format!("{}: {:.*}", text, self.precision, value);
            ui.add(
                Label::new(full_text)
                    .multiline(false)
                    .text_color(text_color),
            );
        }
    }
}

impl<'a> Widget for Slider<'a> {
    fn ui(mut self, ui: &mut Ui) -> InteractInfo {
        let text_style = TextStyle::Button;
        let font = &ui.fonts()[text_style];
        let height = font.line_spacing().max(ui.style().clickable_diameter);

        if let Some(text) = &self.text {
            self.id = self.id.or_else(|| Some(ui.make_unique_child_id(text)));

            ui.columns(2, |columns| {
                let slider_ui = &mut columns[0];
                let interact = self.allocate_slide_space(slider_ui, height);
                let slider_interact = self.slider_ui(slider_ui, interact);

                // Place the text in line with the slider on the left:
                let text_ui = &mut columns[1];
                text_ui.set_desired_height(slider_interact.rect.height());
                text_ui.inner_layout(Layout::horizontal(Align::Center), |ui| {
                    self.text_ui(ui);
                });

                slider_interact
            })
        } else {
            let interact = self.allocate_slide_space(ui, height);
            self.slider_ui(ui, interact)
        }
    }
}
