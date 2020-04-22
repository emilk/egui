#![allow(clippy::new_without_default_derive)]

use crate::{
    layout::{Direction, GuiResponse},
    *,
};

// ----------------------------------------------------------------------------

/// Anything implementing Widget can be added to a Region with Region::add
pub trait Widget {
    fn add_to(self, region: &mut Region) -> GuiResponse;
}

// ----------------------------------------------------------------------------

pub struct Label {
    text: String,
    text_style: TextStyle, // TODO: Option<TextStyle>, where None means "use the default for the region"
    text_color: Option<Color>,
}

impl Label {
    pub fn new<S: Into<String>>(text: S) -> Self {
        Label {
            text: text.into(),
            text_style: TextStyle::Body,
            text_color: None,
        }
    }

    pub fn text_style(mut self, text_style: TextStyle) -> Self {
        self.text_style = text_style;
        self
    }

    pub fn text_color(mut self, text_color: Color) -> Self {
        self.text_color = Some(text_color);
        self
    }
}

/// Usage:  label!("Foo: {}", bar)
#[macro_export]
macro_rules! label {
    ($fmt:expr) => (Label::new($fmt));
    ($fmt:expr, $($arg:tt)*) => (Label::new(format!($fmt, $($arg)*)));
}

impl Widget for Label {
    fn add_to(self, region: &mut Region) -> GuiResponse {
        let font = &region.fonts()[self.text_style];
        let (text, text_size) = font.layout_multiline(&self.text, region.available_width());
        let interact = region.reserve_space(text_size, None);
        region.add_text(interact.rect.min(), self.text_style, text, self.text_color);
        region.response(interact)
    }
}

// ----------------------------------------------------------------------------

pub struct Button {
    text: String,
    text_color: Option<Color>,
}

impl Button {
    pub fn new<S: Into<String>>(text: S) -> Self {
        Button {
            text: text.into(),
            text_color: None,
        }
    }

    pub fn text_color(mut self, text_color: Color) -> Self {
        self.text_color = Some(text_color);
        self
    }
}

impl Widget for Button {
    fn add_to(self, region: &mut Region) -> GuiResponse {
        let id = region.make_position_id();
        let text_style = TextStyle::Button;
        let font = &region.fonts()[text_style];
        let (text, text_size) = font.layout_multiline(&self.text, region.available_width());
        let padding = region.style().button_padding;
        let mut size = text_size + 2.0 * padding;
        size.y = size.y.max(region.style().clickable_diameter);
        let interact = region.reserve_space(size, Some(id));
        let mut text_cursor = interact.rect.left_center() + vec2(padding.x, -0.5 * text_size.y);
        text_cursor.y += 2.0; // TODO: why is this needed?
        region.add_paint_cmd(PaintCmd::Rect {
            corner_radius: region.style().interact_corner_radius(&interact),
            fill_color: region.style().interact_fill_color(&interact),
            outline: region.style().interact_outline(&interact),
            rect: interact.rect,
        });
        let stroke_color = region.style().interact_stroke_color(&interact);
        let text_color = self.text_color.unwrap_or(stroke_color);
        region.add_text(text_cursor, text_style, text, Some(text_color));
        region.response(interact)
    }
}

// ----------------------------------------------------------------------------

#[derive(Debug)]
pub struct Checkbox<'a> {
    checked: &'a mut bool,
    text: String,
    text_color: Option<Color>,
}

impl<'a> Checkbox<'a> {
    pub fn new<S: Into<String>>(checked: &'a mut bool, text: S) -> Self {
        Checkbox {
            checked,
            text: text.into(),
            text_color: None,
        }
    }

    pub fn text_color(mut self, text_color: Color) -> Self {
        self.text_color = Some(text_color);
        self
    }
}

impl<'a> Widget for Checkbox<'a> {
    fn add_to(self, region: &mut Region) -> GuiResponse {
        let id = region.make_position_id();
        let text_style = TextStyle::Button;
        let font = &region.fonts()[text_style];
        let (text, text_size) = font.layout_multiline(&self.text, region.available_width());
        let interact = region.reserve_space(
            region.style().button_padding
                + vec2(region.style().start_icon_width, 0.0)
                + text_size
                + region.style().button_padding,
            Some(id),
        );
        let text_cursor = interact.rect.min()
            + region.style().button_padding
            + vec2(region.style().start_icon_width, 0.0);
        if interact.clicked {
            *self.checked = !*self.checked;
        }
        let (small_icon_rect, big_icon_rect) = region.style().icon_rectangles(&interact.rect);
        region.add_paint_cmd(PaintCmd::Rect {
            corner_radius: 3.0,
            fill_color: region.style().interact_fill_color(&interact),
            outline: None,
            rect: big_icon_rect,
        });

        let stroke_color = region.style().interact_stroke_color(&interact);

        if *self.checked {
            region.add_paint_cmd(PaintCmd::Line {
                points: vec![
                    pos2(small_icon_rect.left(), small_icon_rect.center().y),
                    pos2(small_icon_rect.center().x, small_icon_rect.bottom()),
                    pos2(small_icon_rect.right(), small_icon_rect.top()),
                ],
                color: stroke_color,
                width: region.style().line_width,
            });
        }

        let text_color = self.text_color.unwrap_or(stroke_color);
        region.add_text(text_cursor, text_style, text, Some(text_color));
        region.response(interact)
    }
}

