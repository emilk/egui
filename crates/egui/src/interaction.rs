//! How mouse and touch interzcts with widgets.

use crate::*;

use self::{hit_test::WidgetHits, id::IdSet, input_state::PointerEvent, memory::InteractionState};

/// Calculated at the start of each frame
/// based on:
/// * Widget rects from precious frame
/// * Mouse/touch input
/// * Current [`InteractionState`].
#[derive(Clone, Default)]
pub struct InteractionSnapshot {
    /// The widget that got clicked this frame.
    pub clicked: Option<Id>,

    /// Drag started on this widget this frame.
    ///
    /// This will also be found in `dragged` this frame.
    pub drag_started: Option<Id>,

    /// This widget is being dragged this frame.
    ///
    /// Set the same frame a drag starts,
    /// but unset the frame a drag ends.
    ///
    /// NOTE: this may not have a corresponding [`WidgetRect`],
    /// if this for instance is a drag-and-drop widget which
    /// isn't painted whilst being dragged
    pub dragged: Option<Id>,

    /// This widget was let go this frame,
    /// after having been dragged.
    ///
    /// The widget will not be found in [`Self::dragged`] this frame.
    pub drag_stopped: Option<Id>,

    /// A small set of widgets (usually 0-1) that the pointer is hovering over.
    ///
    /// Show these widgets as highlighted, if they are interactive.
    ///
    /// While dragging or clicking something, nothing else is hovered.
    ///
    /// Use [`Self::contains_pointer`] to find a drop-zone for drag-and-drop.
    pub hovered: IdSet,

    /// All widgets that contain the pointer this frame,
    /// regardless if the user is currently clicking or dragging.
    ///
    /// This is usually a larger set than [`Self::hovered`],
    /// and can be used for e.g. drag-and-drop zones.
    pub contains_pointer: IdSet,
}

impl InteractionSnapshot {
    pub fn ui(&self, ui: &mut crate::Ui) {
        let Self {
            clicked,
            drag_started,
            dragged,
            drag_stopped,
            hovered,
            contains_pointer,
        } = self;

        fn id_ui<'a>(ui: &mut crate::Ui, widgets: impl IntoIterator<Item = &'a Id>) {
            for id in widgets {
                ui.label(id.short_debug_format());
            }
        }

        crate::Grid::new("interaction").show(ui, |ui| {
            ui.label("clicked");
            id_ui(ui, clicked);
            ui.end_row();

            ui.label("drag_started");
            id_ui(ui, drag_started);
            ui.end_row();

            ui.label("dragged");
            id_ui(ui, dragged);
            ui.end_row();

            ui.label("drag_stopped");
            id_ui(ui, drag_stopped);
            ui.end_row();

            ui.label("hovered");
            id_ui(ui, hovered);
            ui.end_row();

            ui.label("contains_pointer");
            id_ui(ui, contains_pointer);
            ui.end_row();
        });
    }
}

pub(crate) fn interact(
    prev_snapshot: &InteractionSnapshot,
    widgets: &WidgetRects,
    hits: &WidgetHits,
    input: &InputState,
    interaction: &mut InteractionState,
) -> InteractionSnapshot {
    crate::profile_function!();

    if let Some(id) = interaction.potential_click_id {
        if !widgets.contains(id) {
            // The widget we were interested in clicking is gone.
            interaction.potential_click_id = None;
        }
    }
    if let Some(id) = interaction.potential_drag_id {
        if !widgets.contains(id) {
            // The widget we were interested in dragging is gone.
            // This is fine! This could be drag-and-drop,
            // and the widget being dragged is now "in the air" and thus
            // not registered in the new frame.
        }
    }

    let mut clicked = None;
    let mut dragged = prev_snapshot.dragged;

    // Note: in the current code a press-release in the same frame is NOT considered a drag.
    for pointer_event in &input.pointer.pointer_events {
        match pointer_event {
            PointerEvent::Moved(_) => {}

            PointerEvent::Pressed { .. } => {
                // Maybe new click?
                if interaction.potential_click_id.is_none() {
                    interaction.potential_click_id = hits.click.map(|w| w.id);
                }

                // Maybe new drag?
                if interaction.potential_drag_id.is_none() {
                    interaction.potential_drag_id = hits.drag.map(|w| w.id);
                }
            }

            PointerEvent::Released { click, button: _ } => {
                if click.is_some() {
                    if let Some(widget) = interaction
                        .potential_click_id
                        .and_then(|id| widgets.get(id))
                    {
                        clicked = Some(widget.id);
                    }
                }

                interaction.potential_drag_id = None;
                interaction.potential_click_id = None;
                dragged = None;
            }
        }
    }

    if dragged.is_none() {
        // Check if we started dragging something new:
        if let Some(widget) = interaction.potential_drag_id.and_then(|id| widgets.get(id)) {
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
                dragged = Some(widget.id);
            }
        }
    }

    // ------------------------------------------------------------------------

    let drag_changed = dragged != prev_snapshot.dragged;
    let drag_stopped = drag_changed.then_some(prev_snapshot.dragged).flatten();
    let drag_started = drag_changed.then_some(dragged).flatten();

    // if let Some(drag_started) = drag_started {
    //     eprintln!(
    //         "Started dragging {} {:?}",
    //         drag_started.id.short_debug_format(),
    //         drag_started.rect
    //     );
    // }

    let contains_pointer: IdSet = hits
        .contains_pointer
        .iter()
        .chain(&hits.click)
        .chain(&hits.drag)
        .map(|w| w.id)
        .collect();

    let hovered = if clicked.is_some() || dragged.is_some() {
        // If currently clicking or dragging, nothing else is hovered.
        clicked.iter().chain(&dragged).copied().collect()
    } else if hits.click.is_some() || hits.drag.is_some() {
        // We are hovering over an interactive widget or two.
        hits.click.iter().chain(&hits.drag).map(|w| w.id).collect()
    } else {
        // Whatever is topmost is what we are hovering.
        // TODO: consider handle hovering over multiple top-most widgets?
        // TODO: allow hovering close widgets?
        hits.contains_pointer
            .last()
            .map(|w| w.id)
            .into_iter()
            .collect()
    };

    InteractionSnapshot {
        clicked,
        drag_started,
        dragged,
        drag_stopped,
        contains_pointer,
        hovered,
    }
}
