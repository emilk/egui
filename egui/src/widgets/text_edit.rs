use crate::{paint::*, *};

#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub(crate) struct State {
    /// We store as PCursor (paragraph number, and character offset within that paragraph).
    /// This is so what if we resize the `TextEdit` region, and text wrapping changes,
    /// we keep the same byte character offset from the beginning of the text,
    /// even though the number of rows changes
    /// (each paragraph can be several rows, due to word wrapping).
    /// The column (character offset) should be able to extend beyond the last word so that we can
    /// go down and still end up on the same column when we return.
    pcursor: Option<PCursor>,
}

// struct PCursorPair {
//     /// Where the selection started (e.g. a drag started).
//     begin: PCursor,
//     /// The end of the selection. When moving with e.g. shift+arrows, this is what moves.
//     /// Note that this may be BEFORE the `begin`.
//     end: PCursor,
// }

/// A text region that the user can edit the contents of.
///
/// Example:
///
/// ```
/// # let mut ui = egui::Ui::__test();
/// # let mut my_string = String::new();
/// let response = ui.add(egui::TextEdit::singleline(&mut my_string));
/// if response.lost_kb_focus {
///     // use my_string
/// }
/// ```
#[derive(Debug)]
pub struct TextEdit<'t> {
    text: &'t mut String,
    id: Option<Id>,
    id_source: Option<Id>,
    text_style: Option<TextStyle>,
    text_color: Option<Srgba>,
    multiline: bool,
    enabled: bool,
    desired_width: Option<f32>,
    desired_height_rows: usize,
}

impl<'t> TextEdit<'t> {
    #[deprecated = "Use `TextEdit::singleline` or `TextEdit::multiline` (or the helper `ui.text_edit_singleline`, `ui.text_edit_multiline`) instead"]
    pub fn new(text: &'t mut String) -> Self {
        Self::multiline(text)
    }

    /// Now newlines (`\n`) allowed. Pressing enter key will result in the `TextEdit` loosing focus (`response.lost_kb_focus`).
    pub fn singleline(text: &'t mut String) -> Self {
        TextEdit {
            text,
            id: None,
            id_source: None,
            text_style: None,
            text_color: None,
            multiline: false,
            enabled: true,
            desired_width: None,
            desired_height_rows: 1,
        }
    }

    /// A `TextEdit` for multiple lines. Pressing enter key will create a new line.
    pub fn multiline(text: &'t mut String) -> Self {
        TextEdit {
            text,
            id: None,
            id_source: None,
            text_style: None,
            text_color: None,
            multiline: true,
            enabled: true,
            desired_width: None,
            desired_height_rows: 4,
        }
    }

    pub fn id(mut self, id: Id) -> Self {
        self.id = Some(id);
        self
    }

    pub fn id_source(mut self, id_source: impl std::hash::Hash) -> Self {
        self.id_source = Some(Id::new(id_source));
        self
    }

    pub fn text_style(mut self, text_style: TextStyle) -> Self {
        self.text_style = Some(text_style);
        self
    }

    pub fn text_color(mut self, text_color: Srgba) -> Self {
        self.text_color = Some(text_color);
        self
    }

    pub fn text_color_opt(mut self, text_color: Option<Srgba>) -> Self {
        self.text_color = text_color;
        self
    }

    /// Default is `true`. If set to `false` then you cannot edit the text.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Set to 0.0 to keep as small as possible
    pub fn desired_width(mut self, desired_width: f32) -> Self {
        self.desired_width = Some(desired_width);
        self
    }

    /// Set the number of rows to show by default.
    /// The default for singleline text is `1`.
    /// The default for multiline text is `4`.
    pub fn desired_rows(mut self, desired_width: f32) -> Self {
        self.desired_width = Some(desired_width);
        self
    }
}

