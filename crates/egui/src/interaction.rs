//! How mouse and touch interzcts with widgets.

use crate::*;

use self::{hit_test::WidgetHits, input_state::PointerEvent, memory::InteractionState};

/// Calculated at the start of each frame
/// based on:
/// * Widget rects from precious frame
/// * Mouse/touch input
/// * Current [`InteractionState`].
#[derive(Clone, Default)]
pub struct InteractionSnapshot {
    /// The widget that got clicked this frame.
    pub clicked: Option<WidgetRect>,

    /// Drag started on this widget this frame.
    ///
    /// This will also be found in `dragged` this frame.
    pub drag_started: Option<WidgetRect>,

    /// This widget is being dragged this frame.
    ///
    /// Set the same frame a drag starts,
    /// but unset the frame a drag ends.
    pub dragged: Option<WidgetRect>,

    /// This widget was let go this frame,
    /// after having been dragged.
    ///
    /// The widget will not be found in [`Self::dragged`] this frame.
    pub drag_ended: Option<WidgetRect>,

    pub contains_pointer: IdMap<WidgetRect>,
    pub hovered: IdMap<WidgetRect>,
}

pub(crate) fn interact(
    prev_snapshot: &InteractionSnapshot,
    widgets: &WidgetRects,
    hits: &WidgetHits,
    input: &InputState,
    interaction: &mut InteractionState,
) -> InteractionSnapshot {
    if let Some(id) = interaction.click_id {
        if !widgets.by_id.contains_key(&id) {
            // The widget we were interested in clicking is gone.
            interaction.click_id = None;
        }
    }
    if let Some(id) = interaction.drag_id {
        if !widgets.by_id.contains_key(&id) {
            // The widget we were interested in dragging is gone.
            interaction.drag_id = None;
        }
    }

    let mut clicked = None;

    // Note: in the current code a press-release in the same frame is NOT considered a drag.
    for pointer_event in &input.pointer.pointer_events {
        match pointer_event {
            PointerEvent::Moved(_) => {}

            PointerEvent::Pressed { .. } => {
                // Maybe new click?
                if interaction.click_id.is_none() {
                    interaction.click_id = hits.click.map(|w| w.id);
                }

                // Maybe new drag?
                if interaction.drag_id.is_none() {
                    interaction.drag_id = hits.drag.map(|w| w.id);
                }
            }

            PointerEvent::Released { click, button: _ } => {
                if click.is_some() {
                    if let Some(widget) = interaction.click_id.and_then(|id| widgets.by_id.get(&id))
                    {
                        clicked = Some(*widget);
                    }
                }

                interaction.drag_id = None;
                interaction.click_id = None;
            }
        }
    }

    // Check if we're dragging something:
    let mut dragged = None;
    if let Some(widget) = interaction.drag_id.and_then(|id| widgets.by_id.get(&id)) {
        let is_dragged = if widget.sense.click && widget.sense.drag {
            // This widget is sensitive to both clicks and drags.
            // When the mouse first is pressed, it could be either,
            // so we postpone the decision until we know.
            input.pointer.is_decidedly_dragging()
        } else {
            // This widget is just sensitive to drags, so we can mark it as dragged right away:
            widget.sense.drag
        };

        if is_dragged {
            dragged = Some(*widget);
        }
    }

    let drag_changed = dragged != prev_snapshot.dragged;
    let drag_ended = drag_changed.then_some(prev_snapshot.dragged).flatten();
    let drag_started = drag_changed.then_some(dragged).flatten();

    let contains_pointer: IdMap<WidgetRect> = hits
        .contains_pointer
        .iter()
        .chain(&hits.top)
        .chain(&hits.click)
        .chain(&hits.drag)
        .map(|w| (w.id, *w))
        .collect();

    let hovered = if clicked.is_some() || dragged.is_some() {
        // If currently clicking or dragging, nother else is hovered.
        clicked.iter().chain(&dragged).map(|w| (w.id, *w)).collect()
    } else if hits.click.is_some() || hits.drag.is_some() {
        // We are hovering over an interactive widget or two. Just highlight these two.
        hits.click
            .iter()
            .chain(&hits.drag)
            .map(|w| (w.id, *w))
            .collect()
    } else {
        // Whatever is topmost is what we are hovering.
        // TODO: consider handle hovering over multiple top-most widgets?
        // TODO: allow hovering close widgets?
        hits.contains_pointer
            .last()
            .map(|w| (w.id, *w))
            .into_iter()
            .collect()
    };

    InteractionSnapshot {
        clicked,
        drag_started,
        dragged,
        drag_ended,
        contains_pointer,
        hovered,
    }
}
