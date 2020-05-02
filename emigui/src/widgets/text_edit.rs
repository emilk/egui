use crate::*;

#[derive(Debug)]
pub struct TextEdit<'t> {
    text: &'t mut String,
    id: Option<Id>,
    text_style: TextStyle, // TODO: Option<TextStyle>, where None means "use the default for the region"
    text_color: Option<Color>,
}

impl<'t> TextEdit<'t> {
    pub fn new(text: &'t mut String) -> Self {
        TextEdit {
            text,
            id: None,
            text_style: TextStyle::Body,
            text_color: Default::default(),
        }
    }

    pub fn id(mut self, id_source: impl std::hash::Hash) -> Self {
        self.id = Some(Id::new(id_source));
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

impl<'t> Widget for TextEdit<'t> {
    fn ui(self, region: &mut Region) -> GuiResponse {
        let id = region.make_child_id(self.id);

        let font = &region.fonts()[self.text_style];
        let line_spacing = font.line_spacing();
        let (text, text_size) = font.layout_multiline(self.text.as_str(), region.available_width());
        let desired_size = text_size.max(vec2(region.available_width(), line_spacing));
        let interact = region.reserve_space(desired_size, Some(id));

        if interact.clicked {
            region.request_kb_focus(id);
        }
        if interact.hovered {
            region.output().cursor_icon = CursorIcon::Text;
        }
        let has_kb_focus = region.has_kb_focus(id);

        if has_kb_focus {
            for event in &region.input().events {
                match event {
                    Event::Copy | Event::Cut => {
                        // TODO: cut
                        region.ctx().output().copied_text = self.text.clone();
                    }
                    Event::Text(text) => {
                        if text == "\u{7f}" {
                            // backspace
                        } else {
                            *self.text += text;
                        }
                    }
                    Event::Key { key, pressed: true } => {
                        match key {
                            Key::Backspace => {
                                self.text.pop(); // TODO: unicode aware
                            }
                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
        }

        region.add_paint_cmd(PaintCmd::Rect {
            rect: interact.rect,
            corner_radius: 0.0,
            // fill_color: Some(color::BLACK),
            fill_color: region.style().interact_fill_color(&interact),
            // fill_color: Some(region.style().background_fill_color()),
            outline: None, //Some(Outline::new(1.0, color::WHITE)),
        });

        if has_kb_focus {
            let cursor_blink_hz = region.style().cursor_blink_hz;
            let show_cursor =
                (region.input().time * cursor_blink_hz as f64 * 3.0).floor() as i64 % 3 != 0;
            if show_cursor {
                let cursor_pos = if let Some(last) = text.last() {
                    interact.rect.min + vec2(last.max_x(), last.y_offset)
                } else {
                    interact.rect.min
                };
                region.add_paint_cmd(PaintCmd::line_segment(
                    (cursor_pos, cursor_pos + vec2(0.0, line_spacing)),
                    color::WHITE,
                    region.style().text_cursor_width,
                ));
            }
        }

        region.add_text(interact.rect.min, self.text_style, text, self.text_color);

        region.response(interact)
    }
}
