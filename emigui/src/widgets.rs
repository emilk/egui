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
}

impl Label {
    pub fn new<S: Into<String>>(text: S) -> Self {
        Label {
            text: text.into(),
            text_style: TextStyle::Body,
        }
    }

    pub fn text_style(mut self, text_style: TextStyle) -> Self {
        self.text_style = text_style;
        self
    }
}

pub fn label<S: Into<String>>(text: S) -> Label {
    Label::new(text)
}

impl Widget for Label {
    fn add_to(self, region: &mut Region) -> GuiResponse {
        let font = &region.fonts()[self.text_style];
        let (text, text_size) = font.layout_multiline(&self.text, region.width());
        let interact = region.reserve_space(text_size, None);
        region.add_text(interact.rect.min(), self.text_style, text);
        region.response(interact)
    }
}

// ----------------------------------------------------------------------------

pub struct Button {
    text: String,
}

impl Button {
    pub fn new<S: Into<String>>(text: S) -> Self {
        Button { text: text.into() }
    }
}

impl Widget for Button {
    fn add_to(self, region: &mut Region) -> GuiResponse {
        let id = region.make_child_id(&self.text);
        let text_style = TextStyle::Button;
        let font = &region.fonts()[text_style];
        let (text, text_size) = font.layout_multiline(&self.text, region.width());
        let interact =
            region.reserve_space(text_size + 2.0 * region.options().button_padding, Some(id));
        let text_cursor = interact.rect.min() + region.options().button_padding;
        region.add_graphic(GuiCmd::Button { interact });
        region.add_text(text_cursor, text_style, text);
        region.response(interact)
    }
}

// ----------------------------------------------------------------------------

#[derive(Debug)]
pub struct Checkbox<'a> {
    checked: &'a mut bool,
    text: String,
}

impl<'a> Checkbox<'a> {
    pub fn new<S: Into<String>>(checked: &'a mut bool, text: S) -> Self {
        Checkbox {
            checked,
            text: text.into(),
        }
    }
}

impl<'a> Widget for Checkbox<'a> {
    fn add_to(self, region: &mut Region) -> GuiResponse {
        let id = region.make_child_id(&self.text);
        let text_style = TextStyle::Button;
        let font = &region.fonts()[text_style];
        let (text, text_size) = font.layout_multiline(&self.text, region.width());
        let interact = region.reserve_space(
            region.options().button_padding
                + vec2(region.options().start_icon_width, 0.0)
                + text_size
                + region.options().button_padding,
            Some(id),
        );
        let text_cursor = interact.rect.min()
            + region.options().button_padding
            + vec2(region.options().start_icon_width, 0.0);
        if interact.clicked {
            *self.checked = !*self.checked;
        }
        region.add_graphic(GuiCmd::Checkbox {
            checked: *self.checked,
            interact,
        });
        region.add_text(text_cursor, text_style, text);
        region.response(interact)
    }
}

// ----------------------------------------------------------------------------

#[derive(Debug)]
pub struct RadioButton {
    checked: bool,
    text: String,
}

impl RadioButton {
    pub fn new<S: Into<String>>(checked: bool, text: S) -> Self {
        RadioButton {
            checked,
            text: text.into(),
        }
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
            region.options().button_padding
                + vec2(region.options().start_icon_width, 0.0)
                + text_size
                + region.options().button_padding,
            Some(id),
        );
        let text_cursor = interact.rect.min()
            + region.options().button_padding
            + vec2(region.options().start_icon_width, 0.0);
        region.add_graphic(GuiCmd::RadioButton {
            checked: self.checked,
            interact,
        });
        region.add_text(text_cursor, text_style, text);
        region.response(interact)
    }
}

// ----------------------------------------------------------------------------

#[derive(Debug)]
pub struct Slider<'a> {
    value: &'a mut f32,
    min: f32,
    max: f32,
    id: Option<Id>,
    text: Option<String>,
    text_on_top: Option<bool>,
}

impl<'a> Slider<'a> {
    pub fn new(value: &'a mut f32, min: f32, max: f32) -> Self {
        Slider {
            value,
            min,
            max,
            id: None,
            text: None,
            text_on_top: None,
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
}

impl<'a> Widget for Slider<'a> {
    fn add_to(self, region: &mut Region) -> GuiResponse {
        let text_style = TextStyle::Button;
        let font = &region.fonts()[text_style];

        if let Some(text) = &self.text {
            let text_on_top = self.text_on_top.unwrap_or_default();
            let full_text = format!("{}: {:.3}", text, self.value);
            let id = Some(self.id.unwrap_or(make_id(text)));
            let mut naked = self;
            naked.id = id;
            naked.text = None;

            if text_on_top {
                let (text, text_size) = font.layout_multiline(&full_text, region.width());
                let pos = region.reserve_space_without_padding(text_size);
                region.add_text(pos, text_style, text);
                naked.add_to(region)
            } else {
                region.columns(2, |columns| {
                    let response = naked.add_to(&mut columns[0]);

                    columns[1].available_space.y = response.rect.size().y;
                    columns[1].horizontal(Align::Center, |region| {
                        region.add(label(full_text));
                    });

                    response
                })
            }
        } else {
            let height = font.line_spacing().max(region.options().clickable_diameter);

            let value = self.value;
            let min = self.min;
            let max = self.max;
            debug_assert!(min <= max);
            let id = region.combined_id(self.id);
            let interact = region.reserve_space(
                Vec2 {
                    x: region.available_space.x,
                    y: height,
                },
                id,
            );

            if interact.active {
                *value = remap_clamp(
                    region.input().mouse_pos.x,
                    interact.rect.min().x,
                    interact.rect.max().x,
                    min,
                    max,
                );
            }

            region.add_graphic(GuiCmd::Slider {
                interact,
                max,
                min,
                value: *value,
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
