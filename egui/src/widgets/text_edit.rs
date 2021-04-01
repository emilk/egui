use crate::{util::undoer::Undoer, *};
use epaint::{text::cursor::*, *};

#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "persistence", serde(default))]
pub(crate) struct State {
    cursorp: Option<CursorPair>,

    #[cfg_attr(feature = "persistence", serde(skip))]
    undoer: Undoer<(CCursorPair, String)>,
}

#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
pub struct CursorPair {
    /// When selecting with a mouse, this is where the mouse was released.
    /// When moving with e.g. shift+arrows, this is what moves.
    /// Note that the two ends can come in any order, and also be equal (no selection).
    pub primary: Cursor,

    /// When selecting with a mouse, this is where the mouse was first pressed.
    /// This part of the cursor does not move when shift is down.
    pub secondary: Cursor,
}

impl CursorPair {
    fn one(cursor: Cursor) -> Self {
        Self {
            primary: cursor,
            secondary: cursor,
        }
    }

    fn two(min: Cursor, max: Cursor) -> Self {
        Self {
            primary: max,
            secondary: min,
        }
    }

    fn as_ccursorp(&self) -> CCursorPair {
        CCursorPair {
            primary: self.primary.ccursor,
            secondary: self.secondary.ccursor,
        }
    }

    fn is_empty(&self) -> bool {
        self.primary.ccursor == self.secondary.ccursor
    }

    /// If there is a selection, None is returned.
    /// If the two ends is the same, that is returned.
    fn single(&self) -> Option<Cursor> {
        if self.is_empty() {
            Some(self.primary)
        } else {
            None
        }
    }

    fn primary_is_first(&self) -> bool {
        let p = self.primary.ccursor;
        let s = self.secondary.ccursor;
        (p.index, p.prefer_next_row) <= (s.index, s.prefer_next_row)
    }

