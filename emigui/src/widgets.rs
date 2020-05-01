#![allow(clippy::new_without_default)]

use crate::{
    layout::{Direction, GuiResponse},
    *,
};

mod slider;
mod text_edit;

pub use {slider::*, text_edit::*};

// ----------------------------------------------------------------------------

/// Anything implementing Widget can be added to a Region with Region::add
pub trait Widget {
    fn ui(self, region: &mut Region) -> GuiResponse;
}

// ----------------------------------------------------------------------------

pub struct Label {
    text: String,
    multiline: bool,
    text_style: TextStyle, // TODO: Option<TextStyle>, where None means "use the default for the region"
    text_color: Option<Color>,
}

impl Label {
    pub fn new(text: impl Into<String>) -> Self {
        Label {
            text: text.into(),
            multiline: true,
            text_style: TextStyle::Body,
            text_color: None,
        }
    }

    pub fn multiline(mut self, multiline: bool) -> Self {
        self.multiline = multiline;
        self
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
    fn ui(self, region: &mut Region) -> GuiResponse {
        let font = &region.fonts()[self.text_style];
        let (text, text_size) = if self.multiline {
            font.layout_multiline(&self.text, region.available_width())
        } else {
            font.layout_single_line(&self.text)
        };
        let interact = region.reserve_space(text_size, None);
        region.add_text(interact.rect.min, self.text_style, text, self.text_color);
        region.response(interact)
    }
}

// ----------------------------------------------------------------------------

pub struct Hyperlink {
    url: String,
    text: String,
}

impl Hyperlink {
    pub fn new(url: impl Into<String>) -> Self {
        let url = url.into();
        Self {
            text: url.clone(),
            url,
        }
    }
}

impl Widget for Hyperlink {
    fn ui(self, region: &mut Region) -> GuiResponse {
        let color = color::LIGHT_BLUE;
        let text_style = TextStyle::Body;
        let id = region.make_child_id(&self.url);
        let font = &region.fonts()[text_style];
        let line_spacing = font.line_spacing();
        // TODO: underline
        let (text, text_size) = font.layout_multiline(&self.text, region.available_width());
        let interact = region.reserve_space(text_size, Some(id));
        if interact.hovered {
            region.ctx().output.lock().cursor_icon = CursorIcon::PointingHand;
        }
        if interact.clicked {
            region.ctx().output.lock().open_url = Some(self.url);
        }

        if interact.hovered {
            // Underline:
            for fragment in &text {
                let pos = interact.rect.min;
                let y = pos.y + fragment.y_offset + line_spacing;
                let y = region.round_to_pixel(y);
                let min_x = pos.x + fragment.min_x();
                let max_x = pos.x + fragment.max_x();
                region.add_paint_cmd(PaintCmd::Line {
                    points: vec![pos2(min_x, y), pos2(max_x, y)],
                    color,
                    width: region.style().line_width,
                });
            }
        }

        region.add_text(interact.rect.min, text_style, text, Some(color));

        region.response(interact)
    }
}

// ----------------------------------------------------------------------------

pub struct Button {
    text: String,
    text_color: Option<Color>,
}

impl Button {
    pub fn new(text: impl Into<String>) -> Self {
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
    fn ui(self, region: &mut Region) -> GuiResponse {
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
    pub fn new(checked: &'a mut bool, text: impl Into<String>) -> Self {
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
    fn ui(self, region: &mut Region) -> GuiResponse {
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
        let text_cursor = interact.rect.min
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
    pub fn new(checked: bool, text: impl Into<String>) -> Self {
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

pub fn radio(checked: bool, text: impl Into<String>) -> RadioButton {
    RadioButton::new(checked, text)
}

impl Widget for RadioButton {
    fn ui(self, region: &mut Region) -> GuiResponse {
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
        let text_cursor = interact.rect.min
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
    fn ui(self, region: &mut Region) -> GuiResponse {
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
