use crate::*;

/// Result of a hit-test against [`WidgetRects`].
///
/// Answers the question "what is under the mouse pointer?".
///
/// Note that this doesn't care if the mouse button is pressed or not,
/// or if we're currently already dragging something.
#[derive(Clone, Debug, Default)]
pub struct WidgetHits {
    /// All widgets that contains the pointer, back-to-front.
    ///
    /// i.e. both a Window and the button in it can contain the pointer.
    ///
    /// Some of these may be widgets in a layer below the top-most layer.
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
    pos: Pos2,
    search_radius: f32,
) -> WidgetHits {
    crate::profile_function!();

    let search_radius_sq = search_radius * search_radius;

    // First pass: find the few widgets close to the given position, sorted back-to-front.
    let close: Vec<WidgetRect> = layer_order
        .iter()
        .filter(|layer| layer.order.allow_interaction())
        .filter_map(|layer_id| widgets.by_layer.get(layer_id))
        .flatten()
        .filter(|w| w.interact_rect.distance_sq_to_pos(pos) <= search_radius_sq)
        .copied()
        .collect();

    let hits = hit_test_on_close(close, pos);

    if let Some(drag) = hits.drag {
        debug_assert!(drag.sense.drag);
    }
    if let Some(click) = hits.click {
        debug_assert!(click.sense.click);
    }

    hits
}

