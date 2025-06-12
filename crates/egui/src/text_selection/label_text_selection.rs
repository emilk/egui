use std::sync::Arc;

use emath::TSTransform;
use epaint::text::cursor::Selection;

use crate::{
    layers::ShapeIdx, text::ByteCursor, Context, CursorIcon, Event, Galley, Id, LayerId, Pos2,
    Rect, Response, Ui,
};

use super::{
    handle_event::SelectionExt as _, text_cursor_state::cursor_rect, visuals::paint_text_selection,
    TextCursorState,
};

/// Turn on to help debug this
const DEBUG: bool = false; // Don't merge `true`!

/// One end of a text selection, inside any widget.
#[derive(Clone, Copy)]
struct WidgetTextCursor {
    widget_id: Id,
    cursor: ByteCursor,

    /// Last known screen position
    pos: Pos2,
}

impl WidgetTextCursor {
    fn new(
        widget_id: Id,
        cursor: impl Into<ByteCursor>,
        global_from_galley: TSTransform,
        galley: &Galley,
    ) -> Self {
        let cursor = cursor.into();
        let pos = global_from_galley * pos_in_galley(galley, cursor);
        Self {
            widget_id,
            cursor,
            pos,
        }
    }
}

fn pos_in_galley(galley: &Galley, cursor: ByteCursor) -> Pos2 {
    galley.pos_from_cursor(cursor).center()
}

impl std::fmt::Debug for WidgetTextCursor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WidgetTextCursor")
            .field("widget_id", &self.widget_id.short_debug_format())
            .field("cursor", &self.cursor)
            .finish()
    }
}

#[derive(Clone, Copy, Debug)]
struct CurrentSelection {
    /// The selection is in this layer.
    ///
    /// This is to constrain a selection to a single Window.
    pub layer_id: LayerId,

    /// When selecting with a mouse, this is where the mouse was released.
    /// When moving with e.g. shift+arrows, this is what moves.
    /// Note that the two ends can come in any order, and also be equal (no selection).
    pub focus: WidgetTextCursor,

    /// When selecting with a mouse, this is where the mouse was first pressed.
    /// This part of the cursor does not move when shift is down.
    pub anchor: WidgetTextCursor,
}

/// Handles text selection in labels (NOT in [`crate::TextEdit`])s.
///
/// One state for all labels, because we only support text selection in one label at a time.
#[derive(Clone, Debug)]
pub struct LabelSelectionState {
    /// The current selection, if any.
    selection: Option<CurrentSelection>,

    selection_bbox_last_frame: Rect,
    selection_bbox_this_frame: Rect,

    /// Any label hovered this frame?
    any_hovered: bool,

    /// Are we in drag-to-select state?
    is_dragging: bool,

    /// Have we reached the widget containing the primary selection?
    has_reached_focus: bool,

    /// Have we reached the widget containing the secondary selection?
    has_reached_anchor: bool,

    /// Accumulated text to copy.
    text_to_copy: String,
    last_copied_galley_rect: Option<Rect>,

    /// Painted selections this frame.
    ///
    /// Kept so we can undo a bad selection visualization if we don't see both ends of the selection this frame.
    painted_selections: Vec<ShapeIdx>,
}

impl Default for LabelSelectionState {
    fn default() -> Self {
        Self {
            selection: Default::default(),
            selection_bbox_last_frame: Rect::NOTHING,
            selection_bbox_this_frame: Rect::NOTHING,
            any_hovered: Default::default(),
            is_dragging: Default::default(),
            has_reached_focus: Default::default(),
            has_reached_anchor: Default::default(),
            text_to_copy: Default::default(),
            last_copied_galley_rect: Default::default(),
            painted_selections: Default::default(),
        }
    }
}

impl LabelSelectionState {
    pub(crate) fn register(ctx: &Context) {
        ctx.on_begin_pass("LabelSelectionState", std::sync::Arc::new(Self::begin_pass));
        ctx.on_end_pass("LabelSelectionState", std::sync::Arc::new(Self::end_pass));
    }

    pub fn load(ctx: &Context) -> Self {
        let id = Id::new(ctx.viewport_id());
        ctx.data(|data| data.get_temp::<Self>(id))
            .unwrap_or_default()
    }

    pub fn store(self, ctx: &Context) {
        let id = Id::new(ctx.viewport_id());
        ctx.data_mut(|data| {
            data.insert_temp(id, self);
        });
    }

