use ahash::HashMap;

use emath::TSTransform;

use crate::{LayerId, Pos2, Sense, WidgetRect, WidgetRects, ahash, emath, id::IdSet};

/// Result of a hit-test against [`WidgetRects`].
///
/// Answers the question "what is under the mouse pointer?".
///
/// Note that this doesn't care if the mouse button is pressed or not,
/// or if we're currently already dragging something.
#[derive(Clone, Debug, Default)]
pub struct WidgetHits {
    /// All widgets close to the pointer, back-to-front.
    ///
    /// This is a superset of all other widgets in this struct.
    pub close: Vec<WidgetRect>,

    /// All widgets that contains the pointer, back-to-front.
    ///
    /// i.e. both a Window and the Button in it can contain the pointer.
    ///
    /// Some of these may be widgets in a layer below the top-most layer.
    ///
    /// This will be used for hovering.
    pub contains_pointer: Vec<WidgetRect>,

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
/// none which is closer than `search_radius`.
pub fn hit_test(
    widgets: &WidgetRects,
    layer_order: &[LayerId],
    layer_to_global: &HashMap<LayerId, TSTransform>,
    pos: Pos2,
    search_radius: f32,
) -> WidgetHits {
    profiling::function_scope!();

    let search_radius_sq = search_radius * search_radius;

    // Transform the position into the local coordinate space of each layer:
    let pos_in_layers: HashMap<LayerId, Pos2> = layer_to_global
        .iter()
        .map(|(layer_id, to_global)| (*layer_id, to_global.inverse() * pos))
        .collect();

    let mut closest_dist_sq = f32::INFINITY;
    let mut closest_hit = None;

    // First pass: find the few widgets close to the given position, sorted back-to-front.
    let mut close: Vec<WidgetRect> = layer_order
        .iter()
        .filter(|layer| layer.order.allow_interaction())
        .flat_map(|&layer_id| widgets.get_layer(layer_id))
        .filter(|&w| {
            if w.interact_rect.is_negative() || w.interact_rect.any_nan() {
                return false;
            }

            let pos_in_layer = pos_in_layers.get(&w.layer_id).copied().unwrap_or(pos);
            // TODO(emilk): we should probably do the distance testing in global space instead
            let dist_sq = w.interact_rect.distance_sq_to_pos(pos_in_layer);

            // In tie, pick last = topmost.
            if dist_sq <= closest_dist_sq {
                closest_dist_sq = dist_sq;
                closest_hit = Some(w);
            }

            dist_sq <= search_radius_sq
        })
        .copied()
        .collect();

    // Transform to global coordinates:
    for hit in &mut close {
        if let Some(to_global) = layer_to_global.get(&hit.layer_id).copied() {
            *hit = hit.transform(to_global);
        }
    }

    close.retain(|rect| !rect.interact_rect.any_nan()); // Protect against bad input and transforms

    // When using layer transforms it is common to stack layers close to each other.
    // For instance, you may have a resize-separator on a panel, with two
    // transform-layers on either side.
    // The resize-separator is technically in a layer _behind_ the transform-layers,
    // but the user doesn't perceive it as such.
    // So how do we handle this case?
    //
    // If we just allow interactions with ALL close widgets,
    // then we might accidentally allow clicks through windows and other bad stuff.
    //
    // Let's try this:
    // * Set up a hit-area (based on search_radius)
    // * Iterate over all hits top-to-bottom
    //   * Stop if any hit covers the whole hit-area, otherwise keep going
    //   * Collect the layers ids in a set
    // * Remove all widgets not in the above layer set
    //
    // This will most often result in only one layer,
    // but if the pointer is at the edge of a layer, we might include widgets in
    // a layer behind it.

    let mut included_layers: ahash::HashSet<LayerId> = Default::default();
    for hit in close.iter().rev() {
        included_layers.insert(hit.layer_id);
        let hit_covers_search_area = contains_circle(hit.interact_rect, pos, search_radius);
        if hit_covers_search_area {
            break; // nothing behind this layer could ever be interacted with
        }
    }

    close.retain(|hit| included_layers.contains(&hit.layer_id));

    // If a widget is disabled, treat it as if it isn't sensing anything.
    // This simplifies the code in `hit_test_on_close` so it doesn't have to check
    // the `enabled` flag everywhere:
    for w in &mut close {
        if !w.enabled {
            w.sense -= Sense::CLICK;
            w.sense -= Sense::DRAG;
        }
    }

    // Find widgets which are hidden behind another widget and discard them.
    // This is the case when a widget fully contains another widget and is on a different layer.
    // It prevents "hovering through" widgets when there is a clickable widget behind.

    let mut hidden = IdSet::default();
    for (i, current) in close.iter().enumerate().rev() {
        for next in &close[i + 1..] {
            if next.interact_rect.contains_rect(current.interact_rect)
                && current.layer_id != next.layer_id
            {
                hidden.insert(current.id);
            }
        }
    }

    close.retain(|c| !hidden.contains(&c.id));

    let mut hits = hit_test_on_close(&close, pos);

    hits.contains_pointer = close
        .iter()
        .filter(|widget| widget.interact_rect.contains(pos))
        .copied()
        .collect();

    hits.close = close;

    {
        // Undo the to_global-transform we applied earlier,
        // go back to local layer-coordinates:

        let restore_widget_rect = |w: &mut WidgetRect| {
            *w = widgets.get(w.id).copied().unwrap_or(*w);
        };

        for wr in &mut hits.close {
            restore_widget_rect(wr);
        }
        for wr in &mut hits.contains_pointer {
            restore_widget_rect(wr);
        }
        if let Some(wr) = &mut hits.drag {
            debug_assert!(
                wr.sense.senses_drag(),
                "We should only return drag hits if they sense drag"
            );
            restore_widget_rect(wr);
        }
        if let Some(wr) = &mut hits.click {
            debug_assert!(
                wr.sense.senses_click(),
                "We should only return click hits if they sense click"
            );
            restore_widget_rect(wr);
        }
    }

    hits
}

/// Returns true if the rectangle contains the whole circle.
fn contains_circle(interact_rect: emath::Rect, pos: Pos2, radius: f32) -> bool {
    interact_rect.shrink(radius).contains(pos)
}

fn hit_test_on_close(close: &[WidgetRect], pos: Pos2) -> WidgetHits {
    #![expect(clippy::collapsible_else_if)]

    // First find the best direct hits:
    let hit_click = find_closest_within(
        close.iter().copied().filter(|w| w.sense.senses_click()),
        pos,
        0.0,
    );
    let hit_drag = find_closest_within(
        close.iter().copied().filter(|w| w.sense.senses_drag()),
        pos,
        0.0,
    );

    match (hit_click, hit_drag) {
        (None, None) => {
            // No direct hit on anything. Find the closest interactive widget.

            let closest = find_closest(
                close
                    .iter()
                    .copied()
                    .filter(|w| w.sense.senses_click() || w.sense.senses_drag()),
                pos,
            );

            if let Some(closest) = closest {
                WidgetHits {
                    click: closest.sense.senses_click().then_some(closest),
                    drag: closest.sense.senses_drag().then_some(closest),
                    ..Default::default()
                }
            } else {
                // Found nothing
                WidgetHits {
                    click: None,
                    drag: None,
                    ..Default::default()
                }
            }
        }

        (None, Some(hit_drag)) => {
            // We have a perfect hit on a drag, but not on click.

            // We have a direct hit on something that implements drag.
            // This could be a big background thing, like a `ScrollArea` background,
            // or a moveable window.
            // It could also be something small, like a slider, or panel resize handle.

            let closest_click = find_closest(
                close.iter().copied().filter(|w| w.sense.senses_click()),
                pos,
            );
            if let Some(closest_click) = closest_click {
                if closest_click.sense.senses_drag() {
                    // We have something close that sense both clicks and drag.
                    // Should we use it over the direct drag-hit?
                    if hit_drag
                        .interact_rect
                        .contains_rect(closest_click.interact_rect)
                    {
                        // This is a smaller thing on a big background - help the user hit it,
                        // and ignore the big drag background.
                        WidgetHits {
                            click: Some(closest_click),
                            drag: Some(closest_click),
                            ..Default::default()
                        }
                    } else {
                        // The drag-widget is separate from the click-widget,
                        // so return only the drag-widget
                        WidgetHits {
                            click: None,
                            drag: Some(hit_drag),
                            ..Default::default()
                        }
                    }
                } else {
                    // This is a close pure-click widget.
                    // However, we should be careful to only return two different widgets
                    // when it is absolutely not going to confuse the user.
                    if hit_drag
                        .interact_rect
                        .contains_rect(closest_click.interact_rect)
                    {
                        // The drag widget is a big background thing (scroll area),
                        // so returning a separate click widget should not be confusing
                        WidgetHits {
                            click: Some(closest_click),
                            drag: Some(hit_drag),
                            ..Default::default()
                        }
                    } else {
                        // The two widgets are just two normal small widgets close to each other.
                        // Highlighting both would be very confusing.
                        WidgetHits {
                            click: None,
                            drag: Some(hit_drag),
                            ..Default::default()
                        }
                    }
                }
            } else {
                // No close clicks.
                // Maybe there is a close drag widget, that is a smaller
                // widget floating on top of a big background?
                // If so, it would be nice to help the user click that.
                let closest_drag = find_closest(
                    close
                        .iter()
                        .copied()
                        .filter(|w| w.sense.senses_drag() && w.id != hit_drag.id),
                    pos,
                );

                if let Some(closest_drag) = closest_drag
                    && hit_drag
                        .interact_rect
                        .contains_rect(closest_drag.interact_rect)
                {
                    // `hit_drag` is a big background thing and `closest_drag` is something small on top of it.
                    // Be helpful and return the small things:
                    return WidgetHits {
                        click: None,
                        drag: Some(closest_drag),
                        ..Default::default()
                    };
                }

                WidgetHits {
                    click: None,
                    drag: Some(hit_drag),
                    ..Default::default()
                }
            }
        }

        (Some(hit_click), None) => {
            // We have a perfect hit on a click-widget, but not on a drag-widget.
            //
            // Note that we don't look for a close drag widget in this case,
            // because I can't think of a case where that would be helpful.
            // This is in contrast with the opposite case,
            // where when hovering directly over a drag-widget (like a big ScrollArea),
            // we look for close click-widgets (e.g. buttons).
            // This is because big background drag-widgets (ScrollArea, Window) are common,
            // but big clickable things aren't.
            // Even if they were, I think it would be confusing for a user if clicking
            // a drag-only widget would click something _behind_ it.

            WidgetHits {
                click: Some(hit_click),
                drag: None,
                ..Default::default()
            }
        }

        (Some(hit_click), Some(hit_drag)) => {
            // We have a perfect hit on both click and drag. Which is the topmost?
            #[expect(clippy::unwrap_used)]
            let click_idx = close.iter().position(|w| *w == hit_click).unwrap();

            #[expect(clippy::unwrap_used)]
            let drag_idx = close.iter().position(|w| *w == hit_drag).unwrap();

            let click_is_on_top_of_drag = drag_idx < click_idx;
            if click_is_on_top_of_drag {
                if hit_click.sense.senses_drag() {
                    // The top thing senses both clicks and drags.
                    WidgetHits {
                        click: Some(hit_click),
                        drag: Some(hit_click),
                        ..Default::default()
                    }
                } else {
                    // They are interested in different things,
                    // and click is on top. Report both hits,
                    // e.g. the top Button and the ScrollArea behind it.
                    WidgetHits {
                        click: Some(hit_click),
                        drag: Some(hit_drag),
                        ..Default::default()
                    }
                }
            } else {
                if hit_drag.sense.senses_click() {
                    // The top thing senses both clicks and drags.
                    WidgetHits {
                        click: Some(hit_drag),
                        drag: Some(hit_drag),
                        ..Default::default()
                    }
                } else {
                    // The top things senses only drags,
                    // so we ignore the click-widget, because it would be confusing
                    // if clicking a drag-widget would actually click something else below it.
                    WidgetHits {
                        click: None,
                        drag: Some(hit_drag),
                        ..Default::default()
                    }
                }
            }
        }
    }
}

fn find_closest(widgets: impl Iterator<Item = WidgetRect>, pos: Pos2) -> Option<WidgetRect> {
    find_closest_within(widgets, pos, f32::INFINITY)
}

fn find_closest_within(
    widgets: impl Iterator<Item = WidgetRect>,
    pos: Pos2,
    max_dist: f32,
) -> Option<WidgetRect> {
    let mut closest: Option<WidgetRect> = None;
    let mut closest_dist_sq = max_dist * max_dist;
    for widget in widgets {
        if widget.interact_rect.is_negative() {
            continue;
        }

        let dist_sq = widget.interact_rect.distance_sq_to_pos(pos);

        // In case of a tie, take the last one = the one on top.
        if dist_sq <= closest_dist_sq {
            closest_dist_sq = dist_sq;
            closest = Some(widget);
        }
    }

    closest
}

#[cfg(test)]
mod tests {
    #![expect(clippy::print_stdout)]

