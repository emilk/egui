use crate::*;

/// Result of a hit-test agains [`WidgetRects`].
///
/// Answers the question "what is under the mouse pointer?".
///
/// Note that this doesn't care if the mouse button is pressed or not,
/// or if we're currently already dragging something.
///
/// For that you need the `InteractionState`.
#[derive(Clone, Debug, Default)]
pub struct WidgetHits {
    /// All widgets that contains the pointer.
    ///
    /// i.e. both a Window and the button in it can ontain the pointer.
    ///
    /// Show tooltips for all of these.
    /// Why? So you can do `ui.scope(|ui| …).response.on_hover_text(…)`
    /// and get a tooltip for the whole ui, even if individual things
    /// in the ui also had a tooltip.
    pub contains_pointer: IdMap<WidgetRect>,

    /// The topmost widget under the pointer, interactive or not.
    ///
    /// Used for nothing?
    pub top: Option<WidgetRect>,

    /// If the user would start a clicking now, this is what would be clicked.
    ///
    /// This is the top one under the pointer, or closest one of the top-most.
    pub click: Option<WidgetRect>,

    /// If the user would start a dragging now, this is what would be dragged.
    ///
    /// This is the top one under the pointer, or closest one of the top-most.
    pub drag: Option<WidgetRect>,
}

/// Find the top or closest widgets to the given position,
/// None which is closer than `search_radius`.
pub fn hit_test(
    widgets: &WidgetRects,
    layer_order: &[LayerId],
    pos: Pos2,
    search_radius: f32,
) -> WidgetHits {
    crate::profile_function!();

    let hit_rect = Rect::from_center_size(pos, Vec2::splat(2.0 * search_radius));

    // The few widgets close to the given position, sorted back-to-front.
    let close: Vec<WidgetRect> = layer_order
        .iter()
        .filter_map(|layer_id| widgets.by_layer.get(layer_id))
        .flatten()
        .filter(|widget| widget.interact_rect.intersects(hit_rect))
        .filter(|w| w.interact_rect.distance_to_pos(pos) <= search_radius)
        .copied()
        .collect();

    // Only those widgets directly under the `pos`.
    let hits: Vec<WidgetRect> = close
        .iter()
        .filter(|widget| widget.interact_rect.contains(pos))
        .copied()
        .collect();

    let hit = hits.last().copied();
    let hit_click = hits.iter().copied().filter(|w| w.sense.click).last();
    let hit_drag = hits.iter().copied().filter(|w| w.sense.drag).last();

    let closest = find_closest(close.iter().copied(), pos);
    let closest_click = find_closest(close.iter().copied().filter(|w| w.sense.click), pos);
    let closest_drag = find_closest(close.iter().copied().filter(|w| w.sense.drag), pos);

    let top = hit.or(closest);
    let click = hit_click.or(closest_click);
    let drag = hit_drag.or(closest_drag);

    // Which widgets which will have tooltips:
    let contains_pointer = hits.into_iter().map(|w| (w.id, w)).collect();

    WidgetHits {
        contains_pointer,
        top,
        click,
        drag,
    }
}

fn find_closest(widgets: impl Iterator<Item = WidgetRect>, pos: Pos2) -> Option<WidgetRect> {
    let mut closest = None;
    let mut cloest_dist_sq = f32::INFINITY;
    for widget in widgets {
        let dist_sq = widget.interact_rect.distance_sq_to_pos(pos);

        // In case of a tie, take the last one = the one on top.
        if dist_sq <= cloest_dist_sq {
            cloest_dist_sq = dist_sq;
            closest = Some(widget);
        }
    }

    closest
}