    fn begin_pass(ctx: &Context) {
        let mut state = Self::load(ctx);

        if ctx.input(|i| i.pointer.any_pressed() && !i.modifiers.shift) {
            // Maybe a new selection is about to begin, but the old one is over:
            // state.selection = None; // TODO(emilk): this makes sense, but doesn't work as expected.
        }

        state.selection_bbox_last_frame = state.selection_bbox_this_frame;
        state.selection_bbox_this_frame = Rect::NOTHING;

        state.any_hovered = false;
        state.has_reached_focus = false;
        state.has_reached_anchor = false;
        state.text_to_copy.clear();
        state.last_copied_galley_rect = None;
        state.painted_selections.clear();

        state.store(ctx);
    }

    fn end_pass(ctx: &Context) {
        let mut state = Self::load(ctx);

        if state.is_dragging {
            ctx.set_cursor_icon(CursorIcon::Text);
        }

        if !state.has_reached_focus || !state.has_reached_anchor {
            // We didn't see both cursors this frame,
            // maybe because they are outside the visible area (scrolling),
            // or one disappeared. In either case we will have horrible glitches, so let's just deselect.

            let prev_selection = state.selection.take();
            if let Some(selection) = prev_selection {
                // This was the first frame of glitch, so hide the
                // glitching by removing all painted selections:
                ctx.graphics_mut(|layers| {
                    if let Some(list) = layers.get_mut(selection.layer_id) {
                        for shape_idx in state.painted_selections.drain(..) {
                            list.mutate_shape(shape_idx, |shape| {
                                if let epaint::Shape::Text(text_shape) = &mut shape.shape {
                                    let galley = Arc::make_mut(&mut text_shape.galley);
                                    for row in &mut galley.rows {
                                        row.visuals.selection_rects = None;
                                    }
                                }
                            });
                        }
                    }
                });
            }
        }

        let pressed_escape = ctx.input(|i| i.key_pressed(crate::Key::Escape));
        let clicked_something_else = ctx.input(|i| i.pointer.any_pressed()) && !state.any_hovered;
        let delected_everything = pressed_escape || clicked_something_else;

        if delected_everything {
            state.selection = None;
        }

        if ctx.input(|i| i.pointer.any_released()) {
            state.is_dragging = false;
        }

        let text_to_copy = std::mem::take(&mut state.text_to_copy);
        if !text_to_copy.is_empty() {
            ctx.copy_text(text_to_copy);
        }

        state.store(ctx);
    }

    pub fn has_selection(&self) -> bool {
        self.selection.is_some()
    }

    pub fn clear_selection(&mut self) {
        self.selection = None;
    }

    fn copy_text(&mut self, new_galley_rect: Rect, galley: &Galley, selection: &Selection) {
        let new_text = selected_text(galley, selection);
        if new_text.is_empty() {
            return;
        }

        if self.text_to_copy.is_empty() {
            self.text_to_copy = new_text;
            self.last_copied_galley_rect = Some(new_galley_rect);
            return;
        }

        let Some(last_copied_galley_rect) = self.last_copied_galley_rect else {
            self.text_to_copy = new_text;
            self.last_copied_galley_rect = Some(new_galley_rect);
            return;
        };

        // We need to append or prepend the new text to the already copied text.
        // We need to do so intelligently.

        if last_copied_galley_rect.bottom() <= new_galley_rect.top() {
            self.text_to_copy.push('\n');
            let vertical_distance = new_galley_rect.top() - last_copied_galley_rect.bottom();
            if estimate_row_height(galley) * 0.5 < vertical_distance {
                self.text_to_copy.push('\n');
            }
        } else {
            let existing_ends_with_space =
                self.text_to_copy.chars().last().map(|c| c.is_whitespace());

            let new_text_starts_with_space_or_punctuation = new_text
                .chars()
                .next()
                .is_some_and(|c| c.is_whitespace() || c.is_ascii_punctuation());

            if existing_ends_with_space == Some(false) && !new_text_starts_with_space_or_punctuation
            {
                self.text_to_copy.push(' ');
            }
        }

        self.text_to_copy.push_str(&new_text);
        self.last_copied_galley_rect = Some(new_galley_rect);
    }