    use emath::{Rect, pos2, vec2};

    use crate::{Id, Sense};

    use super::*;

    fn wr(id: Id, sense: Sense, rect: Rect) -> WidgetRect {
        WidgetRect {
            id,
            layer_id: LayerId::background(),
            rect,
            interact_rect: rect,
            sense,
            enabled: true,
        }
    }

    #[test]
    fn buttons_on_window() {
        let widgets = vec![
            wr(
                Id::new("bg-area"),
                Sense::drag(),
                Rect::from_min_size(pos2(0.0, 0.0), vec2(100.0, 100.0)),
            ),
            wr(
                Id::new("click"),
                Sense::click(),
                Rect::from_min_size(pos2(10.0, 10.0), vec2(10.0, 10.0)),
            ),
            wr(
                Id::new("click-and-drag"),
                Sense::click_and_drag(),
                Rect::from_min_size(pos2(100.0, 10.0), vec2(10.0, 10.0)),
            ),
        ];

        // Perfect hit:
        let hits = hit_test_on_close(&widgets, pos2(15.0, 15.0));
        assert_eq!(hits.click.unwrap().id, Id::new("click"));
        assert_eq!(hits.drag.unwrap().id, Id::new("bg-area"));

        // Close hit:
        let hits = hit_test_on_close(&widgets, pos2(5.0, 5.0));
        assert_eq!(hits.click.unwrap().id, Id::new("click"));
        assert_eq!(hits.drag.unwrap().id, Id::new("bg-area"));

        // Perfect hit:
        let hits = hit_test_on_close(&widgets, pos2(105.0, 15.0));
        assert_eq!(hits.click.unwrap().id, Id::new("click-and-drag"));
        assert_eq!(hits.drag.unwrap().id, Id::new("click-and-drag"));

        // Close hit - should still ignore the drag-background so as not to confuse the user:
        let hits = hit_test_on_close(&widgets, pos2(105.0, 5.0));
        assert_eq!(hits.click.unwrap().id, Id::new("click-and-drag"));
        assert_eq!(hits.drag.unwrap().id, Id::new("click-and-drag"));
    }

