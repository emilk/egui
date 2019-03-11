#![allow(clippy::new_without_default_derive)]

use crate::{
    fonts::TextStyle,
    layout::{make_id, Align, Direction, GuiResponse, Id, Region},
    math::{remap_clamp, vec2, Vec2},
    types::{Color, GuiCmd, PaintCmd},
};

// ----------------------------------------------------------------------------

/// Anything implementing Widget can be added to a Region with Region::add
pub trait Widget {
    fn add_to(self, region: &mut Region) -> GuiResponse;
}

// ----------------------------------------------------------------------------

pub struct Label {
    text: String,
    text_style: TextStyle,
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
        let (text, text_size) = font.layout_multiline(&self.text, region.width());
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
        let id = region.make_child_id(&self.text);
        let text_style = TextStyle::Button;
        let font = &region.fonts()[text_style];
        let (text, text_size) = font.layout_multiline(&self.text, region.width());
        let padding = region.style().button_padding;
        let mut size = text_size + 2.0 * padding;
        size.y = size.y.max(region.style().clickable_diameter);
        let interact = region.reserve_space(size, Some(id));
        let text_cursor = interact.rect.left_center() + vec2(padding.x, -0.5 * text_size.y);
        region.add_graphic(GuiCmd::Button { interact });
        region.add_text(text_cursor, text_style, text, self.text_color);
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
        let id = region.make_child_id(&self.text);
        let text_style = TextStyle::Button;
        let font = &region.fonts()[text_style];
        let (text, text_size) = font.layout_multiline(&self.text, region.width());
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
        region.add_graphic(GuiCmd::Checkbox {
            checked: *self.checked,
            interact,
        });
        region.add_text(text_cursor, text_style, text, self.text_color);
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
        let id = region.make_child_id(&self.text);
        let text_style = TextStyle::Button;
        let font = &region.fonts()[text_style];
        let (text, text_size) = font.layout_multiline(&self.text, region.width());
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
        region.add_graphic(GuiCmd::RadioButton {
            checked: self.checked,
            interact,
        });
        region.add_text(text_cursor, text_style, text, self.text_color);
        region.response(interact)
    }
}

// ----------------------------------------------------------------------------

pub struct Slider<'a> {
    get_set_value: Box<'a + FnMut(Option<f32>) -> f32>,
    min: f32,
    max: f32,
    id: Option<Id>,
    text: Option<String>,
    precision: usize,
    text_color: Option<Color>,
    text_on_top: Option<bool>,
}

impl<'a> Slider<'a> {
    pub fn f32(value: &'a mut f32, min: f32, max: f32) -> Self {
        Slider {
            get_set_value: Box::new(move |v: Option<f32>| {
                if let Some(v) = v {
                    *value = v
                }
                *value
            }),
            min,
            max,
            id: None,
            text: None,
            precision: 3,
            text_on_top: None,
            text_color: None,
        }
    }

    pub fn i32(value: &'a mut i32, min: i32, max: i32) -> Self {
        Slider {
            get_set_value: Box::new(move |v: Option<f32>| {
                if let Some(v) = v {
                    *value = v.round() as i32
                }
                *value as f32
            }),
            min: min as f32,
            max: max as f32,
            id: None,
            text: None,
            precision: 0,
            text_on_top: None,
            text_color: None,
        }
    }

    pub fn usize(value: &'a mut usize, min: usize, max: usize) -> Self {
        Slider {
            get_set_value: Box::new(move |v: Option<f32>| {
                if let Some(v) = v {
                    *value = v.round() as usize
                }
                *value as f32
            }),
            min: min as f32,
            max: max as f32,
            id: None,
            text: None,
            precision: 0,
            text_on_top: None,
            text_color: None,
        }
    }

    pub fn id(mut self, id: Id) -> Self {
        self.id = Some(id);
        self
    }

    pub fn text<S: Into<String>>(mut self, text: S) -> Self {
        self.text = Some(text.into());
        self
    }

    pub fn text_color(mut self, text_color: Color) -> Self {
        self.text_color = Some(text_color);
        self
    }
}

impl<'a> Widget for Slider<'a> {
    fn add_to(mut self, region: &mut Region) -> GuiResponse {
        let text_style = TextStyle::Button;
        let font = &region.fonts()[text_style];

        if let Some(text) = &self.text {
            let text_on_top = self.text_on_top.unwrap_or_default();
            let text_color = self.text_color;
            let full_text = format!(
                "{}: {:.*}",
                text,
                self.precision,
                (self.get_set_value)(None)
            );
            let id = Some(self.id.unwrap_or_else(|| make_id(text)));
            let mut naked = self;
            naked.id = id;
            naked.text = None;

            if text_on_top {
                let (text, text_size) = font.layout_multiline(&full_text, region.width());
                let pos = region.reserve_space_without_padding(text_size);
                region.add_text(pos, text_style, text, text_color);
                naked.add_to(region)
            } else {
                region.columns(2, |columns| {
                    let response = naked.add_to(&mut columns[0]);

                    columns[1].available_space.y = response.rect.size().y;
                    columns[1].horizontal(Align::Center, |region| {
                        region.add(Label::new(full_text));
                    });

                    response
                })
            }
        } else {
            let height = font.line_spacing().max(region.style().clickable_diameter);

            let min = self.min;
            let max = self.max;
            debug_assert!(min <= max);
            let id = region.combined_id(Some(self.id.unwrap_or(42))); // TODO: slider ID
            let interact = region.reserve_space(
                Vec2 {
                    x: region.available_space.x,
                    y: height,
                },
                id,
            );

            if let Some(mouse_pos) = region.input().mouse_pos {
                if interact.active {
                    (self.get_set_value)(Some(remap_clamp(
                        mouse_pos.x,
                        interact.rect.min().x,
                        interact.rect.max().x,
                        min,
                        max,
                    )));
                }
            }

            region.add_graphic(GuiCmd::Slider {
                interact,
                max,
                min,
                value: (self.get_set_value)(None),
            });

            region.response(interact)
        }
    }
}

// ----------------------------------------------------------------------------

pub struct Separator {
    line_width: f32,
    width: f32,
}

impl Separator {
    pub fn new() -> Separator {
        Separator {
            line_width: 2.0,
            width: 6.0,
        }
    }

    pub fn line_width(&mut self, line_width: f32) -> &mut Self {
        self.line_width = line_width;
        self
    }

    pub fn width(&mut self, width: f32) -> &mut Self {
        self.width = width;
        self
    }
}

impl Widget for Separator {
    fn add_to(self, region: &mut Region) -> GuiResponse {
        let available_space = region.available_space;
        let (points, interact) = match region.direction() {
            Direction::Horizontal => {
                let interact = region.reserve_space(vec2(self.width, available_space.y), None);
                (
                    vec![
                        vec2(interact.rect.center().x, interact.rect.min().y),
                        vec2(interact.rect.center().x, interact.rect.max().y),
                    ],
                    interact,
                )
            }
            Direction::Vertical => {
                let interact = region.reserve_space(vec2(available_space.x, self.width), None);
                (
                    vec![
                        vec2(interact.rect.min().x, interact.rect.center().y),
                        vec2(interact.rect.max().x, interact.rect.center().y),
                    ],
                    interact,
                )
            }
        };
        let paint_cmd = PaintCmd::Line {
            points,
            color: Color::WHITE,
            width: self.line_width,
        };
        region.add_graphic(GuiCmd::PaintCommands(vec![paint_cmd]));
        region.response(interact)
    }
}
