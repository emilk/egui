use epaint::{Galley, Pos2};

use crate::{Context, CursorIcon, Event, Id, Response, Ui};

use super::{
    text_cursor_state::cursor_rect, visuals::paint_text_selection, CursorRange, TextCursorState,
};

/// Handle text selection state for a label or similar widget.
///
/// Make sure the widget senses clicks and drags.
///
/// This should be called after painting the text, because this will also
/// paint the text cursor/selection on top.
pub fn label_text_selection(ui: &Ui, response: &Response, galley_pos: Pos2, galley: &Galley) {
    let mut cursor_state = LabelSelectionState::load(ui.ctx(), response.id);
    let original_cursor = cursor_state.range(galley);

    if response.hovered {
        ui.ctx().set_cursor_icon(CursorIcon::Text);
    } else if !cursor_state.is_empty() && ui.input(|i| i.pointer.any_pressed()) {
        // We clicked somewhere else - deselect this label.
        cursor_state = Default::default();
        LabelSelectionState::store(ui.ctx(), response.id, cursor_state);
    }

    if let Some(pointer_pos) = ui.ctx().pointer_interact_pos() {
        let cursor_at_pointer = galley.cursor_from_pos(pointer_pos - galley_pos);
        cursor_state.pointer_interaction(ui, response, cursor_at_pointer, galley);
    }

    if let Some(mut cursor_range) = cursor_state.range(galley) {
        process_selection_key_events(ui.ctx(), galley, response.id, &mut cursor_range);
        cursor_state.set_range(Some(cursor_range));
    }

    let cursor_range = cursor_state.range(galley);

    if let Some(cursor_range) = cursor_range {
        // We paint the cursor on top of the text, in case
        // the text galley has backgrounds (as e.g. `code` snippets in markup do).
        paint_text_selection(
            ui.painter(),
            ui.visuals(),
            galley_pos,
            galley,
            &cursor_range,
        );

        let selection_changed = original_cursor != Some(cursor_range);

        let is_fully_visible = ui.clip_rect().contains_rect(response.rect); // TODO: remove this HACK workaround for https://github.com/emilk/egui/issues/1531

        if selection_changed && !is_fully_visible {
            // Scroll to keep primary cursor in view:
            let row_height = estimate_row_height(galley);
            let primary_cursor_rect =
                cursor_rect(galley_pos, galley, &cursor_range.primary, row_height);
            ui.scroll_to_rect(primary_cursor_rect, None);
        }
    }

    #[cfg(feature = "accesskit")]
    super::accesskit_text::update_accesskit_for_text_widget(
        ui.ctx(),
        response.id,
        cursor_range,
        accesskit::Role::StaticText,
        galley_pos,
        galley,
    );

    if !cursor_state.is_empty() {
        LabelSelectionState::store(ui.ctx(), response.id, cursor_state);
    }
}

/// Handles text selection in labels (NOT in [`crate::TextEdit`])s.
///
/// One state for all labels, because we only support text selection in one label at a time.
#[derive(Clone, Copy, Debug, Default)]
struct LabelSelectionState {
    /// Id of the (only) label with a selection, if any
    id: Option<Id>,

    /// The current selection, if any.
    selection: TextCursorState,
}

impl LabelSelectionState {
    /// Load the range of text of text that is selected for the given widget.
    fn load(ctx: &Context, id: Id) -> TextCursorState {
        ctx.data(|data| data.get_temp::<Self>(Id::NULL))
            .and_then(|state| (state.id == Some(id)).then_some(state.selection))
            .unwrap_or_default()
    }

    /// Load the range of text of text that is selected for the given widget.
    fn store(ctx: &Context, id: Id, selection: TextCursorState) {
        ctx.data_mut(|data| {
            data.insert_temp(
                Id::NULL,
                Self {
                    id: Some(id),
                    selection,
                },
            );
        });
    }
}

fn process_selection_key_events(
    ctx: &Context,
    galley: &Galley,
    widget_id: Id,
    cursor_range: &mut CursorRange,
) {
    let mut copy_text = None;
    let os = ctx.os();

    ctx.input(|i| {
        // NOTE: we have a lock on ui/ctx here,
        // so be careful to not call into `ui` or `ctx` again.

        for event in &i.events {
            match event {
                Event::Copy | Event::Cut => {
                    // This logic means we can select everything in an ellided label (including the `…`)
                    // and still copy the entire un-ellided text!
                    let everything_is_selected =
                        cursor_range.contains(&CursorRange::select_all(galley));

                    let copy_everything = cursor_range.is_empty() || everything_is_selected;

                    if copy_everything {
                        copy_text = Some(galley.text().to_owned());
                    } else {
                        copy_text = Some(cursor_range.slice_str(galley).to_owned());
                    }
                }

                event => {
                    cursor_range.on_event(os, event, galley, widget_id);
                }
            }
        }
    });

    if let Some(copy_text) = copy_text {
        ctx.copy_text(copy_text);
    }
}

fn estimate_row_height(galley: &Galley) -> f32 {
    if let Some(row) = galley.rows.first() {
        row.rect.height()
    } else {
        galley.size().y
    }
}