    /// Handle text selection state for a label or similar widget.
    ///
    /// Make sure the widget senses clicks and drags.
    ///
    /// This also takes care of painting the galley.
    pub fn label_text_selection(
        ui: &Ui,
        response: &Response,
        galley_pos: Pos2,
        mut galley: Arc<Galley>,
        fallback_color: epaint::Color32,
        underline: epaint::Stroke,
    ) {
        let mut state = Self::load(ui.ctx());
        let did_draw_selection = state.on_label(ui, response, galley_pos, &mut galley);

        let shape_idx = ui.painter().add(
            epaint::TextShape::new(galley_pos, galley, fallback_color).with_underline(underline),
        );

        if did_draw_selection {
            state.painted_selections.push(shape_idx);
        }

        state.store(ui.ctx());
    }

    fn cursor_for(
        &mut self,
        ui: &Ui,
        response: &Response,
        global_from_galley: TSTransform,
        galley: &Galley,
    ) -> TextCursorState {
        let Some(selection) = &mut self.selection else {
            // Nothing selected.
            return TextCursorState::default();
        };

        if selection.layer_id != response.layer_id {
            // Selection is in another layer
            return TextCursorState::default();
        }

        let galley_from_global = global_from_galley.inverse();

        let multi_widget_text_select = ui.style().interaction.multi_widget_text_select;

        let may_select_widget =
            multi_widget_text_select || selection.focus.widget_id == response.id;

        if self.is_dragging && may_select_widget {
            if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
                let galley_rect =
                    global_from_galley * Rect::from_min_size(Pos2::ZERO, galley.size());
                let galley_rect = galley_rect
                    // The response rectangle is set by hit testing, which includes interact_radius. galley_rect doesn't.
                    .expand(ui.style().interaction.interact_radius)
                    .intersect(ui.clip_rect());

                let is_in_same_column = galley_rect
                    .x_range()
                    .intersects(self.selection_bbox_last_frame.x_range());

                let has_reached_focus =
                    self.has_reached_focus || response.id == selection.focus.widget_id;
                let has_reached_anchor =
                    self.has_reached_anchor || response.id == selection.anchor.widget_id;

                let new_focus = if response.contains_pointer() {
                    // Dragging into this widget - easy case:
                    Some(galley.cursor_from_pos((galley_from_global * pointer_pos).to_vec2()))
                } else if is_in_same_column
                    && !self.has_reached_focus
                    && selection.focus.pos.y <= selection.anchor.pos.y
                    && pointer_pos.y <= galley_rect.top()
                    && galley_rect.top() <= selection.anchor.pos.y
                {
                    // The user is dragging the text selection upwards, above the first selected widget (this one):
                    if DEBUG {
                        ui.ctx()
                            .debug_text(format!("Upwards drag; include {:?}", response.id));
                    }
                    Some(galley.begin())
                } else if is_in_same_column
                    && has_reached_anchor
                    && has_reached_focus
                    && selection.anchor.pos.y <= selection.focus.pos.y
                    && selection.anchor.pos.y <= galley_rect.bottom()
                    && galley_rect.bottom() <= pointer_pos.y
                {
                    // The user is dragging the text selection downwards, below this widget.
                    // We move the cursor to the end of this widget,
                    // (and we may do the same for the next widget too).
                    if DEBUG {
                        ui.ctx()
                            .debug_text(format!("Downwards drag; include {:?}", response.id));
                    }
                    Some(galley.end())
                } else {
                    None
                };

                if let Some(new_focus) = new_focus {
                    selection.focus =
                        WidgetTextCursor::new(response.id, new_focus, global_from_galley, galley);

                    // We don't want the latency of `drag_started`.
                    let drag_started = ui.input(|i| i.pointer.any_pressed());
                    if drag_started {
                        if selection.layer_id == response.layer_id {
                            if ui.input(|i| i.modifiers.shift) {
                                // A continuation of a previous selection.
                            } else {
                                // A new selection in the same layer.
                                selection.anchor = selection.focus;
                            }
                        } else {
                            // A new selection in a new layer.
                            selection.layer_id = response.layer_id;
                            selection.anchor = selection.focus;
                        }
                    }
                }
            }
        }

        let has_focus = response.id == selection.focus.widget_id;
        let has_anchor = response.id == selection.anchor.widget_id;

        if has_focus {
            selection.focus.pos =
                global_from_galley * pos_in_galley(galley, selection.focus.cursor);
        }
        if has_anchor {
            selection.anchor.pos =
                global_from_galley * pos_in_galley(galley, selection.anchor.cursor);
        }