fn hit_test_on_close(mut close: Vec<WidgetRect>, pos: Pos2) -> WidgetHits {
    #![allow(clippy::collapsible_else_if)]

    // Only those widgets directly under the `pos`.
    let mut hits: Vec<WidgetRect> = close
        .iter()
        .filter(|widget| widget.interact_rect.contains(pos))
        .copied()
        .collect();

    {
        let top_hit = hits.last().copied();
        let top_layer = top_hit.map(|w| w.layer_id);

        if let Some(top_layer) = top_layer {
            // Ignore all layers not in the same layer as the top hit.
            close.retain(|w| w.layer_id == top_layer);
            hits.retain(|w| w.layer_id == top_layer);
        }
    }

    let hit_click = hits.iter().copied().filter(|w| w.sense.click).last();
    let hit_drag = hits.iter().copied().filter(|w| w.sense.drag).last();

    match (hit_click, hit_drag) {
        (None, None) => {
            // No direct hit on anything. Find the closest interactive widget.

            let closest = find_closest(
                close
                    .iter()
                    .copied()
                    .filter(|w| w.sense.click || w.sense.drag),
                pos,
            );

            if let Some(closest) = closest {
                WidgetHits {
                    contains_pointer: hits,
                    click: closest.sense.click.then_some(closest),
                    drag: closest.sense.drag.then_some(closest),
                }
            } else {
                // Found nothing
                WidgetHits {
                    contains_pointer: hits,
                    click: None,
                    drag: None,
                }
            }
        }

        (None, Some(hit_drag)) => {
            // We have a perfect hit on a drag, but not on click.

            // We have a direct hit on something that implements drag.
            // This could be a big background thing, like a `ScrollArea` background,
            // or a moveable window.
            // It could also be something small, like a slider, or panel resize handle.

            let closest_click = find_closest(close.iter().copied().filter(|w| w.sense.click), pos);
            if let Some(closest_click) = closest_click {
                if closest_click.sense.drag {
                    // We have something close that sense both clicks and drag.
                    // Should we use it over the direct drag-hit?
                    if hit_drag
                        .interact_rect
                        .contains_rect(closest_click.interact_rect)
                    {
                        // This is a smaller thing on a big background - help the user hit it,
                        // and ignore the big drag background.
                        WidgetHits {
                            contains_pointer: hits,
                            click: Some(closest_click),
                            drag: Some(closest_click),
                        }
                    } else {
                        // The drag wiudth is separate from the click wiudth,
                        // so return only the drag widget
                        WidgetHits {
                            contains_pointer: hits,
                            click: None,
                            drag: Some(hit_drag),
                        }
                    }
                } else {
                    // These is a close pure-click widget.
                    // However, we should be careful to only return two different widgets
                    // when it is absolutely not going to confuse the user.
                    if hit_drag
                        .interact_rect
                        .contains_rect(closest_click.interact_rect)
                    {
                        // The drag widget is a big background thing (scroll area),
                        // so returning a separate click widget should not be confusing
                        WidgetHits {
                            contains_pointer: hits,
                            click: Some(closest_click),
                            drag: Some(hit_drag),
                        }
                    } else {
                        // The two widgets are just two normal small widgets close to each other.
                        // Highlighting both would be very confusing.
                        WidgetHits {
                            contains_pointer: hits,
                            click: None,
                            drag: Some(hit_drag),
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
                        .filter(|w| w.sense.drag && w.id != hit_drag.id),
                    pos,
                );

                if let Some(closest_drag) = closest_drag {
                    if hit_drag
                        .interact_rect
                        .contains_rect(closest_drag.interact_rect)
                    {
                        // `hit_drag` is a big background thing and `closest_drag` is something small on top of it.
                        // Be helpful and return the small things:
                        return WidgetHits {
                            contains_pointer: hits,
                            click: None,
                            drag: Some(closest_drag),
                        };
                    }
                }

                WidgetHits {
                    contains_pointer: hits,
                    click: None,
                    drag: Some(hit_drag),
                }
            }
        }

        (Some(hit_click), None) => {
            // We have a perfect hit on a click-widget, but not on a drag-widget.

            WidgetHits {
                contains_pointer: hits,
                click: Some(hit_click),
                drag: None, // TODO: we should maybe look for close drag widgets?
            }
        }

        (Some(hit_click), Some(hit_drag)) => {
            // We have a perfect hit on both click and drag. Which is the topmost?
            let click_idx = hits.iter().position(|w| *w == hit_click).unwrap();
            let drag_idx = hits.iter().position(|w| *w == hit_drag).unwrap();

            let click_is_on_top_of_drag = drag_idx < click_idx;
            if click_is_on_top_of_drag {
                if hit_click.sense.drag {
                    // The top thing senses both clicks and drags.
                    WidgetHits {
                        contains_pointer: hits,
                        click: Some(hit_click),
                        drag: Some(hit_click),
                    }
                } else {
                    // They are interested in different things.
                    WidgetHits {
                        contains_pointer: hits,
                        click: Some(hit_click),
                        drag: Some(hit_drag),
                    }
                }
            } else {
                if hit_drag.sense.click {
                    // The top thing senses both clicks and drags.
                    WidgetHits {
                        contains_pointer: hits,
                        click: Some(hit_drag),
                        drag: Some(hit_drag),
                    }
                } else {
                    // The top things senses only drags
                    WidgetHits {
                        contains_pointer: hits,
                        click: None,
                        drag: Some(hit_drag),
                    }
                }
            }
        }
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

#[cfg(test)]
mod tests {
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
        let hits = hit_test_on_close(widgets.clone(), pos2(15.0, 15.0));
        assert_eq!(hits.click.unwrap().id, Id::new("click"));
        assert_eq!(hits.drag.unwrap().id, Id::new("bg-area"));

        // Close hit:
        let hits = hit_test_on_close(widgets.clone(), pos2(5.0, 5.0));
        assert_eq!(hits.click.unwrap().id, Id::new("click"));
        assert_eq!(hits.drag.unwrap().id, Id::new("bg-area"));

        // Perfect hit:
        let hits = hit_test_on_close(widgets.clone(), pos2(105.0, 15.0));
        assert_eq!(hits.click.unwrap().id, Id::new("click-and-drag"));
        assert_eq!(hits.drag.unwrap().id, Id::new("click-and-drag"));

        // Close hit - should still ignore the drag-background so as not to confuse the userr:
        let hits = hit_test_on_close(widgets.clone(), pos2(105.0, 5.0));
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
            eprintln!("Widget {i}: {:?}", w.id);
        }

        // In the middle of the bg-left-label:
        let hits = hit_test_on_close(widgets.clone(), pos2(25.0, 50.0));
        assert_eq!(hits.click.unwrap().id, Id::new("bg-left-label"));
        assert_eq!(hits.drag.unwrap().id, Id::new("bg-left-label"));

        // On both the left click-and-drag and thin handle, but the thin handle is on top and should win:
        let hits = hit_test_on_close(widgets.clone(), pos2(35.0, 50.0));
        assert_eq!(hits.click, None);
        assert_eq!(hits.drag.unwrap().id, Id::new("thin-drag-handle"));

        // Only on the thin-drag-handle:
        let hits = hit_test_on_close(widgets.clone(), pos2(50.0, 50.0));
        assert_eq!(hits.click, None);
        assert_eq!(hits.drag.unwrap().id, Id::new("thin-drag-handle"));

        // On both the thin handle and right label. The label is on top and should win
        let hits = hit_test_on_close(widgets.clone(), pos2(65.0, 50.0));
        assert_eq!(hits.click.unwrap().id, Id::new("fg-right-label"));
        assert_eq!(hits.drag.unwrap().id, Id::new("fg-right-label"));
    }
}
