//! Area is a `Ui` that has no parent, it floats on the background.
//! It has no frame or own size. It is potentially movable.
//! It is the foundation for windows and popups.

use std::{fmt::Debug, hash::Hash};

use crate::*;

/// State that is persisted between frames
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
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
    id: Id,
    movable: bool,
    interactable: bool,
    order: Order,
    default_pos: Option<Pos2>,
    fixed_pos: Option<Pos2>,
}

impl Area {
    pub fn new(id_source: impl Hash) -> Self {
        Self {
            id: Id::new(id_source),
            movable: true,
            interactable: true,
            order: Order::Middle,
            default_pos: None,
            fixed_pos: None,
        }
    }

    pub fn layer(&self) -> LayerId {
        LayerId::new(self.order, self.id)
    }

    /// moveable by dragging the area?
    pub fn movable(mut self, movable: bool) -> Self {
        self.movable = movable;
        self.interactable |= movable;
        self
    }

    pub fn is_movable(&self) -> bool {
        self.movable
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
        self.default_pos = Some(fixed_pos);
        self.fixed_pos = Some(fixed_pos);
        self.movable = false;
        self
    }
}

pub(crate) struct Prepared {
    layer_id: LayerId,
    state: State,
    movable: bool,
}

impl Area {
    pub(crate) fn begin(self, ctx: &CtxRef) -> Prepared {
        let Area {
            id,
            movable,
            order,
            interactable,
            default_pos,
            fixed_pos,
        } = self;

        let layer_id = LayerId::new(order, id);

        let state = ctx.memory().areas.get(id).cloned();
        let mut state = state.unwrap_or_else(|| State {
            pos: default_pos.unwrap_or_else(|| automatic_area_position(ctx)),
            size: Vec2::zero(),
            interactable,
        });
        state.pos = fixed_pos.unwrap_or(state.pos);
        state.pos = ctx.round_pos_to_pixels(state.pos);

        Prepared {
            layer_id,
            state,
            movable,
        }
    }

    pub fn show(self, ctx: &CtxRef, add_contents: impl FnOnce(&mut Ui)) -> Response {
        let prepared = self.begin(ctx);
        let mut content_ui = prepared.content_ui(ctx);
        add_contents(&mut content_ui);
        prepared.end(ctx, content_ui)
    }
}

impl Prepared {
    pub(crate) fn state(&self) -> &State {
        &self.state
    }

    pub(crate) fn state_mut(&mut self) -> &mut State {
        &mut self.state
    }

    pub(crate) fn content_ui(&self, ctx: &CtxRef) -> Ui {
        let max_rect = Rect::from_min_size(self.state.pos, Vec2::infinity());
        let clip_rect = max_rect
            .expand(ctx.style().visuals.clip_rect_margin)
            .intersect(ctx.input().screen_rect);
        Ui::new(
            ctx.clone(),
            self.layer_id,
            self.layer_id.id,
            max_rect,
            clip_rect,
        )
    }

    #[allow(clippy::needless_pass_by_value)] // intentional to swallow up `content_ui`.
    pub(crate) fn end(self, ctx: &CtxRef, content_ui: Ui) -> Response {
        let Prepared {
            layer_id,
            mut state,
            movable,
        } = self;

        state.size = content_ui.min_rect().size();

        let interact_id = layer_id.id.with("move");
        let sense = if movable {
            Sense::click_and_drag()
        } else {
            Sense::click() // allow clicks to bring to front
        };

        let move_response = ctx.interact(
            Rect::everything(),
            ctx.style().spacing.item_spacing,
            layer_id,
            interact_id,
            state.rect(),
            sense,
        );

        if move_response.active && movable {
            state.pos += ctx.input().mouse.delta;
        }

        state.pos = ctx.constrain_window_rect(state.rect()).min;

        if (move_response.active || move_response.clicked)
            || mouse_pressed_on_area(ctx, layer_id)
            || !ctx.memory().areas.visible_last_frame(&layer_id)
        {
            ctx.memory().areas.move_to_top(layer_id);
            ctx.request_repaint();
        }
        ctx.memory().areas.set_state(layer_id, state);

        move_response
    }
}

fn mouse_pressed_on_area(ctx: &Context, layer_id: LayerId) -> bool {
    if let Some(mouse_pos) = ctx.input().mouse.pos {
        ctx.input().mouse.pressed && ctx.layer_id_at(mouse_pos) == Some(layer_id)
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