impl<'t> Widget for TextEdit<'t> {
    fn ui(self, ui: &mut Ui) -> Response {
        let TextEdit {
            text,
            id,
            id_source,
            text_style,
            text_color,
            multiline,
            enabled,
            desired_width,
            desired_height_rows,
        } = self;

        let id = id.unwrap_or_else(|| {
            if let Some(id_source) = id_source {
                ui.make_persistent_id(id_source)
            } else {
                // Since we are only storing cursor, perfect persistence Id not super important
                ui.make_position_id()
            }
        });

        let mut state = ui.memory().text_edit.get(&id).cloned().unwrap_or_default();

        let text_style = text_style.unwrap_or_else(|| ui.style().body_text_style);
        let font = &ui.fonts()[text_style];
        let line_spacing = font.row_height();
        let available_width = ui.available().width();
        let mut galley = if multiline {
            font.layout_multiline(text.clone(), available_width)
        } else {
            font.layout_single_line(text.clone())
        };

        let desired_width = desired_width.unwrap_or_else(|| ui.style().spacing.text_edit_width);
        let desired_height = (desired_height_rows.at_least(1) as f32) * line_spacing;
        let desired_size = vec2(
            galley.size.x.max(desired_width.min(available_width)),
            galley.size.y.max(desired_height),
        );
        let rect = ui.allocate_space(desired_size);
        let sense = if enabled {
            Sense::click_and_drag()
        } else {
            Sense::nothing()
        };
        let response = ui.interact(rect, id, sense);

        if response.clicked && enabled {
            ui.memory().request_kb_focus(id);
            if let Some(mouse_pos) = ui.input().mouse.pos {
                state.pcursor = Some(
                    galley
                        .cursor_from_pos(mouse_pos - response.rect.min)
                        .pcursor,
                );
            }
        } else if ui.input().mouse.click || (ui.input().mouse.pressed && !response.hovered) {
            // User clicked somewhere else
            ui.memory().surrender_kb_focus(id);
        }
        if !enabled {
            ui.memory().surrender_kb_focus(id);
        }

        if response.hovered && enabled {
            ui.output().cursor_icon = CursorIcon::Text;
        }

        if ui.memory().has_kb_focus(id) && enabled {
            let mut cursor = state
                .pcursor
                .map(|pcursor| galley.from_pcursor(pcursor))
                .unwrap_or_else(|| galley.end());

            for event in &ui.input().events {
                let did_mutate_text = match event {
                    Event::Copy | Event::Cut => {
                        // TODO: cut
                        ui.ctx().output().copied_text = text.clone();
                        None
                    }
                    Event::Text(text_to_insert) => {
                        // Newlines are handled by `Key::Enter`.
                        if !text_to_insert.is_empty()
                            && text_to_insert != "\n"
                            && text_to_insert != "\r"
                        {
                            let mut ccursor = cursor.ccursor;
                            insert_text(&mut ccursor, text, text_to_insert);
                            Some(ccursor)
                        } else {
                            None
                        }
                    }
                    Event::Key {
                        key: Key::Enter,
                        pressed: true,
                        ..
                    } => {
                        if multiline {
                            let mut ccursor = cursor.ccursor;
                            insert_text(&mut ccursor, text, "\n");
                            Some(ccursor)
                        } else {
                            // Common to end input with enter
                            ui.memory().surrender_kb_focus(id);
                            break;
                        }
                    }
                    Event::Key {
                        key: Key::Escape,
                        pressed: true,
                        ..
                    } => {
                        ui.memory().surrender_kb_focus(id);
                        break;
                    }
                    Event::Key {
                        key,
                        pressed: true,
                        modifiers,
                    } => on_key_press(&mut cursor, text, &galley, *key, modifiers),
                    Event::Key { .. } => None,
                };

                if let Some(new_ccursor) = did_mutate_text {
                    // Layout again to avoid frame delay, and to keep `text` and `galley` in sync.
                    let font = &ui.fonts()[text_style];
                    galley = if multiline {
                        font.layout_multiline(text.clone(), available_width)
                    } else {
                        font.layout_single_line(text.clone())
                    };

                    // Set cursor using new galley:
                    cursor = galley.from_ccursor(new_ccursor);
                }
            }
            state.pcursor = Some(cursor.pcursor);
        }

        let painter = ui.painter();
        let visuals = ui.style().interact(&response);

        {
            let bg_rect = response.rect.expand(2.0); // breathing room for content
            painter.add(PaintCmd::Rect {
                rect: bg_rect,
                corner_radius: visuals.corner_radius,
                fill: ui.style().visuals.dark_bg_color,
                // fill: visuals.bg_fill,
                stroke: visuals.bg_stroke,
            });
        }

        if ui.memory().has_kb_focus(id) {
            let cursor_blink_hz = ui.style().visuals.cursor_blink_hz;
            let show_cursor = if 0.0 < cursor_blink_hz {
                ui.ctx().request_repaint(); // TODO: only when cursor blinks on or off
                (ui.input().time * cursor_blink_hz as f64 * 3.0).floor() as i64 % 3 != 0
            } else {
                true
            };

            if show_cursor {
                if let Some(pcursor) = state.pcursor {
                    let cursor_pos = response.rect.min + galley.pos_from_pcursor(pcursor);
                    painter.line_segment(
                        [cursor_pos, cursor_pos + vec2(0.0, line_spacing)],
                        (ui.style().visuals.text_cursor_width, color::WHITE),
                    );
                }
            }
        }

        let text_color = text_color
            .or(ui.style().visuals.override_text_color)
            .unwrap_or_else(|| visuals.text_color());
        painter.galley(response.rect.min, galley, text_style, text_color);
        ui.memory().text_edit.insert(id, state);

        Response {
            lost_kb_focus: ui.memory().lost_kb_focus(id), // we may have lost it during the course of this function
            ..response
        }
    }
}

