//! Area is a `Ui` that has no parent, it floats on the background.
//! It has no frame or own size. It is potentially movable.
//! It is the foundation for windows and popups.

use std::{fmt::Debug, hash::Hash};

use crate::*;

/// State that is persisted between frames
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "persistence", derive(serde::Deserialize, serde::Serialize))]
pub(crate) struct State {
    /// Last known pos
    pub pos: Pos2,

    /// Last know size. Used for catching clicks.
    pub size: Vec2,

    /// If false, clicks goes straight through to what is behind us.
    /// Good for tooltips etc.
    pub interactable: bool,
}

impl State {
    pub fn rect(&self) -> Rect {
        Rect::from_min_size(self.pos, self.size)
    }
}

/// An area on the screen that can be moved by dragging.
///
/// This forms the base of the [`Window`] container.
///
/// ```
/// # let mut ctx = egui::CtxRef::default();
/// # ctx.begin_frame(Default::default());
/// # let ctx = &ctx;
/// egui::Area::new("my_area")
///     .fixed_pos(egui::pos2(32.0, 32.0))
///     .show(ctx, |ui| {
///         ui.label("Floating text!");
///     });
#[derive(Clone, Copy, Debug)]
pub struct Area {
    pub(crate) id: Id,
    movable: bool,
    interactable: bool,
    enabled: bool,
    order: Order,
    default_pos: Option<Pos2>,
    new_pos: Option<Pos2>,
    drag_bounds: Option<Rect>,
}

impl Area {
    pub fn new(id_source: impl Hash) -> Self {
        Self {
            id: Id::new(id_source),
            movable: true,
            interactable: true,
            enabled: true,
            order: Order::Middle,
            default_pos: None,
            new_pos: None,
            drag_bounds: None,
        }
    }

    pub fn id(mut self, id: Id) -> Self {
        self.id = id;
        self
    }

    pub fn layer(&self) -> LayerId {
        LayerId::new(self.order, self.id)
    }

    /// If false, no content responds to click
    /// and widgets will be shown grayed out.
    /// You won't be able to move the window.
    /// Default: `true`.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// moveable by dragging the area?
    pub fn movable(mut self, movable: bool) -> Self {
        self.movable = movable;
        self.interactable |= movable;
        self
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn is_movable(&self) -> bool {
        self.movable && self.enabled
    }

    /// If false, clicks goes straight through to what is behind us.
    /// Good for tooltips etc.
    pub fn interactable(mut self, interactable: bool) -> Self {
        self.interactable = interactable;
        self.movable &= interactable;
        self
    }

    /// `order(Order::Foreground)` for an Area that should always be on top
    pub fn order(mut self, order: Order) -> Self {
        self.order = order;
        self
    }

    pub fn default_pos(mut self, default_pos: impl Into<Pos2>) -> Self {
        self.default_pos = Some(default_pos.into());
        self
    }

    /// Positions the window and prevents it from being moved
    pub fn fixed_pos(mut self, fixed_pos: impl Into<Pos2>) -> Self {
        let fixed_pos = fixed_pos.into();
        self.new_pos = Some(fixed_pos);
        self.movable = false;
        self
    }

    /// Positions the window but you can still move it.
    pub fn current_pos(mut self, current_pos: impl Into<Pos2>) -> Self {
        let current_pos = current_pos.into();
        self.new_pos = Some(current_pos);
        self
    }

    /// Constrain the area up to which the window can be dragged.
    pub fn drag_bounds(mut self, bounds: Rect) -> Self {
        self.drag_bounds = Some(bounds);
        self
    }
}

pub(crate) struct Prepared {
    layer_id: LayerId,
    state: State,
    movable: bool,
    enabled: bool,
    drag_bounds: Option<Rect>,
}

impl Area {
    pub(crate) fn begin(self, ctx: &CtxRef) -> Prepared {
        let Area {
            id,
            movable,
            order,
            interactable,
            enabled,
            default_pos,
            new_pos,
            drag_bounds,
        } = self;

        let layer_id = LayerId::new(order, id);

        let state = ctx.memory().areas.get(id).cloned();
        let mut state = state.unwrap_or_else(|| State {
            pos: default_pos.unwrap_or_else(|| automatic_area_position(ctx)),
            size: Vec2::ZERO,
            interactable,
        });
        state.pos = new_pos.unwrap_or(state.pos);
        state.pos = ctx.round_pos_to_pixels(state.pos);

        Prepared {
            layer_id,
            state,
            movable,
            enabled,
            drag_bounds,
        }
    }

    pub fn show(self, ctx: &CtxRef, add_contents: impl FnOnce(&mut Ui)) -> Response {
        let prepared = self.begin(ctx);
        let mut content_ui = prepared.content_ui(ctx);
        add_contents(&mut content_ui);
        prepared.end(ctx, content_ui)
    }

    pub fn show_open_close_animation(&self, ctx: &CtxRef, frame: &Frame, is_open: bool) {
        // must be called first so animation managers know the latest state
        let visibility_factor = ctx.animate_bool(self.id.with("close_animation"), is_open);

        if is_open {
            // we actually only show close animations.
            // when opening a window we show it right away.
            return;
        }
        if visibility_factor <= 0.0 {
            return;
        }

        let layer_id = LayerId::new(self.order, self.id);
        let area_rect = ctx.memory().areas.get(self.id).map(|area| area.rect());
        if let Some(area_rect) = area_rect {
            let clip_rect = ctx.available_rect();
            let painter = Painter::new(ctx.clone(), layer_id, clip_rect);

            // shrinkage: looks kinda a bad on its own
            // let area_rect =
            //     Rect::from_center_size(area_rect.center(), visibility_factor * area_rect.size());

            let frame = frame.multiply_with_opacity(visibility_factor);
            painter.add(frame.paint(area_rect));
        }
    }
}

impl Prepared {
    pub(crate) fn state(&self) -> &State {
        &self.state
    }

