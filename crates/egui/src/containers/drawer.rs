use emath::{Align2, Pos2, Rect, Vec2, vec2};

use crate::{Area, AreaState, Context, Id, InnerResponse, LayerId, Order, Ui};

/// The edge of a target rectangle that a [`Drawer`] is attached to.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum DrawerEdge {
    #[default]
    Left,
    Right,
    Top,
    Bottom,
}

/// An overlay attached to an edge of the current viewport or a caller-provided rectangle.
///
/// A drawer does not reserve space or provide a backdrop. The caller owns its open state and any
/// dismissal or modal behavior. Coordinates are egui points in the current viewport; native
/// window placement and monitor selection are handled by the integration.
///
/// ```
/// # egui::__run_test_ctx(|ctx| {
/// let open = true;
/// egui::Drawer::new(egui::Id::unique("navigation"))
///     .edge(egui::DrawerEdge::Left)
///     .extent(280.0)
///     .open(open)
///     .show(ctx, |ui| ui.label("Navigation"));
/// # });
/// ```
#[must_use = "you should call .show()"]
#[derive(Clone, Debug)]
pub struct Drawer {
    id: Id,
    edge: DrawerEdge,
    extent: f32,
    parent_rect: Option<Rect>,
    open: bool,
    order: Order,
}

impl Drawer {
    /// Create a drawer with the given globally unique id and a default extent of 320 points.
    pub fn new(id: Id) -> Self {
        Self {
            id,
            edge: DrawerEdge::Left,
            extent: 320.0,
            parent_rect: None,
            open: true,
            order: Order::Foreground,
        }
    }

    /// Set the target edge. Defaults to [`DrawerEdge::Left`].
    #[inline]
    pub fn edge(mut self, edge: DrawerEdge) -> Self {
        self.edge = edge;
        self
    }

    /// Set the width for left/right drawers or height for top/bottom drawers, in points.
    ///
    /// The extent is clamped to the target rectangle. Defaults to 320 points.
    #[inline]
    pub fn extent(mut self, extent: f32) -> Self {
        self.extent = extent.max(0.0);
        self
    }

    /// Set the rectangle the drawer attaches to. Defaults to the current viewport content rect.
    ///
    /// The rectangle must use the current viewport's egui point coordinate space.
    #[inline]
    pub fn parent_rect(mut self, rect: Rect) -> Self {
        self.parent_rect = Some(rect);
        self
    }

    /// Set whether to show the drawer. Defaults to `true`.
    #[inline]
    pub fn open(mut self, open: bool) -> Self {
        self.open = open;
        self
    }

    /// Set the layer order. Defaults to [`Order::Foreground`].
    #[inline]
    pub fn order(mut self, order: Order) -> Self {
        self.order = order;
        self
    }

    /// Show the drawer, returning `None` when closed.
    ///
    /// Placement is recomputed from the target rectangle each frame. The contents are given the
    /// full drawer size as their minimum size, so the drawer spans its target along the edge.
    pub fn show<R>(
        self,
        ctx: &Context,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> Option<InnerResponse<R>> {
        if !self.open {
            return None;
        }

        let target_rect = self.parent_rect.unwrap_or_else(|| ctx.content_rect());
        let target_size = target_rect.size().max(Vec2::ZERO);
        let extent = match self.edge {
            DrawerEdge::Left | DrawerEdge::Right => self.extent.min(target_size.x),
            DrawerEdge::Top | DrawerEdge::Bottom => self.extent.min(target_size.y),
        };

        let size = match self.edge {
            DrawerEdge::Left | DrawerEdge::Right => vec2(extent, target_size.y),
            DrawerEdge::Top | DrawerEdge::Bottom => vec2(target_size.x, extent),
        };
        let min = match self.edge {
            DrawerEdge::Left | DrawerEdge::Top => target_rect.left_top(),
            DrawerEdge::Right => Pos2::new(target_rect.right() - size.x, target_rect.top()),
            DrawerEdge::Bottom => Pos2::new(target_rect.left(), target_rect.bottom() - size.y),
        };
        let rect = Rect::from_min_size(min, size);
        let layer_id = LayerId::new(self.order, self.id);

        // Area normally performs an invisible first sizing pass. The drawer's geometry is already
        // known, so seed its state to make it visible and interactive on its first frame.
        let area_state = || AreaState {
            pivot_pos: Some(rect.min),
            pivot: Align2::LEFT_TOP,
            size: Some(size),
            interactable: true,
            last_became_visible_at: None,
        };
        ctx.memory_mut(|mem| mem.areas_mut().set_state(layer_id, area_state()));

        let InnerResponse {
            inner,
            mut response,
        } = Area::new(self.id)
            .order(self.order)
            .movable(false)
            .interactable(true)
            .constrain_to(target_rect)
            .default_size(size)
            .fixed_pos(rect.min)
            .fade_in(false)
            .show(ctx, |ui| {
                ui.set_clip_rect(rect);
                ui.set_min_size(size);
                add_contents(ui)
            });

        // Keep overflowing contents clipped to the drawer and report the drawer bounds even when
        // the child layout asks to grow beyond its target rectangle.
        response.rect = rect;
        response.interact_rect = rect;
        ctx.memory_mut(|mem| mem.areas_mut().set_state(layer_id, area_state()));

        Some(InnerResponse { inner, response })
    }
}

#[cfg(test)]
mod tests {
    use super::{Drawer, DrawerEdge};
    use crate::{Context, Id, RawInput, Rect, pos2, vec2};
    use core::cell::Cell;