    #[test]
    fn thin_resize_handle_next_to_label() {
        let widgets = vec![
            wr(
                Id::new("bg-area"),
                Sense::drag(),
                Rect::from_min_size(pos2(0.0, 0.0), vec2(100.0, 100.0)),
            ),
            wr(
                Id::new("bg-left-label"),
                Sense::click_and_drag(),
                Rect::from_min_size(pos2(0.0, 0.0), vec2(40.0, 100.0)),
            ),
            wr(
                Id::new("thin-drag-handle"),
                Sense::drag(),
                Rect::from_min_size(pos2(30.0, 0.0), vec2(70.0, 100.0)),
            ),
            wr(
                Id::new("fg-right-label"),
                Sense::click_and_drag(),
                Rect::from_min_size(pos2(60.0, 0.0), vec2(50.0, 100.0)),
            ),
        ];

        for (i, w) in widgets.iter().enumerate() {
            println!("Widget {i}: {:?}", w.id);
        }

        // In the middle of the bg-left-label:
        let hits = hit_test_on_close(&widgets, pos2(25.0, 50.0));
        assert_eq!(hits.click.unwrap().id, Id::new("bg-left-label"));
        assert_eq!(hits.drag.unwrap().id, Id::new("bg-left-label"));

        // On both the left click-and-drag and thin handle, but the thin handle is on top and should win:
        let hits = hit_test_on_close(&widgets, pos2(35.0, 50.0));
        assert_eq!(hits.click, None);
        assert_eq!(hits.drag.unwrap().id, Id::new("thin-drag-handle"));

        // Only on the thin-drag-handle:
        let hits = hit_test_on_close(&widgets, pos2(50.0, 50.0));
        assert_eq!(hits.click, None);
        assert_eq!(hits.drag.unwrap().id, Id::new("thin-drag-handle"));

        // On both the thin handle and right label. The label is on top and should win
        let hits = hit_test_on_close(&widgets, pos2(65.0, 50.0));
        assert_eq!(hits.click.unwrap().id, Id::new("fg-right-label"));
        assert_eq!(hits.drag.unwrap().id, Id::new("fg-right-label"));
    }
}