    pub(crate) fn state_mut(&mut self) -> &mut State {
        &mut self.state
    }

    pub(crate) fn drag_bounds(&self) -> Option<Rect> {
        self.drag_bounds
    }

    pub(crate) fn content_ui(&self, ctx: &CtxRef) -> Ui {
        let max_rect = Rect::from_min_size(self.state.pos, Vec2::INFINITY);
        let shadow_radius = ctx.style().visuals.window_shadow.extrusion; // hacky
        let bounds = self.drag_bounds.unwrap_or_else(|| ctx.input().screen_rect);

        let mut clip_rect = max_rect
            .expand(ctx.style().visuals.clip_rect_margin)
            .expand(shadow_radius)
            .intersect(bounds);

        // Windows are constrained to central area,
        // (except in rare cases where they don't fit).
        // Adjust clip rect so we don't cast shadows on side panels:
        let central_area = ctx.available_rect();
        let is_within_central_area = central_area.contains(self.state.pos);
        if is_within_central_area {
            clip_rect = clip_rect.intersect(central_area);
        }

        let mut ui = Ui::new(
            ctx.clone(),
            self.layer_id,
            self.layer_id.id,
            max_rect,
            clip_rect,
        );
        ui.set_enabled(self.enabled);

        ui
    }

    #[allow(clippy::needless_pass_by_value)] // intentional to swallow up `content_ui`.
    pub(crate) fn end(self, ctx: &CtxRef, content_ui: Ui) -> Response {
        let Prepared {
            layer_id,
            mut state,
            movable,
            enabled,
            drag_bounds,
        } = self;

        state.size = content_ui.min_rect().size();

        let interact_id = layer_id.id.with("move");
        let sense = if movable {
            Sense::click_and_drag()
        } else {
            Sense::click() // allow clicks to bring to front
        };

        let move_response = ctx.interact(
            Rect::EVERYTHING,
            ctx.style().spacing.item_spacing,
            layer_id,
            interact_id,
            state.rect(),
            sense,
            enabled,
        );

        if move_response.dragged() && movable {
            state.pos += ctx.input().pointer.delta();
        }

        if let Some(bounds) = drag_bounds {
            state.pos = ctx.constrain_window_rect_to_area(state.rect(), bounds).min;
        } else {
            state.pos = ctx.constrain_window_rect(state.rect()).min;
        }

        if (move_response.dragged() || move_response.clicked())
            || pointer_pressed_on_area(ctx, layer_id)
            || !ctx.memory().areas.visible_last_frame(&layer_id)
        {
            ctx.memory().areas.move_to_top(layer_id);
            ctx.request_repaint();
        }
        ctx.memory().areas.set_state(layer_id, state);

        move_response
    }
}

fn pointer_pressed_on_area(ctx: &Context, layer_id: LayerId) -> bool {
    if let Some(pointer_pos) = ctx.input().pointer.interact_pos() {
        ctx.input().pointer.any_pressed() && ctx.layer_id_at(pointer_pos) == Some(layer_id)
    } else {
        false
    }
}

fn automatic_area_position(ctx: &Context) -> Pos2 {
    let mut existing: Vec<Rect> = ctx
        .memory()
        .areas
        .visible_windows()
        .into_iter()
        .map(State::rect)
        .collect();
    existing.sort_by_key(|r| r.left().round() as i32);

    let available_rect = ctx.available_rect();

    let spacing = 16.0;
    let left = available_rect.left() + spacing;
    let top = available_rect.top() + spacing;

    if existing.is_empty() {
        return pos2(left, top);
    }

    // Separate existing rectangles into columns:
    let mut column_bbs = vec![existing[0]];

    for &rect in &existing {
        let current_column_bb = column_bbs.last_mut().unwrap();
        if rect.left() < current_column_bb.right() {
            // same column
            *current_column_bb = current_column_bb.union(rect);
        } else {
            // new column
            column_bbs.push(rect);
        }
    }

    {
        // Look for large spaces between columns (empty columns):
        let mut x = left;
        for col_bb in &column_bbs {
            let available = col_bb.left() - x;
            if available >= 300.0 {
                return pos2(x, top);
            }
            x = col_bb.right() + spacing;
        }
    }

    // Find first column with some available space at the bottom of it:
    for col_bb in &column_bbs {
        if col_bb.bottom() < available_rect.center().y {
            return pos2(col_bb.left(), col_bb.bottom() + spacing);
        }
    }

    // Maybe we can fit a new column?
    let rightmost = column_bbs.last().unwrap().right();
    if rightmost + 200.0 < available_rect.right() {
        return pos2(rightmost + spacing, top);
    }

    // Ok, just put us in the column with the most space at the bottom:
    let mut best_pos = pos2(left, column_bbs[0].bottom() + spacing);
    for col_bb in &column_bbs {
        let col_pos = pos2(col_bb.left(), col_bb.bottom() + spacing);
        if col_pos.y < best_pos.y {
            best_pos = col_pos;
        }
    }
    best_pos
}
