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
    text_on_top: Option<bool>,
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
            text_on_top: None,
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
}

impl<'a> Widget for Slider<'a> {
    fn ui(mut self, ui: &mut Ui) -> InteractInfo {
        let text_style = TextStyle::Button;
        let font = &ui.fonts()[text_style];

        if let Some(text) = &self.text {
            if self.id.is_none() {
                self.id = Some(ui.make_unique_child_id(text));
            }

            let text_on_top = self.text_on_top.unwrap_or_default();
            let text_color = self.text_color.unwrap_or_else(|| ui.style().text_color);
            let value = (self.get_set_value)(None);
            let full_text = format!("{}: {:.*}", text, self.precision, value);

            let slider_sans_text = Slider { text: None, ..self };

            if text_on_top {
                let galley = font.layout_single_line(full_text);
                let pos = ui.allocate_space(galley.size).min;
                ui.painter().galley(pos, galley, text_style, text_color);
                slider_sans_text.ui(ui)
            } else {
                ui.columns(2, |columns| {
                    // Slider on the left:
                    let slider_response = columns[0].add(slider_sans_text);

                    // Place the text in line with the slider on the left:
                    columns[1].set_desired_height(slider_response.rect.height());
                    columns[1].inner_layout(Layout::horizontal(Align::Center), |ui| {
                        ui.add(Label::new(full_text).multiline(false));
                    });

                    slider_response.into()
                })
            }
        } else {
            let height = font.line_spacing().max(ui.style().clickable_diameter);
            let handle_radius = height / 2.5;

            let id = self.id.unwrap_or_else(|| ui.make_position_id());

            let size = Vec2 {
                x: ui.available().width(),
                y: height,
            };
            let rect = ui.allocate_space(size);
            let interact = ui.interact(rect, id, Sense::click_and_drag());

            let left = interact.rect.left() + handle_radius;
            let right = interact.rect.right() - handle_radius;

            let range = self.range.clone();
            debug_assert!(range.start() <= range.end());

            if let Some(mouse_pos) = ui.input().mouse.pos {
                if interact.active {
                    self.set_value_f32(remap_clamp(mouse_pos.x, left..=right, range.clone()));
                }
            }

            // Paint it:
            {
                let value = self.get_value_f32();

                let rect = interact.rect;
                let rail_radius = ui.painter().round_to_pixel((height / 8.0).max(2.0));
                let rail_rect = Rect::from_min_max(
                    pos2(interact.rect.left(), rect.center().y - rail_radius),
                    pos2(interact.rect.right(), rect.center().y + rail_radius),
                );
                let marker_center_x = remap_clamp(value, range, left..=right);

                ui.painter().add(PaintCmd::Rect {
                    rect: rail_rect,
                    corner_radius: rail_radius,
                    fill: Some(ui.style().background_fill),
                    outline: Some(LineStyle::new(1.0, color::gray(200, 255))), // TODO
                });

                ui.painter().add(PaintCmd::Circle {
                    center: pos2(marker_center_x, rail_rect.center().y),
                    radius: handle_radius,
                    fill: Some(ui.style().interact(&interact).fill),
                    outline: Some(LineStyle::new(
                        ui.style().interact(&interact).stroke_width,
                        ui.style().interact(&interact).stroke_color,
                    )),
                });
            }

            interact
        }
    }
}