    fn show_at(screen_rect: Rect, drawer: &Drawer) -> Option<crate::Response> {
        let ctx = Context::default();
        ctx.set_fonts(crate::FontDefinitions::empty());
        let response = Cell::new(None);
        let output = ctx.run_ui(
            RawInput {
                screen_rect: Some(screen_rect),
                ..Default::default()
            },
            |ctx| {
                response.set(
                    drawer
                        .to_owned()
                        .show(ctx, |_| ())
                        .map(|response| response.response),
                );
            },
        );
        output.drop_without_applying_deltas();
        response.into_inner()
    }

    #[test]
    fn attaches_to_each_edge_and_clamps_extent() {
        let target = Rect::from_min_size(crate::pos2(10.0, 20.0), vec2(300.0, 200.0));
        let cases = [
            (
                DrawerEdge::Left,
                Rect::from_min_size(target.min, vec2(70.0, 200.0)),
            ),
            (
                DrawerEdge::Right,
                Rect::from_min_size(pos2(240.0, 20.0), vec2(70.0, 200.0)),
            ),
            (
                DrawerEdge::Top,
                Rect::from_min_size(target.min, vec2(300.0, 70.0)),
            ),
            (
                DrawerEdge::Bottom,
                Rect::from_min_size(pos2(10.0, 150.0), vec2(300.0, 70.0)),
            ),
        ];

        for (edge, expected) in cases {
            let response = show_at(
                target,
                &Drawer::new(Id::unique("drawer"))
                    .edge(edge)
                    .extent(70.0)
                    .parent_rect(target),
            )
            .unwrap();
            assert_eq!(response.rect, expected);
        }

        let clamped = show_at(
            target,
            &Drawer::new(Id::unique("clamped"))
                .edge(DrawerEdge::Right)
                .extent(500.0)
                .parent_rect(target),
        )
        .unwrap();
        assert_eq!(clamped.rect, target);
    }

    #[test]
    fn positions_and_sizes_follow_viewport_changes() {
        let first = Rect::from_min_size(crate::pos2(0.0, 0.0), vec2(800.0, 600.0));
        let second = Rect::from_min_size(crate::pos2(0.0, 0.0), vec2(500.0, 400.0));
        let ctx = Context::default();
        ctx.set_fonts(crate::FontDefinitions::empty());
        let drawer = || {
            Drawer::new(Id::unique("drawer"))
                .edge(DrawerEdge::Right)
                .extent(320.0)
        };

        let response = Cell::new(None);
        let output = ctx.run_ui(
            RawInput {
                screen_rect: Some(first),
                ..Default::default()
            },
            |ctx| response.set(Some(drawer().show(ctx, |_| ()).unwrap().response.rect)),
        );
        output.drop_without_applying_deltas();
        assert_eq!(
            response.get().unwrap(),
            Rect::from_min_size(crate::pos2(480.0, 0.0), vec2(320.0, 600.0))
        );
        let response = Cell::new(None);
        let output = ctx.run_ui(
            RawInput {
                screen_rect: Some(second),
                ..Default::default()
            },
            |ctx| response.set(Some(drawer().show(ctx, |_| ()).unwrap().response.rect)),
        );
        output.drop_without_applying_deltas();
        assert_eq!(
            response.get().unwrap(),
            Rect::from_min_size(crate::pos2(180.0, 0.0), vec2(320.0, 400.0))
        );
    }