    fn sorted(&self) -> [Cursor; 2] {
        if self.primary_is_first() {
            [self.primary, self.secondary]
        } else {
            [self.secondary, self.primary]
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
struct CCursorPair {
    /// When selecting with a mouse, this is where the mouse was released.
    /// When moving with e.g. shift+arrows, this is what moves.
    /// Note that the two ends can come in any order, and also be equal (no selection).
    pub primary: CCursor,

    /// When selecting with a mouse, this is where the mouse was first pressed.
    /// This part of the cursor does not move when shift is down.
    pub secondary: CCursor,
}

impl CCursorPair {
    fn one(ccursor: CCursor) -> Self {
        Self {
            primary: ccursor,
            secondary: ccursor,
        }
    }

    fn two(min: CCursor, max: CCursor) -> Self {
        Self {
            primary: max,
            secondary: min,
        }
    }
}

/// A text region that the user can edit the contents of.
///
/// Se also [`Ui::text_edit_singleline`] and  [`Ui::text_edit_multiline`].
///
/// Example:
///
/// ```
/// # let mut ui = egui::Ui::__test();
/// # let mut my_string = String::new();
/// let response = ui.add(egui::TextEdit::singleline(&mut my_string));
/// if response.changed() {
///     // …
/// }
/// if response.lost_focus() && ui.input().key_pressed(egui::Key::Enter) {
///     // …
/// }
/// ```
#[must_use = "You should put this widget in an ui with `ui.add(widget);`"]
#[derive(Debug)]
pub struct TextEdit<'t> {
    text: &'t mut String,
    hint_text: String,
    id: Option<Id>,
    id_source: Option<Id>,
    text_style: Option<TextStyle>,
    text_color: Option<Color32>,
    frame: bool,
    multiline: bool,
    enabled: bool,
    desired_width: Option<f32>,
    desired_height_rows: usize,
    tab_as_spaces: bool,
    tab_moves_focus: bool,
}
impl<'t> TextEdit<'t> {
    pub fn cursor(ui: &Ui, id: Id) -> Option<CursorPair> {
        ui.memory()
            .text_edit
            .get(&id)
            .and_then(|state| state.cursorp)
    }
}

impl<'t> TextEdit<'t> {
    #[deprecated = "Use `TextEdit::singleline` or `TextEdit::multiline` (or the helper `ui.text_edit_singleline`, `ui.text_edit_multiline`) instead"]
    pub fn new(text: &'t mut String) -> Self {
        Self::multiline(text)
    }

    /// Now newlines (`\n`) allowed. Pressing enter key will result in the `TextEdit` loosing focus (`response.lost_focus`).
    pub fn singleline(text: &'t mut String) -> Self {
        TextEdit {
            text,
            hint_text: Default::default(),
            id: None,
            id_source: None,
            text_style: None,
            text_color: None,
            frame: true,
            multiline: false,
            enabled: true,
            desired_width: None,
            desired_height_rows: 1,
            tab_as_spaces: false,
            tab_moves_focus: true,
        }
    }

    /// A `TextEdit` for multiple lines. Pressing enter key will create a new line.
    pub fn multiline(text: &'t mut String) -> Self {
        TextEdit {
            text,
            hint_text: Default::default(),
            id: None,
            id_source: None,
            text_style: None,
            frame: true,
            text_color: None,
            multiline: true,
            enabled: true,
            desired_width: None,
            desired_height_rows: 4,
            tab_as_spaces: false,
            tab_moves_focus: true,
        }
    }

    /// Registers if this widget will insert spaces instead of tab char
    ///
    /// ```rust, ignore
    /// ui.add(egui::TextEdit::multiline(&mut self.multiline_text_input)
    ///     .tab_as_spaces(true));
    /// ```
    pub fn tab_as_spaces(mut self, b: bool) -> Self {
        self.tab_as_spaces = b;
        self
    }

    /// When this is true, then pass focus to the next
    /// widget.
    ///
    /// When this is false, then insert identation based on the value of
    /// `tab_as_spaces` property.
    ///
    /// ```rust, ignore
    /// ui.add(egui::TextEdit::multiline(&mut self.multiline_text_input)
    ///     .tab_moves_focus(true));
    /// ```
    pub fn tab_moves_focus(mut self, b: bool) -> Self {
        self.tab_moves_focus = b;
        self
    }

    /// Build a `TextEdit` focused on code editing.
    /// By default it comes with:
    /// - monospaced font
    /// - focus lock
    /// - tab as spaces
    ///
    /// Shortcut for:
    /// ```rust, ignore
    /// egui::TextEdit::multiline(code_snippet)
    ///     .text_style(TextStyle::Monospace)
    ///     .tab_as_spaces(true)
    ///     .tab_moves_focus(false);
    /// ```
    pub fn code_editor(self) -> Self {
        self.text_style(TextStyle::Monospace)
            .tab_as_spaces(true)
            .tab_moves_focus(false)
    }

    /// Build a `TextEdit` focused on code editing with configurable `Tab` management.
    ///
    /// Shortcut for:
    /// ```rust, ignore
    /// egui::TextEdit::multiline(code_snippet)
    ///     .code_editor()
    ///     .tab_as_spaces(tab_as_spaces)
    ///     .tab_moves_focus(tab_moves_focus);
    /// ```
    pub fn code_editor_with_config(self, tab_as_spaces: bool, tab_moves_focus: bool) -> Self {
        self.code_editor()
            .tab_as_spaces(tab_as_spaces)
            .tab_moves_focus(tab_moves_focus)
    }

    pub fn id(mut self, id: Id) -> Self {
        self.id = Some(id);
        self
    }

    /// A source for the unique `Id`, e.g. `.id_source("second_text_edit_field")` or `.id_source(loop_index)`.
    pub fn id_source(mut self, id_source: impl std::hash::Hash) -> Self {
        self.id_source = Some(Id::new(id_source));
        self
    }

    /// Show a faint hint text when the text field is empty.
    pub fn hint_text(mut self, hint_text: impl Into<String>) -> Self {
        self.hint_text = hint_text.into();
        self
    }

    pub fn text_style(mut self, text_style: TextStyle) -> Self {
        self.text_style = Some(text_style);
        self
    }

    pub fn text_color(mut self, text_color: Color32) -> Self {
        self.text_color = Some(text_color);
        self
    }

    pub fn text_color_opt(mut self, text_color: Option<Color32>) -> Self {
        self.text_color = text_color;
        self
    }

    /// Default is `true`. If set to `false` then you cannot edit the text.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Default is `true`. If set to `false` there will be no frame showing that this is editable text!
    pub fn frame(mut self, frame: bool) -> Self {
        self.frame = frame;
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
    pub fn desired_rows(mut self, desired_height_rows: usize) -> Self {
        self.desired_height_rows = desired_height_rows;
        self
    }
}

impl<'t> Widget for TextEdit<'t> {
    fn ui(self, ui: &mut Ui) -> Response {
        let frame = self.frame;
        let where_to_put_background = ui.painter().add(Shape::Noop);

        let margin = Vec2::new(4.0, 2.0);
        let max_rect = ui.available_rect_before_wrap().shrink2(margin);
        let mut content_ui = ui.child_ui(max_rect, *ui.layout());
        let response = self.content_ui(&mut content_ui);
        let frame_rect = response.rect.expand2(margin);
        let response = response | ui.allocate_rect(frame_rect, Sense::hover());

        if frame {
            let visuals = ui.style().interact(&response);
            let frame_rect = response.rect.expand(visuals.expansion);
            let shape = if response.has_focus() {
                Shape::Rect {
                    rect: frame_rect,
                    corner_radius: visuals.corner_radius,
                    // fill: ui.visuals().selection.bg_fill,
                    fill: ui.visuals().extreme_bg_color,
                    stroke: ui.visuals().selection.stroke,
                }
            } else {
                Shape::Rect {
                    rect: frame_rect,
                    corner_radius: visuals.corner_radius,
                    fill: ui.visuals().extreme_bg_color,
                    stroke: visuals.bg_stroke, // TODO: we want to show something here, or a text-edit field doesn't "pop".
                }
            };

            ui.painter().set(where_to_put_background, shape);
        }

        response
    }
}

impl<'t> TextEdit<'t> {
    fn content_ui(self, ui: &mut Ui) -> Response {
        let TextEdit {
            text,
            hint_text,
            id,
            id_source,
            text_style,
            text_color,
            frame: _,
            multiline,
            enabled,
            desired_width,
            desired_height_rows,
            tab_as_spaces,
            tab_moves_focus,
        } = self;

        let text_style = text_style.unwrap_or_else(|| ui.style().body_text_style);
        let font = &ui.fonts()[text_style];
        let line_spacing = font.row_height();
        let available_width = ui.available_width();
        let mut galley = if multiline {
            font.layout_multiline(text.clone(), available_width)
        } else {
            font.layout_single_line(text.clone())
        };

        let desired_width = desired_width.unwrap_or_else(|| ui.spacing().text_edit_width);
        let desired_height = (desired_height_rows.at_least(1) as f32) * line_spacing;
        let desired_size = vec2(
            galley.size.x.max(desired_width.min(available_width)),
            galley.size.y.max(desired_height),
        );
        let (auto_id, rect) = ui.allocate_space(desired_size);

        let id = id.unwrap_or_else(|| {
            if let Some(id_source) = id_source {
                ui.make_persistent_id(id_source)
            } else {
                auto_id // Since we are only storing the cursor a persistent Id is not super important
            }
        });
        let mut state = ui.memory().text_edit.get(&id).cloned().unwrap_or_default();

        let sense = if enabled {
            Sense::click_and_drag()
        } else {
            Sense::hover()
        };
        let mut response = ui.interact(rect, id, sense);

        if enabled {
            if let Some(pointer_pos) = ui.input().pointer.interact_pos() {
                // TODO: triple-click to select whole paragraph
                // TODO: drag selected text to either move or clone (ctrl on windows, alt on mac)

                let cursor_at_pointer = galley.cursor_from_pos(pointer_pos - response.rect.min);

                if ui.visuals().text_cursor_preview
                    && response.hovered()
                    && ui.input().pointer.is_moving()
                {
                    // preview:
                    paint_cursor_end(ui, response.rect.min, &galley, &cursor_at_pointer);
                }

                if response.double_clicked() {
                    // Select word:
                    let center = cursor_at_pointer;
                    let ccursorp = select_word_at(text, center.ccursor);
                    state.cursorp = Some(CursorPair {
                        primary: galley.from_ccursor(ccursorp.primary),
                        secondary: galley.from_ccursor(ccursorp.secondary),
                    });
                    response.mark_changed();
                } else if response.hovered() && ui.input().pointer.any_pressed() {
                    ui.memory().request_focus(id);
                    if ui.input().modifiers.shift {
                        if let Some(cursorp) = &mut state.cursorp {
                            cursorp.primary = cursor_at_pointer;
                        } else {
                            state.cursorp = Some(CursorPair::one(cursor_at_pointer));
                        }
                    } else {
                        state.cursorp = Some(CursorPair::one(cursor_at_pointer));
                    }
                    response.mark_changed();
                } else if ui.input().pointer.any_down() && response.is_pointer_button_down_on() {
                    if let Some(cursorp) = &mut state.cursorp {
                        cursorp.primary = cursor_at_pointer;
                        response.mark_changed();
                    }
                }
            }
        }

        if response.hovered() && enabled {
            ui.output().cursor_icon = CursorIcon::Text;
        }

        if ui.memory().has_focus(id) && enabled {
            ui.memory().lock_focus(id, !tab_moves_focus);

            let mut cursorp = state
                .cursorp
                .map(|cursorp| {
                    // We only keep the PCursor (paragraph number, and character offset within that paragraph).
                    // This is so what if we resize the `TextEdit` region, and text wrapping changes,
                    // we keep the same byte character offset from the beginning of the text,
                    // even though the number of rows changes
                    // (each paragraph can be several rows, due to word wrapping).
                    // The column (character offset) should be able to extend beyond the last word so that we can
                    // go down and still end up on the same column when we return.
                    CursorPair {
                        primary: galley.from_pcursor(cursorp.primary.pcursor),
                        secondary: galley.from_pcursor(cursorp.secondary.pcursor),
                    }
                })
                .unwrap_or_else(|| CursorPair::one(galley.end()));

            // We feed state to the undoer both before and after handling input
            // so that the undoer creates automatic saves even when there are no events for a while.
            state
                .undoer
                .feed_state(ui.input().time, &(cursorp.as_ccursorp(), text.clone()));

            for event in &ui.input().events {
                let did_mutate_text = match event {
                    Event::Copy => {
                        if cursorp.is_empty() {
                            ui.ctx().output().copied_text = text.clone();
                        } else {
                            ui.ctx().output().copied_text = selected_str(text, &cursorp).to_owned();
                        }
                        None
                    }
                    Event::Cut => {
                        if cursorp.is_empty() {
                            ui.ctx().output().copied_text = std::mem::take(text);
                            Some(CCursorPair::default())
                        } else {
                            ui.ctx().output().copied_text = selected_str(text, &cursorp).to_owned();
                            Some(CCursorPair::one(delete_selected(text, &cursorp)))
                        }
                    }
                    Event::Text(text_to_insert) => {
                        // Newlines are handled by `Key::Enter`.
                        if !text_to_insert.is_empty()
                            && text_to_insert != "\n"
                            && text_to_insert != "\r"
                        {
                            let mut ccursor = delete_selected(text, &cursorp);

                            insert_text(&mut ccursor, text, text_to_insert);
                            Some(CCursorPair::one(ccursor))
                        } else {
                            None
                        }
                    }
                    Event::Key {
                        key: Key::LeftBracket,
                        pressed: true,
                        modifiers,
                    } if modifiers.command && !modifiers.shift && !modifiers.alt  => {
                        let cursorp = handle_remove_paragraphs_identation(
                            tab_as_spaces,
                            text::MAX_TAB_SIZE,
                            &cursorp,
                            text,
                        );

                        Some(cursorp)
                    }
                    Event::Key {
                        key: Key::RightBracket,
                        pressed: true,
                        modifiers,
                    } if modifiers.command && !modifiers.shift && !modifiers.alt  => {
                        let cursorp = handle_insert_paragraphs_identation(
                            tab_as_spaces,
                            text::MAX_TAB_SIZE,
                            &cursorp,
                            text,
                        );

                        Some(cursorp)
                    }
                    Event::Key {
                        key: Key::Tab,
                        pressed: true,
                        modifiers,
                    } => {
                        let max_tab_size = text::MAX_TAB_SIZE;

                        if multiline && ui.memory().has_lock_focus(id) {
                            let [min, max] = cursorp.sorted();

                            let is_insert_identation = modifiers.is_none()
                                && min.pcursor.paragraph == max.pcursor.paragraph;

                            let is_remove_identation = modifiers.shift && !modifiers.command;

                            let is_add_identation = modifiers.is_none() && min.pcursor.paragraph != max.pcursor.paragraph;

                            if is_insert_identation {
                                let ccursor = handle_identation_insert(
                                    tab_as_spaces,
                                    max_tab_size,
                                    &cursorp,
                                    text,
                                );
                                Some(CCursorPair::one(ccursor))
                            } else if is_remove_identation {
                                let cursorp = handle_remove_paragraphs_identation(
                                    tab_as_spaces,
                                    max_tab_size,
                                    &cursorp,
                                    text,
                                );

                                Some(cursorp)
                            } else if is_add_identation {
                                let cursorp = handle_insert_paragraphs_identation(
                                    tab_as_spaces,
                                    max_tab_size,
                                    &cursorp,
                                    text,
                                );

                                Some(cursorp)
                            } else {
                                None
                            }
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
                            let mut ccursor = delete_selected(text, &cursorp);
                            insert_text(&mut ccursor, text, "\n");
                            Some(CCursorPair::one(ccursor))
                        } else {
                            ui.memory().surrender_focus(id); // End input with enter
                            break;
                        }
                    }
                    Event::Key {
                        key: Key::Z,
                        pressed: true,
                        modifiers,
                    } if modifiers.command && !modifiers.shift => {
                        // TODO: redo
                        if let Some((undo_ccursorp, undo_txt)) =
                            state.undoer.undo(&(cursorp.as_ccursorp(), text.clone()))
                        {
                            *text = undo_txt.clone();
                            Some(*undo_ccursorp)
                        } else {
                            None
                        }
                    }

                    Event::Key {
                        key,
                        pressed: true,
                        modifiers,
                    } => on_key_press(&mut cursorp, text, &galley, *key, modifiers),

                    _ => None,
                };

                if let Some(new_ccursorp) = did_mutate_text {
                    response.mark_changed();

                    // Layout again to avoid frame delay, and to keep `text` and `galley` in sync.
                    let font = &ui.fonts()[text_style];
                    galley = if multiline {
                        font.layout_multiline(text.clone(), available_width)
                    } else {
                        font.layout_single_line(text.clone())
                    };

                    // Set cursorp using new galley:
                    cursorp = CursorPair {
                        primary: galley.from_ccursor(new_ccursorp.primary),
                        secondary: galley.from_ccursor(new_ccursorp.secondary),
                    };
                }
            }
            state.cursorp = Some(cursorp);

            state
                .undoer
                .feed_state(ui.input().time, &(cursorp.as_ccursorp(), text.clone()));
        }

        if ui.memory().has_focus(id) {
            if let Some(cursorp) = state.cursorp {
                paint_cursor_selection(ui, response.rect.min, &galley, &cursorp);
                paint_cursor_end(ui, response.rect.min, &galley, &cursorp.primary);
            }
        }

        let text_color = text_color
            .or(ui.visuals().override_text_color)
            // .unwrap_or_else(|| ui.style().interact(&response).text_color()); // too bright
            .unwrap_or_else(|| ui.visuals().widgets.inactive.text_color());
        ui.painter()
            .galley(response.rect.min, galley, text_style, text_color);

        if text.is_empty() && !hint_text.is_empty() {
            let font = &ui.fonts()[text_style];
            let galley = if multiline {
                font.layout_multiline(hint_text, available_width)
            } else {
                font.layout_single_line(hint_text)
            };
            let hint_text_color = ui.visuals().weak_text_color();
            ui.painter()
                .galley(response.rect.min, galley, text_style, hint_text_color);
        }

        ui.memory().text_edit.insert(id, state);

        response.widget_info(|| WidgetInfo::text_edit(&*text));
        response
    }
}

// ----------------------------------------------------------------------------

fn paint_cursor_selection(ui: &mut Ui, pos: Pos2, galley: &Galley, cursorp: &CursorPair) {
    let color = ui.visuals().selection.bg_fill;
    if cursorp.is_empty() {
        return;
    }
    let [min, max] = cursorp.sorted();
    let min = min.rcursor;
    let max = max.rcursor;

    for ri in min.row..=max.row {
        let row = &galley.rows[ri];
        let left = if ri == min.row {
            row.x_offset(min.column)
        } else {
            row.min_x()
        };
        let right = if ri == max.row {
            row.x_offset(max.column)
        } else {
            let newline_size = if row.ends_with_newline {
                row.height() / 2.0 // visualize that we select the newline
            } else {
                0.0
            };
            row.max_x() + newline_size
        };
        let rect = Rect::from_min_max(pos + vec2(left, row.y_min), pos + vec2(right, row.y_max));
        ui.painter().rect_filled(rect, 0.0, color);
    }
}

fn paint_cursor_end(ui: &mut Ui, pos: Pos2, galley: &Galley, cursor: &Cursor) {
    let stroke = ui.visuals().selection.stroke;

    let cursor_pos = galley.pos_from_cursor(cursor).translate(pos.to_vec2());
    let cursor_pos = cursor_pos.expand(1.5); // slightly above/below row

    let top = cursor_pos.center_top();
    let bottom = cursor_pos.center_bottom();

    ui.painter().line_segment(
        [top, bottom],
        (ui.visuals().text_cursor_width, stroke.color),
    );

    if false {
        // Roof/floor:
        let extrusion = 3.0;
        let width = 1.0;
        ui.painter().line_segment(
            [top - vec2(extrusion, 0.0), top + vec2(extrusion, 0.0)],
            (width, stroke.color),
        );
        ui.painter().line_segment(
            [bottom - vec2(extrusion, 0.0), bottom + vec2(extrusion, 0.0)],
            (width, stroke.color),
        );
    }
}

// ----------------------------------------------------------------------------

fn selected_str<'s>(text: &'s str, cursorp: &CursorPair) -> &'s str {
    let [min, max] = cursorp.sorted();
    let byte_begin = byte_index_from_char_index(text, min.ccursor.index);
    let byte_end = byte_index_from_char_index(text, max.ccursor.index);
    &text[byte_begin..byte_end]
}

fn byte_index_from_char_index(s: &str, char_index: usize) -> usize {
    for (ci, (bi, _)) in s.char_indices().enumerate() {
        if ci == char_index {
            return bi;
        }
    }
    s.len()
}

fn insert_text(ccursor: &mut CCursor, text: &mut String, text_to_insert: &str) {
    let mut char_it = text.chars();
    let mut new_text = String::with_capacity(text.len() + text_to_insert.len());
    for _ in 0..ccursor.index {
        let c = char_it.next().unwrap();
        new_text.push(c);
    }
    ccursor.index += text_to_insert.chars().count();
    new_text += text_to_insert;
    new_text.extend(char_it);
    *text = new_text;
}

fn insert_identation(
    tab_as_spaces: bool,
    max_tab_size: usize,
    ccursor: &mut CCursor,
    text: &mut String,
) {
    if tab_as_spaces {
        let line_start = ccursor_paragraph_start(text, *ccursor);

        let virtual_columns_count = virtual_columns_count(max_tab_size, text, line_start, *ccursor);
        let mut spaces_to_insert = max_tab_size;
        spaces_to_insert -= virtual_columns_count % max_tab_size;

        for _ in 0..spaces_to_insert {
            insert_text(ccursor, text, " ");
        }

        return;
    }

    insert_text(ccursor, text, "\t");
}

fn handle_identation_insert(
    tab_as_spaces: bool,
    max_tab_size: usize,
    cursorp: &CursorPair,
    text: &mut String,
) -> CCursor {
    let mut ccursor = delete_selected(text, cursorp);
    insert_identation(tab_as_spaces, max_tab_size, &mut ccursor, text);

    ccursor
}

fn convert_identation_to_spaces(
    max_tab_size: usize,
    identation: &str,
    paragraph_offset: usize,
    cursorp: &mut CCursorPair,
) -> String {
    // Because we convert tabs to spaces we add a bit more capacity
    // Hopefuly it will be enough and no other alloc will be done
    let mut new_identation = String::with_capacity(identation.len() + 5 * max_tab_size);

    let mut char_it = identation.chars().enumerate().peekable();

    let mut tab_size = max_tab_size;
    while let Some((index, c)) = char_it.peek() {
        if *c == ' ' {
            char_it.next();
            new_identation += " ";

            tab_size -= 1;

            if tab_size == 0 {
                tab_size = max_tab_size;
            }
        } else if *c == '\t' {
            if tab_size == 0 {
                tab_size = max_tab_size;
            }

            for _ in 0..tab_size {
                new_identation += " ";
            }

            let insert_offset = paragraph_offset + index;
            let amount = tab_size.saturating_sub(1);
            *cursorp = update_selection_insert(*cursorp, insert_offset, amount);

            char_it.next();
        } else {
            break;
        }
    }

    for (_, c) in char_it {
        new_identation.push(c);
    }

    new_identation
}

fn convert_identation_to_tabs(
    max_tab_size: usize,
    identation: &str,
    paragraph_offset: usize,
    cursorp: &mut CCursorPair,
) -> String {
    let mut new_identation = String::with_capacity(identation.len());

    let mut char_it = identation.chars().enumerate().peekable();
    let mut tab_size = max_tab_size;

    while let Some((index, c)) = char_it.peek() {
        if *c == ' ' {
            tab_size -= 1;

            if tab_size == 0 {
                new_identation.push('\t');

                let delete_offset = paragraph_offset + index.saturating_sub(max_tab_size);
                update_selection_delete(*cursorp, delete_offset, tab_size);

                tab_size = max_tab_size;
            }

            char_it.next();
        } else if *c == '\t' {
            char_it.next();
            new_identation.push('\t');

            if tab_size != max_tab_size {
                tab_size = max_tab_size;
            }
        }
    }

    // We do this because we might have something like this
    // --->__let x = 4;
    //
    // As you can see the second tab chunck contains:
    // __le
    //
    // We have to put those spaces back
    for _ in 0..(max_tab_size - tab_size) {
        new_identation.push(' ');

        let last_index = new_identation.chars().count() - 1;

        if last_index < cursorp.primary.index {
            cursorp.primary += 1;
        }

        if last_index < cursorp.secondary.index {
            cursorp.secondary += 1;
        }
    }

    new_identation
}

fn convert_identation(
    tabs_as_spaces: bool,
    max_tab_size: usize,
    identation: &str,
    paragraph_offset: usize,
    cursorp: &mut CCursorPair,
) -> String {
    if tabs_as_spaces {
        convert_identation_to_spaces(max_tab_size, identation, paragraph_offset, cursorp)
    } else {
        convert_identation_to_tabs(max_tab_size, identation, paragraph_offset, cursorp)
    }
}

fn extract_identation(paragraph: &str) -> String {
    let char_it = paragraph.chars().take_while(|x| *x == ' ' || *x == '\t');

    let mut identation = String::new();
    for c in char_it {
        identation.push(c);
    }

    identation
}

fn extract_content(paragraph: &str) -> String {
    let char_it = paragraph.chars().skip_while(|x| *x == ' ' || *x == '\t');

    let mut content = String::new();
    for c in char_it {
        content.push(c);
    }

    content
}

fn decrease_identation(
    tab_as_spaces: bool,
    max_tab_size: usize,
    identation: &str,
    paragraph_offset: usize,
    cursorp: &mut CCursorPair,
) -> String {
    let converted_identation = convert_identation(
        tab_as_spaces,
        max_tab_size,
        identation,
        paragraph_offset,
        cursorp,
    );
    let mut new_identation = String::with_capacity(converted_identation.len());

    if tab_as_spaces {
        // With spaces we have two cases
        // 1. ____ __let x = 2;   <- Here we remove the spaces
        //                           between the full spaces block
        //                           and `let`.
        //
        // 2. ____ ____let x = 1; <- Here we remove a full spaces block

        let spaces_count = converted_identation.chars().count();
        let spaces_to_remove = max_tab_size - spaces_count % max_tab_size;

        new_identation.extend(
            converted_identation
                .chars()
                .take(spaces_count.saturating_sub(spaces_to_remove)),
        );

        if spaces_count > 0 {
            let delete_offset = paragraph_offset + spaces_count.saturating_sub(1).saturating_sub(spaces_to_remove);
            *cursorp = update_selection_delete(*cursorp, delete_offset, spaces_to_remove);
        }
    } else {
        // With tabs we have two cases
        // 1. --->__let x = 2;   <- Here we remove the spaces
        //                          between the tab and `let`
        //
        // 2. --->--->let x = 1; <- Here we remove a tab

        let mut spaces_removed = 0;
        let mut rev_ident_it = converted_identation.chars().rev().peekable();

        // Case 1: We remove potential spaces
        while let Some(c) = rev_ident_it.peek() {
            if *c == ' ' {
                rev_ident_it.next();
                spaces_removed += 1;
            } else {
                break;
            }
        }

        // Case 2: We remove a tab
        let chars_removed = if spaces_removed == 0 {
            rev_ident_it.next();
            1
        } else {
            spaces_removed
        };

        let delete_offset = paragraph_offset + rev_ident_it.clone().count().saturating_sub(1);
        *cursorp = update_selection_delete(*cursorp, delete_offset, chars_removed);

        new_identation.extend(rev_ident_it);
    }

    new_identation
}

fn increase_identation(
    tab_as_spaces: bool,
    max_tab_size: usize,
    identation: &str,
    paragraph_offset: usize,
    cursorp: &mut CCursorPair,
) -> String {
    let converted_identation = convert_identation(
        tab_as_spaces,
        max_tab_size,
        identation,
        paragraph_offset,
        cursorp,
    );
    let mut new_identation = String::with_capacity(converted_identation.len());

    if tab_as_spaces {
        // With spaces we have two cases
        // 1. ____ __let x = 2;   <- Here we add necesary spaces
        //                           between the full spaces block
        //                           and `let`.
        //
        // 2. ____ ____let x = 1; <- Here we add a full spaces block

        let spaces_count = converted_identation.chars().count();
        let spaces_to_add = max_tab_size - spaces_count % max_tab_size;

        new_identation = converted_identation;

        for _ in 0..spaces_to_add {
            new_identation.push(' ')
        }

        let insert_offset = paragraph_offset + spaces_count + spaces_to_add;
        *cursorp = update_selection_insert(*cursorp, insert_offset, spaces_to_add);
    } else {
        // With tabs we have two cases
        // 1. --->__let x = 2;   <- Here we remove the spaces
        //                          between the tab and `let`
        //                          and then add a tab
        //
        // 2. --->--->let x = 1; <- Here we add a tab

        let mut spaces_removed: usize = 0;
        let mut rev_ident_it = converted_identation.chars().rev().peekable();

        // Case 1: We remove potential spaces
        while let Some(c) = rev_ident_it.peek() {
            if *c == ' ' {
                rev_ident_it.next();
                spaces_removed += 1;
            } else {
                break;
            }
        }

        // Case 2: We add a tab
        new_identation.push('\t');
        let chars_added = if spaces_removed == 0 {
            1
        } else {
            spaces_removed.saturating_sub(1)
        };

        let insert_offset = paragraph_offset + rev_ident_it.clone().count().saturating_sub(1);
        *cursorp = update_selection_insert(*cursorp, insert_offset, chars_added);

        new_identation.extend(rev_ident_it);
    }

    new_identation
}

fn handle_remove_paragraphs_identation(
    tab_as_spaces: bool,
    max_tab_size: usize,
    cursorp: &CursorPair,
    text: &mut String,
) -> CCursorPair {
    let [min, max] = cursorp.sorted();
    let mut cursorp = CCursorPair::two(min.ccursor, max.ccursor);

    let mut paragraph_offset = 0;
    let new_text = text
        .lines()
        .enumerate()
        .map(|(index, paragraph)| {
            let is_paragraph_to_ident =
                index >= min.pcursor.paragraph && index <= max.pcursor.paragraph;

            if is_paragraph_to_ident {
                let identation = extract_identation(paragraph);
                let content = extract_content(paragraph);

                let mut new_paragraph = decrease_identation(
                    tab_as_spaces,
                    max_tab_size,
                    &identation,
                    paragraph_offset,
                    &mut cursorp,
                );
                new_paragraph += &content;

                paragraph_offset += new_paragraph.chars().count();
                new_paragraph
            } else {
                paragraph_offset += paragraph.chars().count();
                paragraph.to_string()
            }
        })
        .collect::<Vec<String>>()
        .join("\n");

    *text = new_text;

    cursorp
}

fn handle_insert_paragraphs_identation(
    tab_as_spaces: bool,
    max_tab_size: usize,
    cursorp: &CursorPair,
    text: &mut String,
) -> CCursorPair {
    let [min, max] = cursorp.sorted();
    let mut cursorp = CCursorPair::two(min.ccursor, max.ccursor);

    let mut paragraph_offset = 0;
    let new_text = text
        .lines()
        .enumerate()
        .map(|(index, paragraph)| {
            let is_paragraph_to_ident =
                index >= min.pcursor.paragraph && index <= max.pcursor.paragraph;

            if is_paragraph_to_ident {
                let identation = extract_identation(paragraph);
                let content = extract_content(paragraph);

                let mut new_paragraph = increase_identation(
                    tab_as_spaces,
                    max_tab_size,
                    &identation,
                    paragraph_offset,
                    &mut cursorp,
                );
                new_paragraph += &content;

                paragraph_offset += new_paragraph.chars().count();
                new_paragraph
            } else {
                paragraph_offset += paragraph.chars().count().saturating_sub(1);
                paragraph.to_string()
            }
        })
        .collect::<Vec<String>>()
        .join("\n");

    *text = new_text;

    cursorp
}

// ----------------------------------------------------------------------------

fn delete_selected(text: &mut String, cursorp: &CursorPair) -> CCursor {
    let [min, max] = cursorp.sorted();
    delete_selected_ccursor_range(text, [min.ccursor, max.ccursor])
}

fn delete_selected_ccursor_range(text: &mut String, [min, max]: [CCursor; 2]) -> CCursor {
    let [min, max] = [min.index, max.index];
    assert!(min <= max);
    if min < max {
        let mut char_it = text.chars();
        let mut new_text = String::with_capacity(text.len());
        for _ in 0..min {
            new_text.push(char_it.next().unwrap())
        }
        new_text.extend(char_it.skip(max - min));
        *text = new_text;
    }
    CCursor {
        index: min,
        prefer_next_row: true,
    }
}

fn delete_previous_char(text: &mut String, ccursor: CCursor) -> CCursor {
    if ccursor.index > 0 {
        let max_ccursor = ccursor;
        let min_ccursor = max_ccursor - 1;
        delete_selected_ccursor_range(text, [min_ccursor, max_ccursor])
    } else {
        ccursor
    }
}

fn delete_next_char(text: &mut String, ccursor: CCursor) -> CCursor {
    delete_selected_ccursor_range(text, [ccursor, ccursor + 1])
}

fn delete_previous_word(text: &mut String, max_ccursor: CCursor) -> CCursor {
    let min_ccursor = ccursor_previous_word(text, max_ccursor);
    delete_selected_ccursor_range(text, [min_ccursor, max_ccursor])
}

fn delete_next_word(text: &mut String, min_ccursor: CCursor) -> CCursor {
    let max_ccursor = ccursor_next_word(text, min_ccursor);
    delete_selected_ccursor_range(text, [min_ccursor, max_ccursor])
}

fn delete_paragraph_before_cursor(
    text: &mut String,
    galley: &Galley,
    cursorp: &CursorPair,
) -> CCursor {
    let [min, max] = cursorp.sorted();
    let min = galley.from_pcursor(PCursor {
        paragraph: min.pcursor.paragraph,
        offset: 0,
        prefer_next_row: true,
    });
    if min.ccursor == max.ccursor {
        delete_previous_char(text, min.ccursor)
    } else {
        delete_selected(text, &CursorPair::two(min, max))
    }
}

fn delete_paragraph_after_cursor(
    text: &mut String,
    galley: &Galley,
    cursorp: &CursorPair,
) -> CCursor {
    let [min, max] = cursorp.sorted();
    let max = galley.from_pcursor(PCursor {
        paragraph: max.pcursor.paragraph,
        offset: usize::MAX, // end of paragraph
        prefer_next_row: false,
    });
    if min.ccursor == max.ccursor {
        delete_next_char(text, min.ccursor)
    } else {
        delete_selected(text, &CursorPair::two(min, max))
    }
}

// ----------------------------------------------------------------------------

/// Returns `Some(new_cursor)` if we did mutate `text`.
fn on_key_press(
    cursorp: &mut CursorPair,
    text: &mut String,
    galley: &Galley,
    key: Key,
    modifiers: &Modifiers,
) -> Option<CCursorPair> {
    match key {
        Key::Backspace => {
            let ccursor = if modifiers.mac_cmd {
                delete_paragraph_before_cursor(text, galley, cursorp)
            } else if let Some(cursor) = cursorp.single() {
                if modifiers.alt || modifiers.ctrl {
                    // alt on mac, ctrl on windows
                    delete_previous_word(text, cursor.ccursor)
                } else {
                    delete_previous_char(text, cursor.ccursor)
                }
            } else {
                delete_selected(text, cursorp)
            };
            Some(CCursorPair::one(ccursor))
        }
        Key::Delete => {
            let ccursor = if modifiers.mac_cmd {
                delete_paragraph_after_cursor(text, galley, cursorp)
            } else if let Some(cursor) = cursorp.single() {
                if modifiers.alt || modifiers.ctrl {
                    // alt on mac, ctrl on windows
                    delete_next_word(text, cursor.ccursor)
                } else {
                    delete_next_char(text, cursor.ccursor)
                }
            } else {
                delete_selected(text, cursorp)
            };
            let ccursor = CCursor {
                prefer_next_row: true,
                ..ccursor
            };
            Some(CCursorPair::one(ccursor))
        }

        Key::A if modifiers.command => {
            // select all
            *cursorp = CursorPair::two(Cursor::default(), galley.end());
            None
        }

        Key::K if modifiers.ctrl => {
            let ccursor = delete_paragraph_after_cursor(text, galley, cursorp);
            Some(CCursorPair::one(ccursor))
        }

        Key::U if modifiers.ctrl => {
            let ccursor = delete_paragraph_before_cursor(text, galley, cursorp);
            Some(CCursorPair::one(ccursor))
        }

        Key::W if modifiers.ctrl => {
            let ccursor = if let Some(cursor) = cursorp.single() {
                delete_previous_word(text, cursor.ccursor)
            } else {
                delete_selected(text, cursorp)
            };
            Some(CCursorPair::one(ccursor))
        }

        Key::ArrowLeft | Key::ArrowRight | Key::ArrowUp | Key::ArrowDown | Key::Home | Key::End => {
            move_single_cursor(&mut cursorp.primary, galley, key, modifiers);
            if !modifiers.shift {
                cursorp.secondary = cursorp.primary;
            }
            None
        }

        _ => None,
    }
}

fn move_single_cursor(cursor: &mut Cursor, galley: &Galley, key: Key, modifiers: &Modifiers) {
    match key {
        Key::ArrowLeft => {
            if modifiers.alt || modifiers.ctrl {
                // alt on mac, ctrl on windows
                *cursor = galley.from_ccursor(ccursor_previous_word(&galley.text, cursor.ccursor));
            } else if modifiers.mac_cmd {
                *cursor = galley.cursor_begin_of_row(cursor);
            } else {
                *cursor = galley.cursor_left_one_character(cursor);
            }
        }
        Key::ArrowRight => {
            if modifiers.alt || modifiers.ctrl {
                // alt on mac, ctrl on windows
                *cursor = galley.from_ccursor(ccursor_next_word(&galley.text, cursor.ccursor));
            } else if modifiers.mac_cmd {
                *cursor = galley.cursor_end_of_row(cursor);
            } else {
                *cursor = galley.cursor_right_one_character(cursor);
            }
        }
        Key::ArrowUp => {
            if modifiers.command {
                // mac and windows behavior
                *cursor = Cursor::default();
            } else {
                *cursor = galley.cursor_up_one_row(cursor);
            }
        }
        Key::ArrowDown => {
            if modifiers.command {
                // mac and windows behavior
                *cursor = galley.end();
            } else {
                *cursor = galley.cursor_down_one_row(cursor);
            }
        }

        Key::Home => {
            if modifiers.ctrl {
                // windows behavior
                *cursor = Cursor::default();
            } else {
                *cursor = galley.cursor_begin_of_row(cursor);
            }
        }
        Key::End => {
            if modifiers.ctrl {
                // windows behavior
                *cursor = galley.end();
            } else {
                *cursor = galley.cursor_end_of_row(cursor);
            }
        }

        _ => unreachable!(),
    }
}

// ----------------------------------------------------------------------------

fn select_word_at(text: &str, ccursor: CCursor) -> CCursorPair {
    if ccursor.index == 0 {
        CCursorPair::two(ccursor, ccursor_next_word(text, ccursor))
    } else {
        let it = text.chars();
        let mut it = it.skip(ccursor.index - 1);
        if let Some(char_before_cursor) = it.next() {
            if let Some(char_after_cursor) = it.next() {
                if is_word_char(char_before_cursor) && is_word_char(char_after_cursor) {
                    let min = ccursor_previous_word(text, ccursor + 1);
                    let max = ccursor_next_word(text, min);
                    CCursorPair::two(min, max)
                } else if is_word_char(char_before_cursor) {
                    let min = ccursor_previous_word(text, ccursor);
                    let max = ccursor_next_word(text, min);
                    CCursorPair::two(min, max)
                } else if is_word_char(char_after_cursor) {
                    let max = ccursor_next_word(text, ccursor);
                    CCursorPair::two(ccursor, max)
                } else {
                    let min = ccursor_previous_word(text, ccursor);
                    let max = ccursor_next_word(text, ccursor);
                    CCursorPair::two(min, max)
                }
            } else {
                let min = ccursor_previous_word(text, ccursor);
                CCursorPair::two(min, ccursor)
            }
        } else {
            let max = ccursor_next_word(text, ccursor);
            CCursorPair::two(ccursor, max)
        }
    }
}

fn ccursor_paragraph_start(text: &str, ccursor: CCursor) -> CCursor {
    let num_chars = text.chars().count();
    let paragraph_start = text
        .chars()
        .rev()
        .skip(num_chars - ccursor.index)
        .position(|x| x == '\n');

    let paragraph_start = match paragraph_start {
        Some(ps) => ccursor.index - ps,
        None => 0,
    };

    CCursor::new(paragraph_start)
}

#[allow(dead_code)]
fn ccursor_paragraph_end(text: &str, ccursor: CCursor) -> CCursor {
    let paragraph_end = text.chars().skip(ccursor.index).position(|x| x == '\n');

    let paragraph_end = match paragraph_end {
        Some(pe) => pe,
        None => text.chars().skip(ccursor.index).count(),
    };

    CCursor {
        index: ccursor.index + paragraph_end,
        prefer_next_row: false,
    }
}

fn ccursor_next_word(text: &str, ccursor: CCursor) -> CCursor {
    CCursor {
        index: next_word_boundary_char_index(text.chars(), ccursor.index),
        prefer_next_row: false,
    }
}

fn ccursor_previous_word(text: &str, ccursor: CCursor) -> CCursor {
    let num_chars = text.chars().count();
    CCursor {
        index: num_chars
            - next_word_boundary_char_index(text.chars().rev(), num_chars - ccursor.index),
        prefer_next_row: true,
    }
}

fn next_word_boundary_char_index(it: impl Iterator<Item = char>, mut index: usize) -> usize {
    let mut it = it.skip(index);
    if let Some(_first) = it.next() {
        index += 1;

        if let Some(second) = it.next() {
            index += 1;
            for next in it {
                if is_word_char(next) != is_word_char(second) {
                    break;
                }
                index += 1;
            }
        }
    }
    index
}

fn is_word_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Virtal columns keep track of tabs and how many columns they are using
fn virtual_columns_count(
    tab_size: usize,
    text: &str,
    line_start: CCursor,
    current_position: CCursor,
) -> usize {
    text.chars()
        .enumerate()
        .skip(line_start.index)
        .take_while(|(idx, _)| *idx < current_position.index)
        .fold(0, |columns, (idx, x)| {
            if x == '\t' {
                columns + (tab_size - idx % tab_size)
            } else {
                columns + 1
            }
        })
}

fn update_selection_insert(ccursorp: CCursorPair, insert_offset: usize, amount: usize) -> CCursorPair {
    let (mut min, mut max) = if ccursorp.primary.index < ccursorp.secondary.index {
        (ccursorp.primary.index, ccursorp.secondary.index)
    } else {
        (ccursorp.secondary.index, ccursorp.primary.index)
    };

    if insert_offset < min {
        min = min.saturating_add(amount);
    }

    if insert_offset <= max {
        max = max.saturating_add(amount);
    }

    let min_ccursor = CCursor {
        index: min,
        prefer_next_row: false,
    };

    let max_ccursor = CCursor {
        index: max,
        prefer_next_row: false,
    };

    if ccursorp.primary.index < ccursorp.secondary.index {
        CCursorPair::two(max_ccursor, min_ccursor)
    } else {
        CCursorPair::two(min_ccursor, max_ccursor)
    }
}

fn update_selection_delete(ccursorp: CCursorPair, delete_offset: usize, amount: usize) -> CCursorPair {
    let (mut min, mut max) = if ccursorp.primary.index < ccursorp.secondary.index {
        (ccursorp.primary.index, ccursorp.secondary.index)
    } else {
        (ccursorp.secondary.index, ccursorp.primary.index)
    };

    // () = chunk removed
    // [] = selection

    // --(---)----[-----]----->
    let is_before_selection = delete_offset < min
        && delete_offset < max
        && delete_offset + amount < min
        && delete_offset + amount < max;

    // --(-------[---)--]----->
    let is_before_selection_overlapping_min = delete_offset < min
        && delete_offset < max
        && delete_offset + amount >= min
        && delete_offset + amount < max;

    // ----[--(----)--]------>
    let is_inside_selection_not_overlapping = delete_offset >= min
        && delete_offset < max
        && delete_offset + amount < max;

    // ----[--(-------]--)--->
    let is_inside_selection_overlapping_max = delete_offset >= min
        && delete_offset < max
        && delete_offset + amount >= max;

    // ----(--[-------]--)--->
    let is_before_selection_overlapping_min_and_max = delete_offset < min
        && delete_offset < max
        && delete_offset + amount >= max;

    if is_before_selection {
        min = min.saturating_sub(amount);
        max = max.saturating_sub(amount);
    } else if is_before_selection_overlapping_min {
        min = delete_offset;
        max = max.saturating_sub(amount);
    } else if is_inside_selection_not_overlapping {
        max = max.saturating_sub(amount);
    } else if is_inside_selection_overlapping_max {
        max = delete_offset;
    } else if is_before_selection_overlapping_min_and_max {
        min = delete_offset;
        max = delete_offset;
    }

    let min_ccursor = CCursor {
        index: min,
        prefer_next_row: false,
    };

    let max_ccursor = CCursor {
        index: max,
        prefer_next_row: false,
    };

    if ccursorp.primary.index < ccursorp.secondary.index {
        CCursorPair::two(max_ccursor, min_ccursor)
    } else {
        CCursorPair::two(min_ccursor, max_ccursor)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_ccursor_paragraph_start() {
        let test = ccursor_paragraph_start("", CCursor::new(0));
        assert_eq!(0, test.index);

        let test = ccursor_paragraph_start("\n", CCursor::new(1));
        assert_eq!(1, test.index);

        let test = ccursor_paragraph_start("ASDF", CCursor::new(2));
        assert_eq!(0, test.index);

        let test = ccursor_paragraph_start("\nASDF", CCursor::new(3));
        assert_eq!(1, test.index);

        let test = ccursor_paragraph_start("\n\n\n", CCursor::new(2));
        assert_eq!(2, test.index);
    }

    #[test]
    fn test_ccursor_paragraph_end() {
        let test = ccursor_paragraph_end("", CCursor::new(0));
        assert_eq!(0, test.index);

        let test = ccursor_paragraph_end("ASDF", CCursor::new(2));
        assert_eq!(4, test.index);

        let test = ccursor_paragraph_end("ASDF\naaa", CCursor::new(4));
        assert_eq!(4, test.index);

        let test = ccursor_paragraph_end("\nASDF", CCursor::new(3));
        assert_eq!(5, test.index);

        let test = ccursor_paragraph_end("\n\n\n", CCursor::new(2));
        assert_eq!(2, test.index);
    }

    #[test]
    fn test_identation_virtual_columns_count() {
        let test = virtual_columns_count(text::MAX_TAB_SIZE, "", CCursor::new(0), CCursor::new(0));
        assert_eq!(0, test);

        let test =
            virtual_columns_count(text::MAX_TAB_SIZE, "   ", CCursor::new(0), CCursor::new(3));
        assert_eq!(3, test);

        let test =
            virtual_columns_count(text::MAX_TAB_SIZE, "\t", CCursor::new(0), CCursor::new(1));
        assert_eq!(text::MAX_TAB_SIZE, test);

        let test =
            virtual_columns_count(text::MAX_TAB_SIZE, " \t", CCursor::new(0), CCursor::new(2));
        assert_eq!(text::MAX_TAB_SIZE, test);

        let test =
            virtual_columns_count(text::MAX_TAB_SIZE, "  \t", CCursor::new(0), CCursor::new(3));
        assert_eq!(text::MAX_TAB_SIZE, test);

        let test = virtual_columns_count(
            text::MAX_TAB_SIZE,
            "   \t",
            CCursor::new(0),
            CCursor::new(4),
        );
        assert_eq!(text::MAX_TAB_SIZE, test);
    }

    #[test]
    fn test_decrease_identation() {
        let tab_as_spaces = true;
        let mut cursorp = CCursorPair::one(CCursor::new(0));
        let new_ident = decrease_identation(tab_as_spaces, text::MAX_TAB_SIZE, "", 0, &mut cursorp);
        assert_eq!("", new_ident);

        let mut cursorp = CCursorPair::one(CCursor::new(0));
        let new_ident =
            decrease_identation(tab_as_spaces, text::MAX_TAB_SIZE, "  ", 0, &mut cursorp);
        assert_eq!("", new_ident);

        let mut cursorp = CCursorPair::one(CCursor::new(0));
        let new_ident =
            decrease_identation(tab_as_spaces, text::MAX_TAB_SIZE, "\t", 0, &mut cursorp);
        assert_eq!("", new_ident);

        let mut cursorp = CCursorPair::one(CCursor::new(0));
        let new_ident =
            decrease_identation(tab_as_spaces, text::MAX_TAB_SIZE, "    \t", 0, &mut cursorp);
        assert_eq!("    ", new_ident);

        let mut cursorp = CCursorPair::one(CCursor::new(0));
        let new_ident =
            decrease_identation(tab_as_spaces, text::MAX_TAB_SIZE, "\t    ", 0, &mut cursorp);
        assert_eq!("    ", new_ident);

        let mut cursorp = CCursorPair::one(CCursor::new(2));
        let new_ident =
            decrease_identation(tab_as_spaces, text::MAX_TAB_SIZE, "\t\t", 0, &mut cursorp);
        assert_eq!(4, cursorp.primary.index);
        assert_eq!(4, cursorp.secondary.index);
        assert_eq!("    ", new_ident);

        let tab_as_spaces = false;
        let mut cursorp = CCursorPair::one(CCursor::new(0));
        let new_ident = decrease_identation(tab_as_spaces, text::MAX_TAB_SIZE, "", 0, &mut cursorp);
        assert_eq!("", new_ident);

        let mut cursorp = CCursorPair::one(CCursor::new(0));
        let new_ident =
            decrease_identation(tab_as_spaces, text::MAX_TAB_SIZE, "  ", 0, &mut cursorp);
        assert_eq!("", new_ident);

        let mut cursorp = CCursorPair::one(CCursor::new(0));
        let new_ident =
            decrease_identation(tab_as_spaces, text::MAX_TAB_SIZE, "\t", 0, &mut cursorp);
        assert_eq!("", new_ident);

        let mut cursorp = CCursorPair::one(CCursor::new(0));
        let new_ident =
            decrease_identation(tab_as_spaces, text::MAX_TAB_SIZE, "    \t", 0, &mut cursorp);
        assert_eq!("\t", new_ident);

        let mut cursorp = CCursorPair::one(CCursor::new(0));
        let new_ident =
            decrease_identation(tab_as_spaces, text::MAX_TAB_SIZE, "\t    ", 0, &mut cursorp);
        assert_eq!("\t", new_ident);

        let mut cursorp = CCursorPair::one(CCursor::new(2));
        let new_ident =
            decrease_identation(tab_as_spaces, text::MAX_TAB_SIZE, "\t\t", 0, &mut cursorp);
        assert_eq!(1, cursorp.primary.index);
        assert_eq!(1, cursorp.secondary.index);
        assert_eq!("\t", new_ident);
    }

    #[test]
    fn test_increase_identation() {
        let tab_as_spaces = true;
        let mut cursorp = CCursorPair::one(CCursor::new(0));
        let new_ident = increase_identation(tab_as_spaces, text::MAX_TAB_SIZE, "", 0, &mut cursorp);
        assert_eq!("    ", new_ident);

        let mut cursorp = CCursorPair::one(CCursor::new(0));
        let new_ident =
            increase_identation(tab_as_spaces, text::MAX_TAB_SIZE, "  ", 0, &mut cursorp);
        assert_eq!("    ", new_ident);

        let mut cursorp = CCursorPair::one(CCursor::new(0));
        let new_ident =
            increase_identation(tab_as_spaces, text::MAX_TAB_SIZE, "\t", 0, &mut cursorp);
        assert_eq!("        ", new_ident);

        let mut cursorp = CCursorPair::one(CCursor::new(0));
        let new_ident =
            increase_identation(tab_as_spaces, text::MAX_TAB_SIZE, "    \t", 0, &mut cursorp);
        assert_eq!("            ", new_ident);

        let mut cursorp = CCursorPair::one(CCursor::new(0));
        let new_ident =
            increase_identation(tab_as_spaces, text::MAX_TAB_SIZE, "\t    ", 0, &mut cursorp);
        assert_eq!("            ", new_ident);

        let mut cursorp = CCursorPair::one(CCursor::new(2));
        let new_ident =
            increase_identation(tab_as_spaces, text::MAX_TAB_SIZE, "\t\t", 0, &mut cursorp);
        assert_eq!(8, cursorp.primary.index);
        assert_eq!(8, cursorp.secondary.index);
        assert_eq!("            ", new_ident);

        let tab_as_spaces = false;
        let mut cursorp = CCursorPair::one(CCursor::new(0));
        let new_ident = increase_identation(tab_as_spaces, text::MAX_TAB_SIZE, "", 0, &mut cursorp);
        assert_eq!("\t", new_ident);

        let mut cursorp = CCursorPair::one(CCursor::new(0));
        let new_ident =
            increase_identation(tab_as_spaces, text::MAX_TAB_SIZE, "  ", 0, &mut cursorp);
        assert_eq!("\t", new_ident);

        let mut cursorp = CCursorPair::one(CCursor::new(0));
        let new_ident =
            increase_identation(tab_as_spaces, text::MAX_TAB_SIZE, "\t", 0, &mut cursorp);
        assert_eq!("\t\t", new_ident);

        let mut cursorp = CCursorPair::one(CCursor::new(0));
        let new_ident =
            increase_identation(tab_as_spaces, text::MAX_TAB_SIZE, "    \t", 0, &mut cursorp);
        assert_eq!("\t\t\t", new_ident);

        let mut cursorp = CCursorPair::one(CCursor::new(0));
        let new_ident =
            increase_identation(tab_as_spaces, text::MAX_TAB_SIZE, "\t    ", 0, &mut cursorp);
        assert_eq!("\t\t\t", new_ident);

        let mut cursorp = CCursorPair::one(CCursor::new(2));
        let new_ident =
            increase_identation(tab_as_spaces, text::MAX_TAB_SIZE, "\t\t", 0, &mut cursorp);
        assert_eq!(3, cursorp.primary.index);
        assert_eq!(3, cursorp.secondary.index);
        assert_eq!("\t\t\t", new_ident);
    }

    #[test]
    fn test_update_selection_insert_before_selection() {
        let test = update_selection_insert(CCursorPair::two(CCursor::new(5), CCursor::new(10)), 1, 5);
        assert_eq!(10, test.secondary.index);
        assert_eq!(15, test.primary.index);
        assert_eq!(false, test.primary.prefer_next_row);
        assert_eq!(false, test.secondary.prefer_next_row);

        let test = update_selection_insert(CCursorPair::two(CCursor::new(10), CCursor::new(5)), 1, 5);
        assert_eq!(10, test.primary.index);
        assert_eq!(15, test.secondary.index);
        assert_eq!(false, test.primary.prefer_next_row);
        assert_eq!(false, test.secondary.prefer_next_row);
    }

    #[test]
    fn test_update_selection_insert_inside_selection() {
        let test = update_selection_insert(CCursorPair::two(CCursor::new(5), CCursor::new(10)), 6, 5);
        assert_eq!(5, test.secondary.index);
        assert_eq!(15, test.primary.index);
        assert_eq!(false, test.primary.prefer_next_row);
        assert_eq!(false, test.secondary.prefer_next_row);

        let test = update_selection_insert(CCursorPair::two(CCursor::new(10), CCursor::new(5)), 6, 5);
        assert_eq!(5, test.primary.index);
        assert_eq!(15, test.secondary.index);
        assert_eq!(false, test.primary.prefer_next_row);
        assert_eq!(false, test.secondary.prefer_next_row);
    }

    #[test]
    fn test_update_selection_insert_after_selection() {
        let test = update_selection_insert(CCursorPair::two(CCursor::new(5), CCursor::new(10)), 15, 5);
        assert_eq!(5, test.secondary.index);
        assert_eq!(10, test.primary.index);
        assert_eq!(false, test.primary.prefer_next_row);
        assert_eq!(false, test.secondary.prefer_next_row);

        let test = update_selection_insert(CCursorPair::two(CCursor::new(10), CCursor::new(5)), 15, 5);
        assert_eq!(5, test.primary.index);
        assert_eq!(10, test.secondary.index);
        assert_eq!(false, test.primary.prefer_next_row);
        assert_eq!(false, test.secondary.prefer_next_row);
    }

    #[test]
    fn test_update_selection_delete_before_selection_not_overlaping() {
        let test = update_selection_delete(CCursorPair::two(CCursor::new(5), CCursor::new(10)), 0, 5);
        assert_eq!(0, test.secondary.index);
        assert_eq!(5, test.primary.index);
        assert_eq!(false, test.primary.prefer_next_row);
        assert_eq!(false, test.secondary.prefer_next_row);

        let test = update_selection_delete(CCursorPair::two(CCursor::new(10), CCursor::new(5)), 0, 5);
        assert_eq!(0, test.primary.index);
        assert_eq!(5, test.secondary.index);
        assert_eq!(false, test.primary.prefer_next_row);
        assert_eq!(false, test.secondary.prefer_next_row);
    }

    #[test]
    fn test_update_selection_delete_before_selection_overlapping_min_cursor() {
        let test = update_selection_delete(CCursorPair::two(CCursor::new(5), CCursor::new(10)), 3, 5);
        assert_eq!(3, test.secondary.index);
        assert_eq!(5, test.primary.index);
        assert_eq!(false, test.primary.prefer_next_row);
        assert_eq!(false, test.secondary.prefer_next_row);

        let test = update_selection_delete(CCursorPair::two(CCursor::new(10), CCursor::new(5)), 3, 5);
        assert_eq!(3, test.primary.index);
        assert_eq!(5, test.secondary.index);
        assert_eq!(false, test.primary.prefer_next_row);
        assert_eq!(false, test.secondary.prefer_next_row);
    }

    #[test]
    fn test_update_selection_delete_inside_selection_not_overlapping() {
        let test = update_selection_delete(CCursorPair::two(CCursor::new(5), CCursor::new(10)), 6, 2);
        assert_eq!(5, test.secondary.index);
        assert_eq!(8, test.primary.index);
        assert_eq!(false, test.primary.prefer_next_row);
        assert_eq!(false, test.secondary.prefer_next_row);

        let test = update_selection_delete(CCursorPair::two(CCursor::new(10), CCursor::new(5)), 6, 2);
        assert_eq!(5, test.primary.index);
        assert_eq!(8, test.secondary.index);
        assert_eq!(false, test.primary.prefer_next_row);
        assert_eq!(false, test.secondary.prefer_next_row);
    }

    #[test]
    fn test_update_selection_delete_inside_selection_overlapping_max() {
        let test = update_selection_delete(CCursorPair::two(CCursor::new(5), CCursor::new(10)), 7, 20);
        assert_eq!(5, test.secondary.index);
        assert_eq!(7, test.primary.index);
        assert_eq!(false, test.primary.prefer_next_row);
        assert_eq!(false, test.secondary.prefer_next_row);

        let test = update_selection_delete(CCursorPair::two(CCursor::new(10), CCursor::new(5)), 7, 20);
        assert_eq!(5, test.primary.index);
        assert_eq!(7, test.secondary.index);
        assert_eq!(false, test.primary.prefer_next_row);
        assert_eq!(false, test.secondary.prefer_next_row);
    }

    #[test]
    fn test_update_selection_delete_after_selection() {
        let test = update_selection_delete(CCursorPair::two(CCursor::new(5), CCursor::new(10)), 15, 20);
        assert_eq!(5, test.secondary.index);
        assert_eq!(10, test.primary.index);
        assert_eq!(false, test.primary.prefer_next_row);
        assert_eq!(false, test.secondary.prefer_next_row);

        let test = update_selection_delete(CCursorPair::two(CCursor::new(10), CCursor::new(5)), 15, 20);
        assert_eq!(5, test.primary.index);
        assert_eq!(10, test.secondary.index);
        assert_eq!(false, test.primary.prefer_next_row);
        assert_eq!(false, test.secondary.prefer_next_row);
    }

    #[test]
    fn test_update_selection_delete_before_selection_overlapping_min_and_max() {
        let test = update_selection_delete(CCursorPair::two(CCursor::new(5), CCursor::new(10)), 3, 20);
        assert_eq!(3, test.secondary.index);
        assert_eq!(3, test.primary.index);
        assert_eq!(false, test.primary.prefer_next_row);
        assert_eq!(false, test.secondary.prefer_next_row);

        let test = update_selection_delete(CCursorPair::two(CCursor::new(10), CCursor::new(5)), 3, 20);
        assert_eq!(3, test.primary.index);
        assert_eq!(3, test.secondary.index);
        assert_eq!(false, test.primary.prefer_next_row);
        assert_eq!(false, test.secondary.prefer_next_row);
    }

    #[test]
    fn test_update_selection_delete_at_cursor_min_no_max_overlapping() {
        let test = update_selection_delete(CCursorPair::two(CCursor::new(5), CCursor::new(10)), 5, 3);
        assert_eq!(5, test.secondary.index);
        assert_eq!(7, test.primary.index);
        assert_eq!(false, test.primary.prefer_next_row);
        assert_eq!(false, test.secondary.prefer_next_row);

        let test = update_selection_delete(CCursorPair::two(CCursor::new(10), CCursor::new(5)), 5, 3);
        assert_eq!(5, test.primary.index);
        assert_eq!(7, test.secondary.index);
        assert_eq!(false, test.primary.prefer_next_row);
        assert_eq!(false, test.secondary.prefer_next_row);
    }

    #[test]
    fn test_update_selection_delete_at_cursor_min_with_max_overlapping() {
        let test = update_selection_delete(CCursorPair::two(CCursor::new(5), CCursor::new(10)), 5, 20);
        assert_eq!(5, test.secondary.index);
        assert_eq!(5, test.primary.index);
        assert_eq!(false, test.primary.prefer_next_row);
        assert_eq!(false, test.secondary.prefer_next_row);

        let test = update_selection_delete(CCursorPair::two(CCursor::new(10), CCursor::new(5)), 5, 20);
        assert_eq!(5, test.primary.index);
        assert_eq!(5, test.secondary.index);
        assert_eq!(false, test.primary.prefer_next_row);
        assert_eq!(false, test.secondary.prefer_next_row);
    }

    #[test]
    fn test_update_selection_insert_at_cursor_min() {
        let test = update_selection_insert(CCursorPair::two(CCursor::new(5), CCursor::new(10)), 5, 3);
        assert_eq!(5, test.secondary.index);
        assert_eq!(13, test.primary.index);
        assert_eq!(false, test.primary.prefer_next_row);
        assert_eq!(false, test.secondary.prefer_next_row);

        let test = update_selection_insert(CCursorPair::two(CCursor::new(10), CCursor::new(5)), 5, 3);
        assert_eq!(5, test.primary.index);
        assert_eq!(13, test.secondary.index);
        assert_eq!(false, test.primary.prefer_next_row);
        assert_eq!(false, test.secondary.prefer_next_row);
    }

    #[test]
    fn test_update_selection_insert_at_cursor_max() {
        let test = update_selection_insert(CCursorPair::two(CCursor::new(5), CCursor::new(10)), 10, 3);
        assert_eq!(5, test.secondary.index);
        assert_eq!(13, test.primary.index);
        assert_eq!(false, test.primary.prefer_next_row);
        assert_eq!(false, test.secondary.prefer_next_row);

        let test = update_selection_insert(CCursorPair::two(CCursor::new(10), CCursor::new(5)), 10, 3);
        assert_eq!(5, test.primary.index);
        assert_eq!(13, test.secondary.index);
        assert_eq!(false, test.primary.prefer_next_row);
        assert_eq!(false, test.secondary.prefer_next_row);
    }
}
