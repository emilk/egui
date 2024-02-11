use crate::*;

/// Result of a hit-test against [`WidgetRects`].
///
/// Answers the question "what is under the mouse pointer?".
///
/// Note that this doesn't care if the mouse button is pressed or not,
/// or if we're currently already dragging something.
///
/// For that you need the [`crate::InteractionState`].
#[derive(Clone, Debug, Default)]
pub struct WidgetHits {
    /// All widgets that contains the pointer, back-to-front.
    ///
    /// i.e. both a Window and the button in it can contain the pointer.
    ///
    /// Some of these may be widgets in a layer below the top-most layer.
    pub contains_pointer: Vec<WidgetRect>,

    /// The topmost widget under the pointer, interactive or not.
    ///
    /// Used for nothing right now.
    pub top: Option<WidgetRect>,

    /// If the user would start a clicking now, this is what would be clicked.
    ///
    /// This is the top one under the pointer, or closest one of the top-most.
    pub click: Option<WidgetRect>,

    /// If the user would start a dragging now, this is what would be dragged.
    ///
    /// This is the top one under the pointer, or closest one of the top-most.
    pub drag: Option<WidgetRect>,

    /// The closest interactive widget under the pointer.
    ///
    /// This is either the same as [`Self::click`] or [`Self::drag`], or both.
    pub closest_interactive: Option<WidgetRect>,
}

/// Find the top or closest widgets to the given position,
/// none which is closer than `search_radius`.
pub fn hit_test(
    widgets: &WidgetRects,
    layer_order: &[LayerId],
    pos: Pos2,
    search_radius: f32,
) -> WidgetHits {
    crate::profile_function!();

    let search_radius_sq = search_radius * search_radius;

    // First pass: find the few widgets close to the given position, sorted back-to-front.
    let mut close: Vec<WidgetRect> = layer_order
        .iter()
        .filter(|layer| layer.order.allow_interaction())
        .filter_map(|layer_id| widgets.by_layer.get(layer_id))
        .flatten()
        .filter(|w| w.interact_rect.distance_sq_to_pos(pos) <= search_radius_sq)
        .copied()
        .collect();

    // Only those widgets directly under the `pos`.
    let mut hits: Vec<WidgetRect> = close
        .iter()
        .filter(|widget| widget.interact_rect.contains(pos))
        .copied()
        .collect();

    let top_hit = hits.last().copied();
    let top_layer = top_hit.map(|w| w.layer_id);

    if let Some(top_layer) = top_layer {
        // Ignore all layers not in the same layer as the top hit.
        close.retain(|w| w.layer_id == top_layer);
        hits.retain(|w| w.layer_id == top_layer);
    }

    let hit_click = hits.iter().copied().filter(|w| w.sense.click).last();
    let hit_drag = hits.iter().copied().filter(|w| w.sense.drag).last();

    let closest = find_closest(close.iter().copied(), pos);
    let closest_click = find_closest(close.iter().copied().filter(|w| w.sense.click), pos);
    let closest_drag = find_closest(close.iter().copied().filter(|w| w.sense.drag), pos);

    let top = top_hit.or(closest);
    let mut click = hit_click.or(closest_click);
    let mut drag = hit_drag.or(closest_drag);

    if let (Some(click), Some(drag)) = (&mut click, &mut drag) {
        // If one of the widgets is interested in both click and drags, let it win.
        // Otherwise we end up in weird situations where both widgets respond to hover,
        // but one of the widgets only responds to _one_ of the events.

        if click.sense.click && click.sense.drag {
            *drag = *click;
        } else if drag.sense.click && drag.sense.drag {
            *click = *drag;
        }
    }

    let closest_interactive = match (click, drag) {
        (Some(click), Some(drag)) => {
            if click.interact_rect.distance_sq_to_pos(pos)
                < drag.interact_rect.distance_sq_to_pos(pos)
            {
                Some(click)
            } else {
                Some(drag)
            }
        }
        (Some(click), None) => Some(click),
        (None, Some(drag)) => Some(drag),
        (None, None) => None,
    };

    WidgetHits {
        contains_pointer: hits,
        top,
        click,
        drag,
        closest_interactive,
    }
}

fn find_closest(widgets: impl Iterator<Item = WidgetRect>, pos: Pos2) -> Option<WidgetRect> {
    let mut closest = None;
    let mut closest_dist_sq = f32::INFINITY;
    for widget in widgets {
        let dist_sq = widget.interact_rect.distance_sq_to_pos(pos);

        // In case of a tie, take the last one = the one on top.
        if dist_sq <= closest_dist_sq {
            closest_dist_sq = dist_sq;
            closest = Some(widget);
        }
    }

    closest
}
