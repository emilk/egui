//! Scroll an enclosing [`crate::ScrollArea`] while a text selection is dragged past its edge
//! so that the selection can grow beyond what is visible.
//!
//! A virtualised list ([`crate::ScrollArea::show_rows`], [`crate::ScrollArea::show_viewport`])
//! is not scrolled: what scrolls out of view would no longer be laid out, so it could neither
//! be selected nor copied.

use emath::{Pos2, Rangef, Rect, Vec2};

use crate::{Id, Ui, UiKind, pass_state::ScrollAreaViewport};

/// How fast to scroll, in points per second, with the pointer right at the edge
const BASE_SPEED: f32 = 240.0;

/// How much faster to scroll, in points per second, for each point the pointer is past the edge
const SPEED_PER_POINT: f32 = 36.0;

/// The fastest we scroll, in points per second
const MAX_SPEED: f32 = 2400.0;

/// Scroll the [`crate::ScrollArea`] enclosing `ui` when a text selection is being
/// dragged with the pointer outside the visible area.
///
/// Call this every pass while dragging, from the widget the drag started in
/// (or the one holding the moving end, if the former is in no scroll area
/// that can scroll towards the pointer).
/// `pointer_pos` is in global coordinates, `selectable` in those of the `ui`'s layer.
/// `selectable` is the area the selection can extend over (the text edit itself,
/// or everything for labels), so that we only scroll in a direction where there is more
/// to select. Along each axis, the nearest enclosing scroll area that scrolls along it,
/// and can still move that way, is scrolled.
///
/// The scroll areas in `left_alone`, and those around them, are not scrolled.
///
/// With `apply` false, only decide, without scrolling.
///
/// Returns the viewports being scrolled, if we asked for a scroll.
/// Nothing happens when `ui` is not inside a scroll area.
pub(crate) fn scroll_while_dragging(
    ui: &Ui,
    pointer_pos: Pos2,
    selectable: Rect,
    left_alone: &[Id],
    apply: bool,
) -> Option<AutoScroll> {
    let ctx = ui.ctx();
    let global_from_ui = ctx
        .layer_transform_to_global(ui.layer_id())
        .unwrap_or_default();
    let dt = ui.input(|i| i.stable_dt).min(0.1); // don't jump after a stall

    let mut deltas: Vec<(Id, Vec2)> = Vec::new();
    let mut auto_scroll = AutoScroll { areas: [None; 2] };
    for d in 0..2 {
        // The viewports of the enclosing scroll areas that scroll along this axis, nearest first,
        // not `ui.clip_rect()`, which may be narrower (like a table cell that clips its contents).
        for (id, viewport) in enclosing_scroll_area_viewports(ui, d) {
            if left_alone.contains(&id) {
                break; // so are those further out, which enclose it
            }
            if viewport.content_may_be_partial {
                // What scrolls out of view may no longer be laid out, so it could neither be
                // selected nor copied. Nor scroll the areas around it: that would scroll it too.
                break;
            }
            if !viewport.rect.is_positive() {
                continue; // scrolled (almost) out of view itself
            }

            // Compare in the scroll area's layer, so that a transformed layer in between
            // (like a zoomed `Scene`) does not distort distances, and the delta is in its units:
            let area_from_global = ctx
                .layer_transform_from_global(viewport.layer_id)
                .unwrap_or_default();
            let selectable = if selectable == Rect::EVERYTHING {
                selectable
            } else {
                area_from_global * (global_from_ui * selectable)
            };

            let velocity =
                auto_scroll_velocity(area_from_global * pointer_pos, viewport.rect, selectable)[d];
            if velocity == 0.0 {
                break; // the pointer is inside this one along this axis
            }
            if !viewport.can_scroll(d, 0.0 < velocity) {
                continue; // at its end, so leave it to those further out
            }
            let global_viewport = area_from_global.inverse() * viewport.rect;

            // A positive scroll delta moves the content down/right, i.e. scrolls towards the start:
            let mut delta = Vec2::ZERO;
            delta[d] = -velocity * dt;
            deltas.push((id, delta));
            auto_scroll.areas[d] = Some(global_viewport);
            break;
        }
    }
    if deltas.is_empty() {
        return None;
    }
    if !apply {
        return Some(auto_scroll);
    }

    // Each delta goes to its own scroll area, which requests a repaint when it applies it,
    // so we keep scrolling while the pointer is held still.
    ctx.pass_state_mut(|state| {
        for (id, delta) in deltas {
            *state.scroll_area_deltas.entry(id).or_default() += delta;
        }
    });
    Some(auto_scroll)
}

/// The viewports being scrolled while a text selection is dragged past their edges.
#[derive(Clone, Copy, Debug)]
pub(crate) struct AutoScroll {
    /// Along each axis, the viewport of the scroll area being scrolled, in global coordinates.
    areas: [Option<Rect>; 2],
}

impl AutoScroll {
    /// Where to act as if the pointer were for hit-testing: just inside the viewports
    /// along the axes being scrolled, also when they are tiny.
    pub(crate) fn clamp(&self, mut pointer_pos: Pos2) -> Pos2 {
        for d in 0..2 {
            if let Some(area) = self.areas[d] {
                let margin = (0.5 * area.size()[d]).min(1.0);
                pointer_pos[d] = pointer_pos[d].clamp(area.min[d] + margin, area.max[d] - margin);
            }
        }
        pointer_pos
    }

    /// Drop the axes along which the pointer at `pointer_pos` is back inside.
    pub(crate) fn still_applies(mut self, pointer_pos: Pos2) -> Option<Self> {
        for d in 0..2 {
            if let Some(area) = self.areas[d] {
                let inside = area.min[d] <= pointer_pos[d] && pointer_pos[d] <= area.max[d];
                if inside {
                    self.areas[d] = None;
                }
            }
        }
        self.areas.iter().any(Option::is_some).then_some(self)
    }
}