// ----------------------------------------------------------------------------

#[derive(Debug)]
pub struct RadioButton {
    checked: bool,
    text: String,
    text_color: Option<Color>,
}

impl RadioButton {
    pub fn new<S: Into<String>>(checked: bool, text: S) -> Self {
        RadioButton {
            checked,
            text: text.into(),
            text_color: None,
        }
    }

    pub fn text_color(mut self, text_color: Color) -> Self {
        self.text_color = Some(text_color);
        self
    }
}

pub fn radio<S: Into<String>>(checked: bool, text: S) -> RadioButton {
    RadioButton::new(checked, text)
}

impl Widget for RadioButton {
    fn add_to(self, region: &mut Region) -> GuiResponse {
        let id = region.make_position_id();
        let text_style = TextStyle::Button;
        let font = &region.fonts()[text_style];
        let (text, text_size) = font.layout_multiline(&self.text, region.available_width());
        let interact = region.reserve_space(
            region.style().button_padding
                + vec2(region.style().start_icon_width, 0.0)
                + text_size
                + region.style().button_padding,
            Some(id),
        );
        let text_cursor = interact.rect.min()
            + region.style().button_padding
            + vec2(region.style().start_icon_width, 0.0);

        let fill_color = region.style().interact_fill_color(&interact);
        let stroke_color = region.style().interact_stroke_color(&interact);

        let (small_icon_rect, big_icon_rect) = region.style().icon_rectangles(&interact.rect);

        region.add_paint_cmd(PaintCmd::Circle {
            center: big_icon_rect.center(),
            fill_color,
            outline: None,
            radius: big_icon_rect.width() / 2.0,
        });

        if self.checked {
            region.add_paint_cmd(PaintCmd::Circle {
                center: small_icon_rect.center(),
                fill_color: Some(stroke_color),
                outline: None,
                radius: small_icon_rect.width() / 2.0,
            });
        }

        let text_color = self.text_color.unwrap_or(stroke_color);
        region.add_text(text_cursor, text_style, text, Some(text_color));
        region.response(interact)
    }
}

// ----------------------------------------------------------------------------

/// Combined into one function (rather than two) to make it easier
/// for the borrow checker.
type SliderGetSet<'a> = Box<dyn 'a + FnMut(Option<f32>) -> f32>;

pub struct Slider<'a> {
    get_set_value: SliderGetSet<'a>,
    min: f32,
    max: f32,
    text: Option<String>,
    precision: usize,
    text_color: Option<Color>,
    text_on_top: Option<bool>,
}

impl<'a> Slider<'a> {
    fn from_get_set(get_set_value: impl 'a + FnMut(Option<f32>) -> f32) -> Self {
        Slider {
            get_set_value: Box::new(get_set_value),
            min: std::f32::NAN,
            max: std::f32::NAN,
            text: None,
            precision: 3,
            text_on_top: None,
            text_color: None,
        }
    }

    pub fn f32(value: &'a mut f32, min: f32, max: f32) -> Self {
        Slider {
            min,
            max,
            precision: 3,
            ..Self::from_get_set(move |v: Option<f32>| {
                if let Some(v) = v {
                    *value = v
                }
                *value
            })
        }
    }

    pub fn i32(value: &'a mut i32, min: i32, max: i32) -> Self {
        Slider {
            min: min as f32,
            max: max as f32,
            precision: 0,
            ..Self::from_get_set(move |v: Option<f32>| {
                if let Some(v) = v {
                    *value = v.round() as i32
                }
                *value as f32
            })
        }
    }

    pub fn usize(value: &'a mut usize, min: usize, max: usize) -> Self {
        Slider {
            min: min as f32,
            max: max as f32,
            precision: 0,
            ..Self::from_get_set(move |v: Option<f32>| {
                if let Some(v) = v {
                    *value = v.round() as usize
                }
                *value as f32
            })
        }
    }

    pub fn text<S: Into<String>>(mut self, text: S) -> Self {
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
        if self.precision == 0 {
            value = value.round();
        }
        (self.get_set_value)(Some(value));
    }
}

