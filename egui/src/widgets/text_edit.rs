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
                        key: Key::Tab,
                        pressed: true,
                        modifiers,
                    } => {
                        if multiline {
                            let mut ccursor = delete_selected(text, &cursorp);

                            if ui.memory().has_lock_focus(id) {
                                if modifiers.shift {
                                    remove_identation(&mut ccursor, text, tab_as_spaces);
                                } else if tab_as_spaces {
                                    insert_spaces_identation(&mut ccursor, text);
                                } else {
                                    insert_text(&mut ccursor, text, "\t");
                                }
                            }

                            Some(CCursorPair::one(ccursor))
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

/// Accepts and returns character offset (NOT byte offset!).
/// Returns tuple with the indexes of the identation block from
/// current line
fn find_identation_start_and_end(text: &str, cursor: CCursor) -> (usize, usize) {
    // We know that new lines, '\n', are a single byte char, but we have to
    // work with char offsets because before the new line there may be any
    // number of multi byte chars.
    // We need to know the char index to be able to correctly set the cursor
    // later.
    let index = cursor.index;
    let chars_count = text.chars().count();

    let start = text
        .chars()
        .rev()
        .skip(chars_count - index)
        .position(|x| x == '\n')
        .unwrap_or(0);

    let end = text
        .chars()
        .skip(index)
        .take_while(|x| *x == ' ' || *x == '\t')
        .fold(index, |index, _| index + 1);

    (start, end)
}

fn convert_identation_to_spaces(
    mut ident_end: usize,
    ccursor: &mut CCursor,
    identation: &mut String,
) {
    // Because we convert tabs to spaces we add a bit more capacity
    // Hopefuly it will be enough and no other alloc will be done
    let mut new_identation = String::with_capacity(identation.len() + 5 * text::MAX_TAB_SIZE);

    let mut char_it = identation
        .chars()
        .peekable();

    let mut tab_size: usize = text::MAX_TAB_SIZE;
    while let Some(c) = char_it.peek() {
        if *c == ' ' {
            char_it.next();
            new_identation += " ";

            tab_size -= 1;

            if tab_size == 0 {
                tab_size = text::MAX_TAB_SIZE;
            }
        } else if *c == '\t' {
            if tab_size == 0 {
                tab_size = text::MAX_TAB_SIZE;
            }

            for _ in 0..tab_size {
                new_identation += " ";
            }

            if ccursor.index < ident_end {
                *ccursor += tab_size.saturating_sub(1);
                ident_end = ident_end.saturating_sub(1);
            }

            char_it.next();
        } else {
            break;
        }
    }

    *identation = new_identation;
}

fn convert_identation_to_tabs(
    mut ident_end: usize,
    ccursor: &mut CCursor,
    identation: &mut String
) {
    let mut new_identation = String::with_capacity(identation.len());

    let mut char_it = identation
        .chars()
        .peekable();

    let mut tab_size: usize = text::MAX_TAB_SIZE;
    while let Some(c) = char_it.peek() {
        if *c == ' ' {
            tab_size -= 1;

            if ccursor.index > ident_end {
                *ccursor -= 1;
                ident_end = ident_end.saturating_sub(1);
            }

            if tab_size == 0 {
                new_identation.push('\t');
                tab_size = text::MAX_TAB_SIZE;

                if ccursor.index > ident_end {
                    *ccursor += 1;
                    ident_end = ident_end.saturating_add(1);
                }
            }

            char_it.next();
        } else if *c == '\t' {
            char_it.next();
            new_identation.push('\t');

            if tab_size != text::MAX_TAB_SIZE {
                tab_size = text::MAX_TAB_SIZE;
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
    for _ in 0..(text::MAX_TAB_SIZE - tab_size) {
        new_identation.push(' ');

        if ccursor.index > ident_end {
            *ccursor += 1;
            ident_end = ident_end.saturating_add(1);
        }
    }

    *identation = new_identation;
}

fn remove_identation(ccursor: &mut CCursor, text: &mut String, tab_as_spaces: bool) {
    let mut new_text = String::with_capacity(text.len());

    let (ident_start, ident_end) = find_identation_start_and_end(text, *ccursor);

    let mut char_it = text.chars().peekable();
    for _ in 0..ident_start {
        let c = char_it.next().unwrap();
        new_text.push(c);
    }

    // Alloc space for 5 levels of indentation with spaces
    let mut identation = String::with_capacity(5 * text::MAX_TAB_SIZE);

    while let Some(c) = char_it.peek() {
        if *c == ' ' || *c == '\t' {
            identation.push(*c);
            char_it.next();
        } else {
            break;
        }
    }

    if tab_as_spaces {
        convert_identation_to_spaces(ident_end, ccursor, &mut identation);

        let mut spaces_count = identation.chars().count();
        let mut ident_it = identation.chars();

        // With spaces we have two cases
        // 1. ____ __let x = 2;   <- Here we remove the spaces
        //                           between the full spaces block
        //                           and `let`.
        //
        // 2. ____ ____let x = 1; <- Here we remove a full spaces block

        if spaces_count > 0 {
            if spaces_count % text::MAX_TAB_SIZE == 0 {
                for _ in 0..text::MAX_TAB_SIZE {
                    ident_it.next();
                    *ccursor -= 1;
                }
            } else {
                while spaces_count % text::MAX_TAB_SIZE != 0 {
                    ident_it.next();
                    spaces_count -= 1;
                    *ccursor -= 1;
                }
            }
        }

        new_text.extend(ident_it);
    } else {
        convert_identation_to_tabs(ident_end, ccursor, &mut identation);

        // With tabs we have two cases
        // 1. --->__let x = 2;   <- Here we remove the spaces
        //                          between the tab and `let`
        //
        // 2. --->--->let x = 1; <- Here we remove a tab

        let mut spaces_removed = 0;
        let mut rev_ident_it = identation.chars().rev().peekable();

        // Case 1: We remove potential spaces
        while let Some(c) = rev_ident_it.peek() {
            if *c == ' ' {
                rev_ident_it.next();
                spaces_removed += 1;
                *ccursor -= 1;
            } else {
                break;
            }
        }

        // Case 2: We remove a tab
        if spaces_removed == 0 && !identation.is_empty() {
            rev_ident_it.next();
            *ccursor -= 1
        }

        new_text.extend(rev_ident_it);
    }

    new_text.extend(char_it);

    *text = new_text;
}

fn insert_spaces_identation(ccursor: &mut CCursor, text: &mut String) {
    let mut new_text = String::with_capacity(text.len() + 5 * text::MAX_TAB_SIZE);

    let (ident_start, ident_end) = find_identation_start_and_end(text, *ccursor);

    let mut char_it = text.chars().enumerate().peekable();
    for _ in 0..ident_start {
        let (_, c) = char_it.next().unwrap();
        new_text.push(c);
    }

    let mut column = 0;
    let mut tab_size = text::MAX_TAB_SIZE;
    while let Some((index, c)) = char_it.peek() {
        if *index == ccursor.index {
            break;
        } else if *c == '\t' {
            char_it.next();
            new_text.push('\t');
            column += tab_size;

            if tab_size != text::MAX_TAB_SIZE {
                tab_size = text::MAX_TAB_SIZE;
            }
        } else {
            new_text.push(*c);
            char_it.next();
            tab_size -= 1;
            column += 1;

            if tab_size == 0 {
                tab_size = text::MAX_TAB_SIZE;
            }
        }
    }

    let mut spaces_to_insert = text::MAX_TAB_SIZE;
    spaces_to_insert -= column % text::MAX_TAB_SIZE;

    for _ in 0..spaces_to_insert {
        new_text.push(' ');

        if ident_end
        *ccursor += 1;
    }

    for (_, c) in char_it {
        new_text.push(c);
    }

    *text = new_text;
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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_find_line_start() {
        assert_eq!((0, 0), find_identation_start_and_end("", CCursor::new(0)));
        assert_eq!((0, 0), find_identation_start_and_end("ASDF", CCursor::new(4)));

        assert_eq!((5, 9), find_identation_start_and_end("ASDF\n    ASDF", CCursor::new(13)));

        assert_eq!((1, 1), find_identation_start_and_end("\n\n\n", CCursor::new(1)));
    }

    #[test]
    fn test_insert_identation_tabs_as_spaces() {
        // Insert in front
        check_insert_spaces_identation(
            &TestData {
                tabs_as_spaces: true,
                input_text: "ASDF",
                input_cursor_index: 0,
                expected_text: "    ASDF",
                expected_cursor_index: 4,
            });

        check_insert_spaces_identation(
            &TestData {
                tabs_as_spaces: true,
                input_text: "  ASDF",
                input_cursor_index: 2,
                expected_text: "    ASDF",
                expected_cursor_index: 4,
            });

        check_insert_spaces_identation(
            &TestData {
                tabs_as_spaces: true,
                input_text: "\tASDF",
                input_cursor_index: 1,
                expected_text: "\t    ASDF",
                expected_cursor_index: 5,
            });

        check_insert_spaces_identation(
            &TestData {
                tabs_as_spaces: true,
                input_text: "\t  ASDF",
                input_cursor_index: 3,
                expected_text: "\t    ASDF",
                expected_cursor_index: 5,
            });

        check_insert_spaces_identation(
            &TestData {
                tabs_as_spaces: true,
                input_text: "  \tASDF",
                input_cursor_index: 3,
                expected_text: "  \t    ASDF",
                expected_cursor_index: 7,
            });
    }

    #[test]
    fn test_remove_identation_tabs_as_spaces() {
        check_remove_identation(
            &TestData {
                tabs_as_spaces: true,
                input_text: "\nASDF",
                input_cursor_index: 3,
                expected_text: "\nASDF",
                expected_cursor_index: 3,
            });

        check_remove_identation(
            &TestData {
                tabs_as_spaces: true,
                input_text: "\n ASDF",
                input_cursor_index: 1,
                expected_text: "\nASDF",
                expected_cursor_index: 1,
            });

        check_remove_identation(
            &TestData {
                tabs_as_spaces: true,
                input_text: "\n    ASDF",
                input_cursor_index: 1,
                expected_text: "\nASDF",
                expected_cursor_index: 1,
            });

        check_remove_identation(
            &TestData {
                tabs_as_spaces: true,
                input_text: "\n\t    ASDF",
                input_cursor_index: 1,
                expected_text: "\n    ASDF",
                expected_cursor_index: 1,
            });

        check_remove_identation(
            &TestData {
                tabs_as_spaces: true,
                input_text: "\n    \tASDF",
                input_cursor_index: 1,
                expected_text: "\n    ASDF",
                expected_cursor_index: 1,
            });

        check_remove_identation(
            &TestData {
                tabs_as_spaces: true,
                input_text: "\n\t\tASDF",
                input_cursor_index: 7,
                expected_text: "\n    ASDF",
                expected_cursor_index: 9,
            });
    }

    #[test]
    fn test_remove_identation_tabs_as_tabs() {
        check_remove_identation(
            &TestData {
                tabs_as_spaces: false,
                input_text: "ASDF",
                input_cursor_index: 2,
                expected_text: "ASDF",
                expected_cursor_index: 2,
            });

        check_remove_identation(
            &TestData {
                tabs_as_spaces: false,
                input_text: " ASDF",
                input_cursor_index: 0,
                expected_text: "ASDF",
                expected_cursor_index: 0,
            });

        check_remove_identation(
            &TestData {
                tabs_as_spaces: false,
                input_text: "    ASDF",
                input_cursor_index: 0,
                expected_text: "ASDF",
                expected_cursor_index: 0,
            });

        check_remove_identation(
            &TestData {
                tabs_as_spaces: false,
                input_text: "\t    ASDF",
                input_cursor_index: 0,
                expected_text: "\tASDF",
                expected_cursor_index: 0,
            });

        check_remove_identation(
            &TestData {
                tabs_as_spaces: false,
                input_text: "    \tASDF",
                input_cursor_index: 0,
                expected_text: "\tASDF",
                expected_cursor_index: 0,
            });

        check_remove_identation(
            &TestData {
                tabs_as_spaces: false,
                input_text: "\t\tASDF",
                input_cursor_index: 6,
                expected_text: "\tASDF",
                expected_cursor_index: 5,
            });
    }

    struct TestData<'a> {
        pub tabs_as_spaces: bool,
        pub input_text: &'a str,
        pub input_cursor_index: usize,
        pub expected_text: &'a str,
        pub expected_cursor_index: usize,
    }

    fn check_insert_spaces_identation(data: &TestData<'_>) {
        let mut text = String::from(data.input_text);
        let mut cursor = CCursor::new(data.input_cursor_index);

        insert_spaces_identation(&mut cursor, &mut text);

        assert_eq!(data.expected_cursor_index, cursor.index);
        assert_eq!(data.expected_text, text);
    }

    fn check_remove_identation(data: &TestData<'_>) {
        let mut text = String::from(data.input_text);
        let mut cursor = CCursor::new(data.input_cursor_index);

        remove_identation(&mut cursor, &mut text, data.tabs_as_spaces);

        assert_eq!(data.expected_cursor_index, cursor.index);
        assert_eq!(data.expected_text, text);
    }
}