/// The [`Ui::unique_id`]s of the contents of the [`crate::ScrollArea`]s enclosing `ui`.
pub(crate) fn enclosing_scroll_areas(ui: &Ui) -> Vec<Id> {
    ui.stack()
        .iter()
        .filter(|frame| frame.kind() == Some(UiKind::ScrollArea))
        .map(|frame| frame.unique_id)
        .collect()
}

/// The [`Ui::unique_id`] of the contents of the nearest [`crate::ScrollArea`] enclosing `ui`
/// that scrolls vertically, if any.
pub(crate) fn nearest_vertical_scroll_area(ui: &Ui) -> Option<Id> {
    ui.ctx().pass_state(|state| {
        ui.stack()
            .iter()
            .filter(|frame| frame.kind() == Some(UiKind::ScrollArea))
            .map(|frame| frame.unique_id)
            .find(|id| {
                state
                    .scroll_area_viewports
                    .get(id)
                    .is_some_and(|viewport| viewport.scrolls[1])
            })
    })
}

/// The visible parts of the [`crate::ScrollArea`]s enclosing `ui` that scroll along axis `d`
/// (or whose content may be partial, whatever they scroll), nearest first,
/// with the [`Ui::unique_id`] of their content.
fn enclosing_scroll_area_viewports(ui: &Ui, d: usize) -> Vec<(Id, ScrollAreaViewport)> {
    ui.ctx().pass_state(|state| {
        ui.stack()
            .iter()
            .filter(|frame| frame.kind() == Some(UiKind::ScrollArea))
            .filter_map(|frame| {
                let viewport = state.scroll_area_viewports.get(&frame.unique_id)?;
                (viewport.scrolls[d] || viewport.content_may_be_partial)
                    .then_some((frame.unique_id, *viewport))
            })
            .collect()
    })
}

/// How fast to scroll (points per second, positive meaning down/right) with the pointer at
/// `pointer_pos` while `viewport` is visible and the selection can extend over `selectable`.
///
/// Zero while the pointer is inside the viewport, and faster the further outside it is.
fn auto_scroll_velocity(pointer_pos: Pos2, viewport: Rect, selectable: Rect) -> Vec2 {
    let axis = |pos: f32, viewport: Rangef, selectable: Rangef| {
        let speed = |past_edge: f32| (BASE_SPEED + SPEED_PER_POINT * past_edge).min(MAX_SPEED);
        if pos > viewport.max && viewport.max < selectable.max {
            speed(pos - viewport.max)
        } else if pos < viewport.min && selectable.min < viewport.min {
            -speed(viewport.min - pos)
        } else {
            0.0
        }
    };

    Vec2::new(
        axis(pointer_pos.x, viewport.x_range(), selectable.x_range()),
        axis(pointer_pos.y, viewport.y_range(), selectable.y_range()),
    )
}

#[cfg(test)]
mod tests {
    use emath::pos2;

    use super::*;

    const VIEWPORT: Rect = Rect::from_min_max(pos2(0.0, 0.0), pos2(200.0, 100.0));

    #[test]
    fn no_scrolling_inside_the_viewport() {
        for pos in [pos2(0.0, 0.0), pos2(100.0, 50.0), pos2(200.0, 100.0)] {
            assert_eq!(
                auto_scroll_velocity(pos, VIEWPORT, Rect::EVERYTHING),
                Vec2::ZERO,
                "at {pos:?}"
            );
        }
    }

    #[test]
    fn scrolls_towards_the_pointer_in_both_axes() {
        let below_right = auto_scroll_velocity(pos2(210.0, 110.0), VIEWPORT, Rect::EVERYTHING);
        assert!(
            below_right.x > 0.0 && below_right.y > 0.0,
            "{below_right:?}"
        );

        let above_left = auto_scroll_velocity(pos2(-10.0, -10.0), VIEWPORT, Rect::EVERYTHING);
        assert!(above_left.x < 0.0 && above_left.y < 0.0, "{above_left:?}");
    }

    #[test]
    fn faster_the_further_out_but_capped() {
        let near = auto_scroll_velocity(pos2(100.0, 101.0), VIEWPORT, Rect::EVERYTHING).y;
        let far = auto_scroll_velocity(pos2(100.0, 150.0), VIEWPORT, Rect::EVERYTHING).y;
        let very_far = auto_scroll_velocity(pos2(100.0, 10_000.0), VIEWPORT, Rect::EVERYTHING).y;
        assert!(0.0 < near && near < far, "{near} < {far}");
        assert!(far < very_far, "{far} < {very_far}");
        assert_eq!(very_far, MAX_SPEED);
    }

    #[test]
    fn only_scrolls_where_there_is_more_to_select() {
        // A text edit that fits inside the viewport, like a single-line one in a list:
        let fits = Rect::from_min_max(pos2(10.0, 40.0), pos2(190.0, 60.0));
        assert_eq!(
            auto_scroll_velocity(pos2(100.0, 150.0), VIEWPORT, fits),
            Vec2::ZERO
        );

        // One that continues below the viewport:
        let continues_below = Rect::from_min_max(pos2(10.0, 40.0), pos2(190.0, 600.0));
        assert!(auto_scroll_velocity(pos2(100.0, 150.0), VIEWPORT, continues_below).y > 0.0);
        assert_eq!(
            auto_scroll_velocity(pos2(100.0, -50.0), VIEWPORT, continues_below),
            Vec2::ZERO,
            "nothing more to select above it"
        );
    }
}
