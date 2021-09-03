use crate::{output::OutputEvent, util::undoer::Undoer, *};
use epaint::text::{cursor::*, Galley, LayoutJob};
use std::ops::Range;
use std::sync::Arc;

#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "persistence", serde(default))]
pub(crate) struct State {
    cursorp: Option<CursorPair>,

    #[cfg_attr(feature = "persistence", serde(skip))]
    undoer: Undoer<(CCursorPair, String)>,

    // If IME candidate window is shown on this text edit.
    #[cfg_attr(feature = "persistence", serde(skip))]
    has_ime: bool,

    // Visual offset when editing singleline text bigger than the width.
    #[cfg_attr(feature = "persistence", serde(skip))]
    singleline_offset: f32,
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

/// Trait constraining what types [`TextEdit`] may use as
/// an underlying buffer.
///
/// Most likely you will use a `String` which implements `TextBuffer`.
pub trait TextBuffer: AsRef<str> + Into<String> {
    /// Inserts text `text` into this buffer at character index `ch_idx`.
    ///
    /// # Notes
    /// `ch_idx` is a *character index*, not a byte index.
    ///
    /// # Return
    /// Returns how many *characters* were successfully inserted
    fn insert_text(&mut self, text: &str, ch_idx: usize) -> usize;

    /// Deletes a range of text `ch_range` from this buffer.
    ///
    /// # Notes
    /// `ch_range` is a *character range*, not a byte range.
    fn delete_char_range(&mut self, ch_range: Range<usize>);

    /// Returns this buffer as a `str`.
    ///
    /// This is an utility method, as it simply relies on the `AsRef<str>`
    /// implementation.
    fn as_str(&self) -> &str {
        self.as_ref()
    }

    /// Clears all characters in this buffer
    fn clear(&mut self) {
        self.delete_char_range(0..self.as_ref().len());
    }

    /// Replaces all contents of this string with `text`
    fn replace(&mut self, text: &str) {
        self.clear();
        self.insert_text(text, 0);
    }

    /// Clears all characters in this buffer and returns a string of the contents.
    fn take(&mut self) -> String {
        let s = self.as_ref().to_owned();
        self.clear();
        s
    }
}

impl TextBuffer for String {
    fn insert_text(&mut self, text: &str, ch_idx: usize) -> usize {
        // Get the byte index from the character index
        let byte_idx = self::byte_index_from_char_index(self, ch_idx);

        // Then insert the string
        self.insert_str(byte_idx, text);

        text.chars().count()
    }

    fn delete_char_range(&mut self, ch_range: Range<usize>) {
        assert!(ch_range.start <= ch_range.end);

        // Get both byte indices
        let byte_start = self::byte_index_from_char_index(self, ch_range.start);
        let byte_end = self::byte_index_from_char_index(self, ch_range.end);

        // Then drain all characters within this range
        self.drain(byte_start..byte_end);
    }

    fn clear(&mut self) {
        self.clear();
    }

    fn replace(&mut self, text: &str) {
        *self = text.to_owned();
    }

    fn take(&mut self) -> String {
        std::mem::take(self)
    }
}

/// A text region that the user can edit the contents of.
///
/// See also [`Ui::text_edit_singleline`] and  [`Ui::text_edit_multiline`].
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
///
/// To fill an [`Ui`] with a [`TextEdit`] use [`Ui::add_sized`]:
///
/// ```
/// # let mut ui = egui::Ui::__test();
/// # let mut my_string = String::new();
/// ui.add_sized(ui.available_size(), egui::TextEdit::multiline(&mut my_string));
/// ```
///
#[must_use = "You should put this widget in an ui with `ui.add(widget);`"]
pub struct TextEdit<'t, S: TextBuffer = String> {
    text: &'t mut S,
    hint_text: String,
    id: Option<Id>,
    id_source: Option<Id>,
    text_style: Option<TextStyle>,
    text_color: Option<Color32>,
    layouter: Option<&'t mut dyn FnMut(&Ui, &str, f32) -> Arc<Galley>>,
    password: bool,
    frame: bool,
    multiline: bool,
    enabled: bool,
    desired_width: Option<f32>,
    desired_height_rows: usize,
    lock_focus: bool,
    cursor_at_end: bool,
}