fn insert_text(ccursor: &mut CCursor, text: &mut String, text_to_insert: &str) {
    let mut char_it = text.chars();
    let mut new_text = String::with_capacity(text.capacity());
    for _ in 0..ccursor.index {
        let c = char_it.next().unwrap();
        new_text.push(c);
    }
    ccursor.index += text_to_insert.chars().count();
    new_text += text_to_insert;
    new_text.extend(char_it);
    *text = new_text;
}

/// Returns `Some(new_cursor)` if we did mutate `text`.
fn on_key_press(
    cursor: &mut Cursor,
    text: &mut String,
    galley: &Galley,
    key: Key,
    modifiers: &Modifiers,
) -> Option<CCursor> {
    // TODO: cursor position preview on mouse hover
    // TODO: drag-select
    // TODO: double-click to select whole word
    // TODO: triple-click to select whole paragraph
    // TODO: drag selected text to either move or clone (ctrl on windows, alt on mac)
    // TODO: ctrl-U to clear paragraph before the cursor
    // TODO: ctrl-W to delete previous word
    // TODO: alt/ctrl + backspace to delete previous word (alt on mac, ctrl on windows)
    // TODO: alt/ctrl + delete to delete next word (alt on mac, ctrl on windows)
    // TODO: cmd-A to select all
    // TODO: shift modifier to only move half of the cursor to select things

    match key {
        Key::Backspace => {
            if cursor.ccursor.index > 0 {
                *cursor = galley.from_ccursor(cursor.ccursor - 1);
                let mut char_it = text.chars();
                let mut new_text = String::with_capacity(text.capacity());
                for _ in 0..cursor.ccursor.index {
                    new_text.push(char_it.next().unwrap())
                }
                new_text.extend(char_it.skip(1));
                *text = new_text;
                Some(cursor.ccursor)
            } else {
                None
            }
        }
        Key::Delete => {
            let mut char_it = text.chars();
            let mut new_text = String::with_capacity(text.capacity());
            for _ in 0..cursor.ccursor.index {
                new_text.push(char_it.next().unwrap())
            }
            new_text.extend(char_it.skip(1));
            *text = new_text;
            Some(cursor.ccursor)
        }

        Key::ArrowLeft => {
            if modifiers.alt || modifiers.ctrl {
                // alt on mac, ctrl on windows
                *cursor = galley.cursor_previous_word(cursor);
            } else if modifiers.mac_cmd {
                *cursor = galley.cursor_begin_of_row(cursor);
            } else {
                *cursor = galley.cursor_left_one_character(cursor);
            }
            None
        }
        Key::ArrowRight => {
            if modifiers.alt || modifiers.ctrl {
                // alt on mac, ctrl on windows
                *cursor = galley.cursor_next_word(cursor);
            } else if modifiers.mac_cmd {
                *cursor = galley.cursor_end_of_row(cursor);
            } else {
                *cursor = galley.cursor_right_one_character(cursor);
            }
            None
        }
        Key::ArrowUp => {
            if modifiers.command {
                // mac and windows behavior
                *cursor = Cursor::default();
            } else {
                *cursor = galley.cursor_up_one_row(cursor);
            }
            None
        }
        Key::ArrowDown => {
            if modifiers.command {
                // mac and windows behavior
                *cursor = galley.end();
            } else {
                *cursor = galley.cursor_down_one_row(cursor);
            }
            None
        }

        Key::Home => {
            if modifiers.ctrl {
                // windows behavior
                *cursor = Cursor::default();
            } else {
                *cursor = galley.cursor_begin_of_row(cursor);
            }
            None
        }
        Key::End => {
            if modifiers.ctrl {
                // windows behavior
                *cursor = galley.end();
            } else {
                *cursor = galley.cursor_end_of_row(cursor);
            }
            None
        }

        Key::Enter | Key::Escape => unreachable!("Handled outside this function"),

        Key::Insert | Key::PageDown | Key::PageUp | Key::Tab => None,
    }
}
