use crate::*;

#[derive(Clone, Copy, Debug, Default, serde_derive::Deserialize, serde_derive::Serialize)]
pub(crate) struct State {
    /// Charctaer based, NOT bytes
    pub cursor: Option<usize>,
}

#[derive(Debug)]
pub struct TextEdit<'t> {
    text: &'t mut String,
    id: Option<Id>,
    text_style: TextStyle, // TODO: Option<TextStyle>, where None means "use the default for the current Ui"
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
    fn ui(self, ui: &mut Ui) -> GuiResponse {
        let TextEdit {
            text,
            id,
            text_style,
            text_color,
        } = self;

        let id = ui.make_child_id(id);

        let mut state = ui.memory().text_edit.get(&id).cloned().unwrap_or_default();

        let font = &ui.fonts()[text_style];
        let line_spacing = font.line_spacing();
        let available_width = ui.available().width();
        let mut galley = font.layout_multiline(text.as_str(), available_width);
        let desired_size = galley.size.max(vec2(available_width, line_spacing));
        let interact = ui.reserve_space(desired_size, Some(id));

        if interact.clicked {
            ui.request_kb_focus(id);
        }
        if interact.hovered {
            ui.output().cursor_icon = CursorIcon::Text;
        }
        let has_kb_focus = ui.has_kb_focus(id);

        if has_kb_focus {
            let mut cursor = state.cursor.unwrap_or_else(|| text.chars().count());
            cursor = clamp(cursor, 0..=text.chars().count());

            for event in &ui.input().events {
                match event {
                    Event::Copy | Event::Cut => {
                        // TODO: cut
                        ui.ctx().output().copied_text = text.clone();
                    }
                    Event::Text(text_to_insert) => {
                        insert_text(&mut cursor, text, text_to_insert);
                    }
                    Event::Key { key, pressed: true } => {
                        on_key_press(&mut cursor, text, *key);
                    }
                    _ => {}
                }
            }
            state.cursor = Some(cursor);

            // layout again to avoid frame delay:
            let font = &ui.fonts()[text_style];
            galley = font.layout_multiline(text.as_str(), available_width);

            // dbg!(&galley);
        }

        ui.add_paint_cmd(PaintCmd::Rect {
            rect: interact.rect,
            corner_radius: 0.0,
            // fill_color: Some(color::BLACK),
            fill_color: ui.style().interact(&interact).fill_color,
            // fill_color: Some(ui.style().background_fill_color()),
            outline: None, //Some(Outline::new(1.0, color::WHITE)),
        });

        if has_kb_focus {
            let cursor_blink_hz = ui.style().cursor_blink_hz;
            let show_cursor =
                (ui.input().time * cursor_blink_hz as f64 * 3.0).floor() as i64 % 3 != 0;
            if show_cursor {
                if let Some(cursor) = state.cursor {
                    let cursor_pos = interact.rect.min + galley.char_start_pos(cursor);
                    ui.add_paint_cmd(PaintCmd::line_segment(
                        [cursor_pos, cursor_pos + vec2(0.0, line_spacing)],
                        color::WHITE,
                        ui.style().text_cursor_width,
                    ));
                }
            }
        }

        ui.add_galley(interact.rect.min, galley, text_style, text_color);
        ui.memory().text_edit.insert(id, state);
        ui.response(interact)
    }
}

fn insert_text(cursor: &mut usize, text: &mut String, text_to_insert: &str) {
    // eprintln!("insert_text before: '{}', cursor at {}", text, cursor);

    let mut char_it = text.chars();
    let mut new_text = String::with_capacity(text.capacity());
    for _ in 0..*cursor {
        let c = char_it.next().unwrap();
        new_text.push(c);
    }
    *cursor += text_to_insert.chars().count();
    new_text += text_to_insert;
    new_text.extend(char_it);
    *text = new_text;

    // eprintln!("insert_text after:  '{}', cursor at {}\n", text, cursor);
}
fn on_key_press(cursor: &mut usize, text: &mut String, key: Key) {
    // eprintln!("on_key_press before: '{}', cursor at {}", text, cursor);

    match key {
        Key::Backspace if *cursor > 0 => {
            *cursor -= 1;

            let mut char_it = text.chars();
            let mut new_text = String::with_capacity(text.capacity());
            for _ in 0..*cursor {
                new_text.push(char_it.next().unwrap())
            }
            new_text.extend(char_it.skip(1));
            *text = new_text;
        }
        Key::Delete => {
            let mut char_it = text.chars();
            let mut new_text = String::with_capacity(text.capacity());
            for _ in 0..*cursor {
                new_text.push(char_it.next().unwrap())
            }
            new_text.extend(char_it.skip(1));
            *text = new_text;
        }
        Key::Home => {
            *cursor = 0; // TODO: start of line
        }
        Key::End => {
            *cursor = text.chars().count(); // TODO: end of line
        }
        Key::Left if *cursor > 0 => {
            *cursor -= 1;
        }
        Key::Right => {
            *cursor = (*cursor + 1).min(text.chars().count());
        }
        _ => {}
    }

    // eprintln!("on_key_press after:  '{}', cursor at {}\n", text, cursor);
}