        self.has_reached_focus |= has_focus;
        self.has_reached_anchor |= has_anchor;

        let focus = has_focus.then_some(selection.focus.cursor);
        let anchor = has_anchor.then_some(selection.anchor.cursor);

        // The following code assumes we will encounter both ends of the cursor
        // at some point (but in any order).
        // If we don't (e.g. because one endpoint is outside the visible scroll areas),
        // we will have annoying failure cases.

        match (focus, anchor) {
            (Some(focus), Some(anchor)) => {
                // This is the only selected label.
                TextCursorState::from(galley.selection(|s| s.select_cursor_range(&anchor, &focus)))
            }

            (Some(focus), None) => {
                // This labels contains only the primary cursor.
                let anchor = if self.has_reached_anchor {
                    // Secondary was before primary.
                    // Select everything up to the cursor.
                    // We assume normal left-to-right and top-down layout order here.
                    galley.begin()
                } else {
                    // Select everything from the cursor onward:
                    galley.end()
                };
                TextCursorState::from(galley.selection(|s| s.select_cursor_range(&anchor, &focus)))
            }

            (None, Some(anchor)) => {
                // This labels contains only the secondary cursor
                let focus = if self.has_reached_focus {
                    // Primary was before secondary.
                    // Select everything up to the cursor.
                    // We assume normal left-to-right and top-down layout order here.
                    galley.begin()
                } else {
                    // Select everything from the cursor onward:
                    galley.end()
                };
                TextCursorState::from(galley.selection(|s| s.select_cursor_range(&anchor, &focus)))
            }

            (None, None) => {
                // This widget has neither the primary or secondary cursor.
                let is_in_middle = self.has_reached_focus != self.has_reached_anchor;
                if is_in_middle {
                    if DEBUG {
                        response.ctx.debug_text(format!(
                            "widget in middle: {:?}, between {:?} and {:?}",
                            response.id, selection.focus.widget_id, selection.anchor.widget_id,
                        ));
                    }
                    // …but it is between the two selection endpoints, and so is fully selected.
                    TextCursorState::from(galley.selection(|s| s.select_all()))
                } else {
                    // Outside the selected range
                    TextCursorState::default()
                }
            }
        }
    }

    /// Returns true if any selections were painted.
    fn on_label(
        &mut self,
        ui: &Ui,
        response: &Response,
        galley_pos_in_layer: Pos2,
        galley: &mut Arc<Galley>,
    ) -> bool {
        let widget_id = response.id;

        let global_from_layer = ui
            .ctx()
            .layer_transform_to_global(ui.layer_id())
            .unwrap_or_default();
        let layer_from_galley = TSTransform::from_translation(galley_pos_in_layer.to_vec2());
        let galley_from_layer = layer_from_galley.inverse();
        let layer_from_global = global_from_layer.inverse();
        let galley_from_global = galley_from_layer * layer_from_global;
        let global_from_galley = global_from_layer * layer_from_galley;

        if response.hovered() {
            ui.ctx().set_cursor_icon(CursorIcon::Text);
        }

        self.any_hovered |= response.hovered();
        self.is_dragging |= response.is_pointer_button_down_on(); // we don't want the initial latency of drag vs click decision

        let old_selection = self.selection;

        let mut cursor_state = self.cursor_for(ui, response, global_from_galley, galley);

        let old_layout_selection = cursor_state.selection();

        if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
            if response.contains_pointer() {
                let galley_space_pos = (galley_from_global * pointer_pos).to_vec2();

                // This is where we handle start-of-drag and double-click-to-select.
                // Actual drag-to-select happens elsewhere.
                let dragged = false;
                cursor_state.pointer_interaction(ui, response, galley_space_pos, galley, dragged);
            }
        }

        if let Some(mut layout_selection) = cursor_state.selection() {
            let galley_rect = global_from_galley * Rect::from_min_size(Pos2::ZERO, galley.size());
            self.selection_bbox_this_frame = self.selection_bbox_this_frame.union(galley_rect);

            if let Some(selection) = &self.selection {
                if selection.focus.widget_id == response.id {
                    process_selection_key_events(
                        ui.ctx(),
                        galley,
                        response.id,
                        &mut layout_selection,
                    );
                }
            }

            if got_copy_event(ui.ctx()) {
                self.copy_text(galley_rect, galley, &layout_selection);
            }

            cursor_state.set_selection(Some(layout_selection));
        }

        // Look for changes due to keyboard and/or mouse interaction:
        let new_layout_selection = cursor_state.selection();
        let selection_changed = old_layout_selection != new_layout_selection;

        if let (true, Some(range)) = (selection_changed, new_layout_selection) {
            // --------------
            // Store results:

            if let Some(selection) = &mut self.selection {
                let focus_changed = Some(range.focus()) != old_layout_selection.map(|r| r.focus());
                let anchor_changed =
                    Some(range.anchor()) != old_layout_selection.map(|r| r.anchor());

                selection.layer_id = response.layer_id;

                if focus_changed || !ui.style().interaction.multi_widget_text_select {
                    selection.focus =
                        WidgetTextCursor::new(widget_id, range.focus(), global_from_galley, galley);
                    self.has_reached_focus = true;
                }
                if anchor_changed || !ui.style().interaction.multi_widget_text_select {
                    selection.anchor = WidgetTextCursor::new(
                        widget_id,
                        range.anchor(),
                        global_from_galley,
                        galley,
                    );
                    self.has_reached_anchor = true;
                }
            } else {
                // Start of a new selection
                self.selection = Some(CurrentSelection {
                    layer_id: response.layer_id,
                    focus: WidgetTextCursor::new(
                        widget_id,
                        range.focus(),
                        global_from_galley,
                        galley,
                    ),
                    anchor: WidgetTextCursor::new(
                        widget_id,
                        range.anchor(),
                        global_from_galley,
                        galley,
                    ),
                });
                self.has_reached_focus = true;
                self.has_reached_anchor = true;
            }
        }

        // Scroll containing ScrollArea on cursor change:
        if let Some(range) = new_layout_selection {
            let old_primary = old_selection.map(|s| s.focus);
            let new_primary = self.selection.as_ref().map(|s| s.focus);
            if let Some(new_primary) = new_primary {
                let primary_changed = old_primary.is_none_or(|old| {
                    old.widget_id != new_primary.widget_id || old.cursor != new_primary.cursor
                });
                if primary_changed && new_primary.widget_id == widget_id {
                    let is_fully_visible = ui.clip_rect().contains_rect(response.rect); // TODO(emilk): remove this HACK workaround for https://github.com/emilk/egui/issues/1531
                    if selection_changed && !is_fully_visible {
                        // Scroll to keep primary cursor in view:
                        let row_height = estimate_row_height(galley);
                        let primary_cursor_rect =
                            global_from_galley * cursor_rect(galley, &range.focus(), row_height);
                        ui.scroll_to_rect(primary_cursor_rect, None);
                    }
                }
            }
        }

        let selection = cursor_state.selection();

        let did_draw_selection = selection
            .is_some_and(|selection| paint_text_selection(galley, ui.visuals(), &selection));

        #[cfg(feature = "accesskit")]
        super::accesskit_text::update_accesskit_for_text_widget(
            ui.ctx(),
            response.id,
            selection,
            accesskit::Role::Label,
            global_from_galley,
            galley,
        );

        did_draw_selection
    }
}

