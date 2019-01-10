use crate::{
    layout::{make_id, GuiResponse, Id, Region},
    math::{remap_clamp, vec2, Vec2},
    types::GuiCmd,
};

// ----------------------------------------------------------------------------

/// Anything implementing Widget can be added to a Region with Region::add
pub trait Widget {
    fn add_to(self, region: &mut Region) -> GuiResponse;
}

// ----------------------------------------------------------------------------

pub struct Label {
    text: String,
}

impl Label {
    pub fn new<S: Into<String>>(text: S) -> Self {
        Label { text: text.into() }
    }
}

pub fn label<S: Into<String>>(text: S) -> Label {
    Label::new(text)
}

impl Widget for Label {
    fn add_to(self, region: &mut Region) -> GuiResponse {
        let (text, text_size) = region.layout_text(&self.text);
        region.add_text(region.cursor(), text);
        let (_, interact) = region.reserve_space(text_size, None);
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
        let (text, text_size) = region.layout_text(&self.text);
        let text_cursor = region.cursor() + region.options().button_padding;
        let (rect, interact) =
            region.reserve_space(text_size + 2.0 * region.options().button_padding, Some(id));
        region.add_graphic(GuiCmd::Button { interact, rect });
        region.add_text(text_cursor, text);
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
        let (text, text_size) = region.layout_text(&self.text);
        let text_cursor = region.cursor()
            + region.options().button_padding
            + vec2(region.options().start_icon_width, 0.0);
        let (rect, interact) = region.reserve_space(
            region.options().button_padding
                + vec2(region.options().start_icon_width, 0.0)
                + text_size
                + region.options().button_padding,
            Some(id),
        );
        if interact.clicked {
            *self.checked = !*self.checked;
        }
        region.add_graphic(GuiCmd::Checkbox {
            checked: *self.checked,
            interact,
            rect,
        });
        region.add_text(text_cursor, text);
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
        let (text, text_size) = region.layout_text(&self.text);
        let text_cursor = region.cursor()
            + region.options().button_padding
            + vec2(region.options().start_icon_width, 0.0);
        let (rect, interact) = region.reserve_space(
            region.options().button_padding
                + vec2(region.options().start_icon_width, 0.0)
                + text_size
                + region.options().button_padding,
            Some(id),
        );
        region.add_graphic(GuiCmd::RadioButton {
            checked: self.checked,
            interact,
            rect,
        });
        region.add_text(text_cursor, text);
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
        if let Some(text) = &self.text {
            let text_on_top = self.text_on_top.unwrap_or_default();
            let full_text = format!("{}: {:.3}", text, self.value);
            let id = Some(self.id.unwrap_or(make_id(text)));
            let mut naked = self;
            naked.id = id;
            naked.text = None;

            if text_on_top {
                let (text, text_size) = region.layout_text(&full_text);
                region.add_text(region.cursor(), text);
                region.reserve_space_inner(text_size);
                naked.add_to(region)
            } else {
                region.columns(2, |columns| {
                    columns[1].add(label(full_text));
                    naked.add_to(&mut columns[0])
                })
            }
        } else {
            let value = self.value;
            let min = self.min;
            let max = self.max;
            debug_assert!(min <= max);
            let id = region.combined_id(self.id);
            let (slider_rect, interact) = region.reserve_space(
                Vec2 {
                    x: region.available_space.x,
                    y: region.data.font.line_spacing(),
                },
                id,
            );

            if interact.active {
                *value = remap_clamp(
                    region.input().mouse_pos.x,
                    slider_rect.min().x,
                    slider_rect.max().x,
                    min,
                    max,
                );
            }

            region.add_graphic(GuiCmd::Slider {
                interact,
                max,
                min,
                rect: slider_rect,
                value: *value,
            });

            region.response(interact)
        }
    }
}

// ----------------------------------------------------------------------------
