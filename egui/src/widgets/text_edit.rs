use crate::{paint::*, *};

#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub(crate) struct State {
    /// Charctaer based, NOT bytes.
    /// TODO: store as line + row
    pub cursor: Option<usize>,
}

/// A text region that the user can edit the contents of.
#[derive(Debug)]
pub struct TextEdit<'t> {
    text: &'t mut String,
    id: Option<Id>,
    text_style: TextStyle, // TODO: Option<TextStyle>, where None means "use the default for the current Ui"
    text_color: Option<Color>,
    multiline: bool,
}

impl<'t> TextEdit<'t> {
    pub fn new(text: &'t mut String) -> Self {
        TextEdit {
            text,
            id: None,
            text_style: TextStyle::Body,
            text_color: Default::default(),
            multiline: true,
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

    pub fn multiline(mut self, multiline: bool) -> Self {
        self.multiline = multiline;
        self
    }
}

impl<'t> Widget for TextEdit<'t> {
    fn ui(self, ui: &mut Ui) -> InteractInfo {
        let TextEdit {
            text,
            id,
            text_style,
            text_color,
            multiline,
        } = self;

        let id = ui.make_child_id(id);

        let mut state = ui.memory().text_edit.get(&id).cloned().unwrap_or_default();

        let font = &ui.fonts()[text_style];
        let line_spacing = font.line_spacing();
        let available_width = ui.available().width();
        let mut galley = if multiline {
            font.layout_multiline(text.clone(), available_width)
        } else {
            font.layout_single_line(text.clone())
        };
        let desired_size = galley.size.max(vec2(available_width, line_spacing));
        let rect = ui.allocate_space(desired_size);
        let interact = ui.interact(rect, id, Sense::click_and_drag()); // TODO: implement drag-select

        if interact.clicked {
            ui.memory().request_kb_focus(id);
            if let Some(mouse_pos) = ui.input().mouse.pos {
                state.cursor = Some(galley.char_at(mouse_pos - interact.rect.min).char_idx);
            }
        } else if ui.input().mouse.click {
            // User clicked somewhere else
            ui.memory().surrender_kb_focus(id);
        }

        if interact.hovered {
            ui.output().cursor_icon = CursorIcon::Text;
        }

        if ui.memory().has_kb_focus(id) {
            let mut cursor = state.cursor.unwrap_or_else(|| text.chars().count());
            cursor = clamp(cursor, 0..=text.chars().count());

            for event in &ui.input().events {
                match event {
                    Event::Copy | Event::Cut => {
                        // TODO: cut
                        ui.ctx().output().copied_text = text.clone();
                    }
                    Event::Text(text_to_insert) => {
                        // newlines are handled by `Key::Enter`.
                        if text_to_insert != "\n" && text_to_insert != "\r" {
                            insert_text(&mut cursor, text, text_to_insert);
                        }
                    }
                    Event::Key {
                        key: Key::Enter,
                        pressed: true,
                    } => {
                        if multiline {
                            insert_text(&mut cursor, text, "\n");
                        }
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
            galley = if multiline {
                font.layout_multiline(text.clone(), available_width)
            } else {
                font.layout_single_line(text.clone())
            };

            // dbg!(&galley);
        }

        let painter = ui.painter();

        {
            let bg_rect = interact.rect.expand(2.0); // breathing room for content
            painter.add(PaintCmd::Rect {
                rect: bg_rect,
                corner_radius: ui.style().interact.style(&interact).corner_radius,
                fill: Some(ui.style().dark_bg_color),
                outline: ui.style().interact.style(&interact).rect_outline,
            });
        }

        if ui.memory().has_kb_focus(id) {
            let cursor_blink_hz = ui.style().cursor_blink_hz;
            let show_cursor = if let Some(cursor_blink_hz) = cursor_blink_hz {
                ui.ctx().request_repaint(); // TODO: only when cursor blinks on or off
                (ui.input().time * cursor_blink_hz as f64 * 3.0).floor() as i64 % 3 != 0
            } else {
                true
            };

            if show_cursor {
                if let Some(cursor) = state.cursor {
                    let cursor_pos = interact.rect.min + galley.char_start_pos(cursor);
                    painter.add(PaintCmd::line_segment(
                        [cursor_pos, cursor_pos + vec2(0.0, line_spacing)],
                        color::WHITE,
                        ui.style().text_cursor_width,
                    ));
                }
            }
        }

        let text_color = text_color.unwrap_or_else(|| ui.style().text_color);
        painter.galley(interact.rect.min, galley, text_style, text_color);
        ui.memory().text_edit.insert(id, state);
        interact
    }
}

fn insert_text(cursor: &mut usize, text: &mut String, text_to_insert: &str) {
    // eprintln!("insert_text {:?}", text_to_insert);

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
        Key::Enter => {} // handled earlier
        Key::Home => {
            // To start of paragraph:
            let pos = line_col_from_char_idx(text, *cursor);
            *cursor = char_idx_from_line_col(text, (pos.0, 0));
        }
        Key::End => {
            // To end of paragraph:
            let pos = line_col_from_char_idx(text, *cursor);
            let line = line_from_number(text, pos.0);
            *cursor = char_idx_from_line_col(text, (pos.0, line.chars().count()));
        }
        Key::Left if *cursor > 0 => {
            *cursor -= 1;
        }
        Key::Right => {
            *cursor = (*cursor + 1).min(text.chars().count());
        }
        Key::Up => {
            let mut pos = line_col_from_char_idx(text, *cursor);
            pos.0 = pos.0.saturating_sub(1);
            *cursor = char_idx_from_line_col(text, pos);
        }
        Key::Down => {
            let mut pos = line_col_from_char_idx(text, *cursor);
            pos.0 += 1;
            *cursor = char_idx_from_line_col(text, pos);
        }
        _ => {}
    }

    // eprintln!("on_key_press after:  '{}', cursor at {}\n", text, cursor);
}

fn line_col_from_char_idx(s: &str, char_idx: usize) -> (usize, usize) {
    let mut char_count = 0;

    let mut last_line_nr = 0;
    let mut last_line = s;
    for (line_nr, line) in s.split('\n').enumerate() {
        let line_width = line.chars().count();
        if char_idx <= char_count + line_width {
            return (line_nr, char_idx - char_count);
        }
        char_count += line_width + 1;
        last_line_nr = line_nr;
        last_line = line;
    }

    // safe fallback:
    (last_line_nr, last_line.chars().count())
}

fn char_idx_from_line_col(s: &str, pos: (usize, usize)) -> usize {
    let mut char_count = 0;
    for (line_nr, line) in s.split('\n').enumerate() {
        if line_nr == pos.0 {
            return char_count + pos.1.min(line.chars().count());
        }
        char_count += line.chars().count() + 1;
    }
    char_count
}

fn line_from_number(s: &str, desired_line_number: usize) -> &str {
    for (line_nr, line) in s.split('\n').enumerate() {
        if line_nr == desired_line_number {
            return line;
        }
    }
    s
}