impl<'t, S: TextBuffer> TextEdit<'t, S> {
    pub fn cursor(ui: &Ui, id: Id) -> Option<CursorPair> {
        ui.memory()
            .id_data
            .get::<State>(&id)
            .and_then(|state| state.cursorp)
    }
}

impl<'t, S: TextBuffer> TextEdit<'t, S> {
    /// No newlines (`\n`) allowed. Pressing enter key will result in the `TextEdit` losing focus (`response.lost_focus`).
    pub fn singleline(text: &'t mut S) -> Self {
        Self {
            desired_height_rows: 1,
            multiline: false,
            ..Self::multiline(text)
        }
    }

    /// A `TextEdit` for multiple lines. Pressing enter key will create a new line.
    pub fn multiline(text: &'t mut S) -> Self {
        Self {
            text,
            hint_text: Default::default(),
            id: None,
            id_source: None,
            text_style: None,
            text_color: None,
            layouter: None,
            password: false,
            frame: true,
            multiline: true,
            enabled: true,
            desired_width: None,
            desired_height_rows: 4,
            lock_focus: false,
            cursor_at_end: true,
        }
    }

    /// Build a `TextEdit` focused on code editing.
    /// By default it comes with:
    /// - monospaced font
    /// - focus lock
    pub fn code_editor(self) -> Self {
        self.text_style(TextStyle::Monospace).lock_focus(true)
    }

    /// Use if you want to set an explicit `Id` for this widget.
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
    #[allow(clippy::needless_pass_by_value)]
    pub fn hint_text(mut self, hint_text: impl ToString) -> Self {
        self.hint_text = hint_text.to_string();
        self
    }