impl<'a> Widget for Slider<'a> {
    fn add_to(mut self, region: &mut Region) -> GuiResponse {
        let text_style = TextStyle::Button;
        let font = &region.fonts()[text_style];

        if let Some(text) = &self.text {
            let text_on_top = self.text_on_top.unwrap_or_default();
            let text_color = self.text_color;
            let value = (self.get_set_value)(None);
            let full_text = format!("{}: {:.*}", text, self.precision, value);

            let slider_sans_text = Slider { text: None, ..self };

            if text_on_top {
                let (text, text_size) = font.layout_multiline(&full_text, region.available_width());
                let pos = region.reserve_space_without_padding(text_size);
                region.add_text(pos, text_style, text, text_color);
                slider_sans_text.add_to(region)
            } else {
                region.columns(2, |columns| {
                    // Slider on the left:
                    let slider_response = columns[0].add(slider_sans_text);

                    // Place the text in line with the slider on the left:
                    columns[1]
                        .desired_rect
                        .set_height(slider_response.rect.height());
                    columns[1].horizontal(Align::Center, |region| {
                        region.add(Label::new(full_text));
                    });

                    slider_response
                })
            }
        } else {
            let height = font.line_spacing().max(region.style().clickable_diameter);
            let handle_radius = height / 2.5;

            let id = region.make_position_id(); // TODO: causes problems for style settings :/

            let interact = region.reserve_space(
                Vec2 {
                    x: region.available_width(),
                    y: height,
                },
                Some(id),
            );

            let left = interact.rect.left() + handle_radius;
            let right = interact.rect.right() - handle_radius;

            let min = self.min;
            let max = self.max;
            debug_assert!(min <= max);

            if let Some(mouse_pos) = region.input().mouse_pos {
                if interact.active {
                    self.set_value_f32(remap_clamp(mouse_pos.x, left, right, min, max));
                }
            }

            // Paint it:
            {
                let value = self.get_value_f32();

                let rect = interact.rect;
                let rail_radius = (height / 8.0).round().max(2.0);
                let rail_rect = Rect::from_min_max(
                    pos2(interact.rect.left(), rect.center().y - rail_radius),
                    pos2(interact.rect.right(), rect.center().y + rail_radius),
                );
                let marker_center_x = remap_clamp(value, min, max, left, right);

                region.add_paint_cmd(PaintCmd::Rect {
                    rect: rail_rect,
                    corner_radius: rail_radius,
                    fill_color: Some(region.style().background_fill_color()),
                    outline: Some(Outline::new(1.0, color::gray(200, 255))), // TODO
                });

                region.add_paint_cmd(PaintCmd::Circle {
                    center: pos2(marker_center_x, rail_rect.center().y),
                    radius: handle_radius,
                    fill_color: region.style().interact_fill_color(&interact),
                    outline: Some(Outline::new(
                        region.style().interact_stroke_width(&interact),
                        region.style().interact_stroke_color(&interact),
                    )),
                });
            }

            region.response(interact)
        }
    }
}

// ----------------------------------------------------------------------------

pub struct Separator {
    line_width: f32,
    min_length: f32,
    extra: f32,
    color: Color,
}

impl Separator {
    pub fn new() -> Separator {
        Separator {
            line_width: 2.0,
            min_length: 6.0,
            extra: 0.0,
            color: color::WHITE,
        }
    }

    pub fn line_width(mut self, line_width: f32) -> Self {
        self.line_width = line_width;
        self
    }

    pub fn min_length(mut self, min_length: f32) -> Self {
        self.min_length = min_length;
        self
    }

    /// Draw this much longer on each side
    pub fn extra(mut self, extra: f32) -> Self {
        self.extra = extra;
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }
}

impl Widget for Separator {
    fn add_to(self, region: &mut Region) -> GuiResponse {
        let available_space = region.available_space();
        let extra = self.extra;
        let (points, interact) = match region.direction() {
            Direction::Horizontal => {
                let interact = region.reserve_space(vec2(self.min_length, available_space.y), None);
                (
                    vec![
                        pos2(interact.rect.center().x, interact.rect.top() - extra),
                        pos2(interact.rect.center().x, interact.rect.bottom() + extra),
                    ],
                    interact,
                )
            }
            Direction::Vertical => {
                let interact = region.reserve_space(vec2(available_space.x, self.min_length), None);
                (
                    vec![
                        pos2(interact.rect.left() - extra, interact.rect.center().y),
                        pos2(interact.rect.right() + extra, interact.rect.center().y),
                    ],
                    interact,
                )
            }
        };
        region.add_paint_cmd(PaintCmd::Line {
            points,
            color: self.color,
            width: self.line_width,
        });
        region.response(interact)
    }
}