    #[test]
    fn closed_drawer_does_not_run_contents() {
        let ctx = Context::default();
        ctx.set_fonts(crate::FontDefinitions::empty());
        let called = Cell::new(false);
        let output = ctx.run_ui(RawInput::default(), |ctx| {
            let output = Drawer::new(Id::unique("drawer"))
                .open(false)
                .show(ctx, |_| called.set(true));
            assert!(output.is_none());
        });
        output.drop_without_applying_deltas();
        assert!(!called.get());
    }

    #[test]
    fn custom_parent_rect_is_used_for_placement() {
        let viewport = Rect::from_min_size(crate::pos2(0.0, 0.0), vec2(800.0, 600.0));
        let parent = Rect::from_min_size(pos2(40.0, 50.0), vec2(400.0, 300.0));
        let response = show_at(
            viewport,
            &Drawer::new(Id::unique("drawer"))
                .edge(DrawerEdge::Bottom)
                .extent(80.0)
                .parent_rect(parent),
        )
        .unwrap();
        assert_eq!(
            response.rect,
            Rect::from_min_size(pos2(40.0, 270.0), vec2(400.0, 80.0))
        );
    }

    #[test]
    fn oversized_contents_do_not_expand_drawer_bounds() {
        let ctx = Context::default();
        ctx.set_fonts(crate::FontDefinitions::empty());
        let screen_rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(800.0, 600.0));
        let response = Cell::new(None);
        let output = ctx.run_ui(
            RawInput {
                screen_rect: Some(screen_rect),
                ..Default::default()
            },
            |ctx| {
                response.set(Some(
                    Drawer::new(Id::unique("drawer"))
                        .edge(DrawerEdge::Left)
                        .extent(120.0)
                        .show(ctx, |ui| {
                            let _ = ui.allocate_space(vec2(400.0, 800.0));
                        })
                        .unwrap()
                        .response
                        .rect,
                ));
            },
        );
        output.drop_without_applying_deltas();
        assert_eq!(
            response.get().unwrap(),
            Rect::from_min_size(pos2(0.0, 0.0), vec2(120.0, 600.0))
        );
    }

    #[test]
    fn viewport_scale_change_keeps_logical_placement() {
        let screen_rect = Rect::from_min_size(pos2(0.0, 0.0), vec2(800.0, 600.0));
        let ctx = Context::default();
        ctx.set_fonts(crate::FontDefinitions::empty());
        let response = Cell::new(None);

        let output = ctx.run_ui(
            RawInput {
                screen_rect: Some(screen_rect),
                ..Default::default()
            },
            |ctx| {
                response.set(Some(
                    Drawer::new(Id::unique("drawer"))
                        .edge(DrawerEdge::Right)
                        .extent(200.0)
                        .show(ctx, |_| ())
                        .unwrap()
                        .response
                        .rect,
                ));
            },
        );
        output.drop_without_applying_deltas();
        assert_eq!(
            response.get().unwrap(),
            Rect::from_min_size(pos2(600.0, 0.0), vec2(200.0, 600.0))
        );

        ctx.set_zoom_factor(2.0);
        let output = ctx.run_ui(
            RawInput {
                screen_rect: Some(screen_rect),
                ..Default::default()
            },
            |ctx| {
                response.set(Some(
                    Drawer::new(Id::unique("drawer"))
                        .edge(DrawerEdge::Right)
                        .extent(200.0)
                        .show(ctx, |_| ())
                        .unwrap()
                        .response
                        .rect,
                ));
            },
        );
        output.drop_without_applying_deltas();
        assert_eq!(
            response.get().unwrap(),
            Rect::from_min_size(pos2(200.0, 0.0), vec2(200.0, 300.0))
        );
    }
}