fn got_copy_event(ctx: &Context) -> bool {
    ctx.input(|i| {
        i.events
            .iter()
            .any(|e| matches!(e, Event::Copy | Event::Cut))
    })
}

/// Returns true if the cursor changed
fn process_selection_key_events(
    ctx: &Context,
    galley: &Galley,
    widget_id: Id,
    selection: &mut Selection,
) -> bool {
    let os = ctx.os();

    let mut changed = false;

    ctx.input(|i| {
        // NOTE: we have a lock on ui/ctx here,
        // so be careful to not call into `ui` or `ctx` again.
        for event in &i.events {
            if let Some(new_selection) = selection.on_event(os, event, galley, widget_id) {
                changed = true;
                *selection = new_selection;
            }
        }
    });

    changed
}

fn selected_text(galley: &Galley, selection: &Selection) -> String {
    // This logic means we can select everything in an elided label (including the `…`)
    // and still copy the entire un-elided text!
    let everything_is_selected = selection.contains(&galley.selection(|s| s.select_all()));

    let copy_everything = selection.is_empty() || everything_is_selected;

    if copy_everything {
        galley.text().to_owned()
    } else {
        selection.slice_str(galley).to_owned()
    }
}

fn estimate_row_height(galley: &Galley) -> f32 {
    if let Some(row) = galley.rows.first() {
        row.rect.height()
    } else {
        galley.size().y
    }
}