    /// If true, hide the letters from view and prevent copying from the field.
    pub fn password(mut self, password: bool) -> Self {
        self.password = password;
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

    /// Override how text is being shown inside the `TextEdit`.
    ///
    /// This can be used to implement things like syntax highlighting.
    ///
    /// This function will be called at least once per frame,
    /// so it is strongly suggested that you cache the results of any syntax highlighter
    /// so as not to waste CPU highlighting the same string every frame.
    ///
    /// The arguments is the enclosing [`Ui`] (so you can access e.g. [`Ui::fonts`]),
    /// the text and the wrap width.
    ///
    /// ```
    /// # let ui = &mut egui::Ui::__test();
    /// # let mut my_code = String::new();
    /// # fn my_memoized_highlighter(s: &str) -> egui::text::LayoutJob { Default::default() }
    /// let mut layouter = |ui: &egui::Ui, string: &str, wrap_width: f32| {
    ///     let mut layout_job: egui::text::LayoutJob = my_memoized_highlighter(string);
    ///     layout_job.wrap_width = wrap_width;
    ///     ui.fonts().layout_job(layout_job)
    /// };
    /// ui.add(egui::TextEdit::multiline(&mut my_code).layouter(&mut layouter));
    /// ```
    pub fn layouter(mut self, layouter: &'t mut dyn FnMut(&Ui, &str, f32) -> Arc<Galley>) -> Self {
        self.layouter = Some(layouter);

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

    /// Set to 0.0 to keep as small as possible.
    /// Set to [`f32::INFINITY`] to take up all available space.
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

    /// When `false` (default), pressing TAB will move focus
    /// to the next widget.
    ///
    /// When `true`, the widget will keep the focus and pressing TAB
    /// will insert the `'\t'` character.
    pub fn lock_focus(mut self, b: bool) -> Self {
        self.lock_focus = b;
        self
    }

    /// When `true` (default), the cursor will initially be placed at the end of the text.
    ///
    /// When `false`, the cursor will initially be placed at the beginning of the text.
    pub fn cursor_at_end(mut self, b: bool) -> Self {
        self.cursor_at_end = b;
        self
    }
}

impl<'t, S: TextBuffer> Widget for TextEdit<'t, S> {
    fn ui(self, ui: &mut Ui) -> Response {
        let frame = self.frame;
        let where_to_put_background = ui.painter().add(Shape::Noop);

        let margin = Vec2::new(4.0, 2.0);
        let max_rect = ui.available_rect_before_wrap().shrink2(margin);
        let mut content_ui = ui.child_ui(max_rect, *ui.layout());
        let response = self.content_ui(&mut content_ui);
        let id = response.id;
        let frame_rect = response.rect.expand2(margin);
        ui.allocate_rect(frame_rect, Sense::hover());
        let frame_response = ui.interact(frame_rect, id, Sense::click());
        let response = response | frame_response;
        if response.clicked() && !response.lost_focus() {
            ui.memory().request_focus(response.id);
        }

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

fn mask_massword(text: &str) -> String {
    std::iter::repeat(epaint::text::PASSWORD_REPLACEMENT_CHAR)
        .take(text.chars().count())
        .collect::<String>()
}

fn mask_if_password(is_password: bool, text: &str) -> String {
    if is_password {
        mask_massword(text)
    } else {
        text.to_owned()
    }
}

impl<'t, S: TextBuffer> TextEdit<'t, S> {
    fn content_ui(self, ui: &mut Ui) -> Response {
        let TextEdit {
            text,
            hint_text,
            id,
            id_source,
            text_style,
            text_color,
            layouter,
            password,
            frame: _,
            multiline,
            enabled,
            desired_width,
            desired_height_rows,
            lock_focus,
            cursor_at_end,
        } = self;

        let text_color = text_color
            .or(ui.visuals().override_text_color)
            // .unwrap_or_else(|| ui.style().interact(&response).text_color()); // too bright
            .unwrap_or_else(|| ui.visuals().widgets.inactive.text_color());

        let prev_text = text.as_ref().to_owned();
        let text_style = text_style
            .or(ui.style().override_text_style)
            .unwrap_or_else(|| ui.style().body_text_style);
        let row_height = ui.fonts().row_height(text_style);
        const MIN_WIDTH: f32 = 24.0; // Never make a `TextEdit` more narrow than this.
        let available_width = ui.available_width().at_least(MIN_WIDTH);
        let desired_width = desired_width.unwrap_or_else(|| ui.spacing().text_edit_width);
        let wrap_width = if ui.layout().horizontal_justify() {
            available_width
        } else {
            desired_width.min(available_width)
        };

        let mut default_layouter = move |ui: &Ui, text: &str, wrap_width: f32| {
            let text = mask_if_password(password, text);
            ui.fonts().layout_job(if multiline {
                LayoutJob::simple(text, text_style, text_color, wrap_width)
            } else {
                LayoutJob::simple_singleline(text, text_style, text_color)
            })
        };

        let layouter = layouter.unwrap_or(&mut default_layouter);

        let copy_if_not_password = |ui: &Ui, text: String| {
            if !password {
                ui.ctx().output().copied_text = text;
            }
        };

        let mut galley = layouter(ui, text.as_ref(), wrap_width);

        let desired_height = (desired_height_rows.at_least(1) as f32) * row_height;
        let desired_size = vec2(wrap_width, galley.size.y.max(desired_height));
        let (auto_id, rect) = ui.allocate_space(desired_size);

        let id = id.unwrap_or_else(|| {
            if let Some(id_source) = id_source {
                ui.make_persistent_id(id_source)
            } else {
                auto_id // Since we are only storing the cursor a persistent Id is not super important
            }
        });
        let mut state = ui.memory().id_data.get_or_default::<State>(id).clone();

        let sense = if enabled {
            Sense::click_and_drag()
        } else {
            Sense::hover()
        };
        let mut response = ui.interact(rect, id, sense);
        let painter = ui.painter_at(rect);

        if enabled {
            if let Some(pointer_pos) = ui.input().pointer.interact_pos() {
                // TODO: triple-click to select whole paragraph
                // TODO: drag selected text to either move or clone (ctrl on windows, alt on mac)
                let singleline_offset = vec2(state.singleline_offset, 0.0);
                let cursor_at_pointer =
                    galley.cursor_from_pos(pointer_pos - response.rect.min + singleline_offset);

                if ui.visuals().text_cursor_preview
                    && response.hovered()
                    && ui.input().pointer.is_moving()
                {
                    // preview:
                    paint_cursor_end(
                        ui,
                        row_height,
                        &painter,
                        response.rect.min,
                        &galley,
                        &cursor_at_pointer,
                    );
                }

                if response.double_clicked() {
                    // Select word:
                    let center = cursor_at_pointer;
                    let ccursorp = select_word_at(text.as_ref(), center.ccursor);
                    state.cursorp = Some(CursorPair {
                        primary: galley.from_ccursor(ccursorp.primary),
                        secondary: galley.from_ccursor(ccursorp.secondary),
                    });
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
                } else if ui.input().pointer.any_down() && response.is_pointer_button_down_on() {
                    if let Some(cursorp) = &mut state.cursorp {
                        cursorp.primary = cursor_at_pointer;
                    }
                }
            }
        }

        if response.hovered() && enabled {
            ui.output().cursor_icon = CursorIcon::Text;
        }

        let mut text_cursor = None;
        let prev_text_cursor = state.cursorp;
        if ui.memory().has_focus(id) && enabled {
            ui.memory().lock_focus(id, lock_focus);

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
                .unwrap_or_else(|| {
                    if cursor_at_end {
                        CursorPair::one(galley.end())
                    } else {
                        CursorPair::default()
                    }
                });

            // We feed state to the undoer both before and after handling input
            // so that the undoer creates automatic saves even when there are no events for a while.
            state.undoer.feed_state(
                ui.input().time,
                &(cursorp.as_ccursorp(), text.as_ref().to_owned()),
            );

            for event in &ui.input().events {
                let did_mutate_text = match event {
                    Event::Copy => {
                        if cursorp.is_empty() {
                            copy_if_not_password(ui, text.as_ref().to_owned());
                        } else {
                            copy_if_not_password(
                                ui,
                                selected_str(text.as_ref(), &cursorp).to_owned(),
                            );
                        }
                        None
                    }
                    Event::Cut => {
                        if cursorp.is_empty() {
                            copy_if_not_password(ui, text.take());
                            Some(CCursorPair::default())
                        } else {
                            copy_if_not_password(
                                ui,
                                selected_str(text.as_ref(), &cursorp).to_owned(),
                            );
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
                        if multiline && ui.memory().has_lock_focus(id) {
                            let mut ccursor = delete_selected(text, &cursorp);
                            if modifiers.shift {
                                // TODO: support removing indentation over a selection?
                                decrease_identation(&mut ccursor, text);
                            } else {
                                insert_text(&mut ccursor, text, "\t");
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
                        if let Some((undo_ccursorp, undo_txt)) = state
                            .undoer
                            .undo(&(cursorp.as_ccursorp(), text.as_ref().to_owned()))
                        {
                            text.replace(undo_txt);
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

                    Event::CompositionStart => {
                        state.has_ime = true;
                        None
                    }

                    Event::CompositionUpdate(text_mark) => {
                        if !text_mark.is_empty()
                            && text_mark != "\n"
                            && text_mark != "\r"
                            && state.has_ime
                        {
                            let mut ccursor = delete_selected(text, &cursorp);
                            let start_cursor = ccursor;
                            insert_text(&mut ccursor, text, text_mark);
                            Some(CCursorPair::two(start_cursor, ccursor))
                        } else {
                            None
                        }
                    }

                    Event::CompositionEnd(prediction) => {
                        if !prediction.is_empty()
                            && prediction != "\n"
                            && prediction != "\r"
                            && state.has_ime
                        {
                            state.has_ime = false;
                            let mut ccursor = delete_selected(text, &cursorp);
                            insert_text(&mut ccursor, text, prediction);
                            Some(CCursorPair::one(ccursor))
                        } else {
                            None
                        }
                    }

                    _ => None,
                };

                if let Some(new_ccursorp) = did_mutate_text {
                    response.mark_changed();

                    // Layout again to avoid frame delay, and to keep `text` and `galley` in sync.
                    galley = layouter(ui, text.as_ref(), wrap_width);

                    // Set cursorp using new galley:
                    cursorp = CursorPair {
                        primary: galley.from_ccursor(new_ccursorp.primary),
                        secondary: galley.from_ccursor(new_ccursorp.secondary),
                    };
                }
            }
            state.cursorp = Some(cursorp);
            text_cursor = Some(cursorp);

            state.undoer.feed_state(
                ui.input().time,
                &(cursorp.as_ccursorp(), text.as_ref().to_owned()),
            );
        }

        let mut text_draw_pos = response.rect.min;

        // Visual clipping for singleline text editor with text larger than width
        if !multiline {
            let cursor_pos = match (state.cursorp, ui.memory().has_focus(id)) {
                (Some(cursorp), true) => galley.pos_from_cursor(&cursorp.primary).min.x,
                _ => 0.0,
            };

            let mut offset_x = state.singleline_offset;
            let visible_range = offset_x..=offset_x + desired_size.x;

            if !visible_range.contains(&cursor_pos) {
                if cursor_pos < *visible_range.start() {
                    offset_x = cursor_pos;
                } else {
                    offset_x = cursor_pos - desired_size.x;
                }
            }

            offset_x = offset_x
                .at_most(galley.size.x - desired_size.x)
                .at_least(0.0);

            state.singleline_offset = offset_x;
            text_draw_pos -= vec2(offset_x, 0.0);
        }

        if ui.memory().has_focus(id) {
            if let Some(cursorp) = state.cursorp {
                paint_cursor_selection(ui, &painter, text_draw_pos, &galley, &cursorp);
                paint_cursor_end(
                    ui,
                    row_height,
                    &painter,
                    text_draw_pos,
                    &galley,
                    &cursorp.primary,
                );

                if enabled {
                    ui.ctx().output().text_cursor_pos = Some(
                        galley
                            .pos_from_cursor(&cursorp.primary)
                            .translate(response.rect.min.to_vec2())
                            .left_top(),
                    );
                }
            }
        }

        painter.galley(text_draw_pos, galley);
        if text.as_ref().is_empty() && !hint_text.is_empty() {
            let hint_text_color = ui.visuals().weak_text_color();
            let galley = ui.fonts().layout_job(if multiline {
                LayoutJob::simple(hint_text, text_style, hint_text_color, desired_size.x)
            } else {
                LayoutJob::simple_singleline(hint_text, text_style, hint_text_color)
            });
            painter.galley(response.rect.min, galley);
        }

        ui.memory().id_data.insert(id, state);

        let selection_changed = if let (Some(text_cursor), Some(prev_text_cursor)) =
            (text_cursor, prev_text_cursor)
        {
            text_cursor.primary.ccursor.index != prev_text_cursor.primary.ccursor.index
                || text_cursor.secondary.ccursor.index != prev_text_cursor.secondary.ccursor.index
        } else {
            false
        };

        if response.changed {
            response.widget_info(|| {
                WidgetInfo::text_edit(
                    mask_if_password(password, prev_text.as_str()),
                    mask_if_password(password, text.as_str()),
                )
            });
        } else if selection_changed {
            let text_cursor = text_cursor.unwrap();
            let char_range =
                text_cursor.primary.ccursor.index..=text_cursor.secondary.ccursor.index;
            let info = WidgetInfo::text_selection_changed(
                char_range,
                mask_if_password(password, text.as_str()),
            );
            response
                .ctx
                .output()
                .events
                .push(OutputEvent::TextSelectionChanged(info));
        } else {
            response.widget_info(|| {
                WidgetInfo::text_edit(
                    mask_if_password(password, prev_text.as_str()),
                    mask_if_password(password, text.as_str()),
                )
            });
        }
        response
    }
}

// ----------------------------------------------------------------------------

fn paint_cursor_selection(
    ui: &mut Ui,
    painter: &Painter,
    pos: Pos2,
    galley: &Galley,
    cursorp: &CursorPair,
) {
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
            row.rect.left()
        };
        let right = if ri == max.row {
            row.x_offset(max.column)
        } else {
            let newline_size = if row.ends_with_newline {
                row.height() / 2.0 // visualize that we select the newline
            } else {
                0.0
            };
            row.rect.right() + newline_size
        };
        let rect = Rect::from_min_max(
            pos + vec2(left, row.min_y()),
            pos + vec2(right, row.max_y()),
        );
        painter.rect_filled(rect, 0.0, color);
    }
}

fn paint_cursor_end(
    ui: &mut Ui,
    row_height: f32,
    painter: &Painter,
    pos: Pos2,
    galley: &Galley,
    cursor: &Cursor,
) {
    let stroke = ui.visuals().selection.stroke;

    let mut cursor_pos = galley.pos_from_cursor(cursor).translate(pos.to_vec2());
    cursor_pos.max.y = cursor_pos.max.y.at_least(cursor_pos.min.y + row_height); // Handle completely empty galleys
    cursor_pos = cursor_pos.expand(1.5); // slightly above/below row

    let top = cursor_pos.center_top();
    let bottom = cursor_pos.center_bottom();

    painter.line_segment(
        [top, bottom],
        (ui.visuals().text_cursor_width, stroke.color),
    );

    if false {
        // Roof/floor:
        let extrusion = 3.0;
        let width = 1.0;
        painter.line_segment(
            [top - vec2(extrusion, 0.0), top + vec2(extrusion, 0.0)],
            (width, stroke.color),
        );
        painter.line_segment(
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

fn insert_text<S: TextBuffer>(ccursor: &mut CCursor, text: &mut S, text_to_insert: &str) {
    ccursor.index += text.insert_text(text_to_insert, ccursor.index);
}

// ----------------------------------------------------------------------------

fn delete_selected<S: TextBuffer>(text: &mut S, cursorp: &CursorPair) -> CCursor {
    let [min, max] = cursorp.sorted();
    delete_selected_ccursor_range(text, [min.ccursor, max.ccursor])
}

fn delete_selected_ccursor_range<S: TextBuffer>(text: &mut S, [min, max]: [CCursor; 2]) -> CCursor {
    text.delete_char_range(min.index..max.index);
    CCursor {
        index: min.index,
        prefer_next_row: true,
    }
}

fn delete_previous_char<S: TextBuffer>(text: &mut S, ccursor: CCursor) -> CCursor {
    if ccursor.index > 0 {
        let max_ccursor = ccursor;
        let min_ccursor = max_ccursor - 1;
        delete_selected_ccursor_range(text, [min_ccursor, max_ccursor])
    } else {
        ccursor
    }
}

fn delete_next_char<S: TextBuffer>(text: &mut S, ccursor: CCursor) -> CCursor {
    delete_selected_ccursor_range(text, [ccursor, ccursor + 1])
}

fn delete_previous_word<S: TextBuffer>(text: &mut S, max_ccursor: CCursor) -> CCursor {
    let min_ccursor = ccursor_previous_word(text.as_ref(), max_ccursor);
    delete_selected_ccursor_range(text, [min_ccursor, max_ccursor])
}

fn delete_next_word<S: TextBuffer>(text: &mut S, min_ccursor: CCursor) -> CCursor {
    let max_ccursor = ccursor_next_word(text.as_ref(), min_ccursor);
    delete_selected_ccursor_range(text, [min_ccursor, max_ccursor])
}

fn delete_paragraph_before_cursor<S: TextBuffer>(
    text: &mut S,
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

fn delete_paragraph_after_cursor<S: TextBuffer>(
    text: &mut S,
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
fn on_key_press<S: TextBuffer>(
    cursorp: &mut CursorPair,
    text: &mut S,
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
        Key::Delete if !(cfg!(target_os = "windows") && modifiers.shift) => {
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
                *cursor = galley.from_ccursor(ccursor_previous_word(galley.text(), cursor.ccursor));
            } else if modifiers.mac_cmd {
                *cursor = galley.cursor_begin_of_row(cursor);
            } else {
                *cursor = galley.cursor_left_one_character(cursor);
            }
        }
        Key::ArrowRight => {
            if modifiers.alt || modifiers.ctrl {
                // alt on mac, ctrl on windows
                *cursor = galley.from_ccursor(ccursor_next_word(galley.text(), cursor.ccursor));
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

/// Accepts and returns character offset (NOT byte offset!).
fn find_line_start(text: &str, current_index: CCursor) -> CCursor {
    // We know that new lines, '\n', are a single byte char, but we have to
    // work with char offsets because before the new line there may be any
    // number of multi byte chars.
    // We need to know the char index to be able to correctly set the cursor
    // later.
    let chars_count = text.chars().count();

    let position = text
        .chars()
        .rev()
        .skip(chars_count - current_index.index)
        .position(|x| x == '\n');

    match position {
        Some(pos) => CCursor::new(current_index.index - pos),
        None => CCursor::new(0),
    }
}

fn decrease_identation<S: TextBuffer>(ccursor: &mut CCursor, text: &mut S) {
    let line_start = find_line_start(text.as_ref(), *ccursor);

    let remove_len = if text.as_ref()[line_start.index..].starts_with('\t') {
        Some(1)
    } else if text.as_ref()[line_start.index..]
        .chars()
        .take(text::TAB_SIZE)
        .all(|c| c == ' ')
    {
        Some(text::TAB_SIZE)
    } else {
        None
    };

    if let Some(len) = remove_len {
        text.delete_char_range(line_start.index..(line_start.index + len));
        if *ccursor != line_start {
            *ccursor -= len;
        }
    }
}
