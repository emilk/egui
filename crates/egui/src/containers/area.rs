//! Area is a [`Ui`] that has no parent, it floats on the background.
//! It has no frame or own size. It is potentially movable.
//! It is the foundation for windows and popups.

use crate::*;

/// State that is persisted between frames.
// TODO(emilk): this is not currently stored in `Memory::data`, but maybe it should be?
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub(crate) struct State {
    /// Last known pos of the pivot
    pub pivot_pos: Pos2,

    pub pivot: Align2,

    /// Last know size. Used for catching clicks.
    pub size: Vec2,

    /// If false, clicks goes straight through to what is behind us.
    /// Good for tooltips etc.
    pub interactable: bool,
}

impl State {
    pub fn left_top_pos(&self) -> Pos2 {
        pos2(
            self.pivot_pos.x - self.pivot.x().to_factor() * self.size.x,
            self.pivot_pos.y - self.pivot.y().to_factor() * self.size.y,
        )
    }

    pub fn set_left_top_pos(&mut self, pos: Pos2) {
        self.pivot_pos = pos2(
            pos.x + self.pivot.x().to_factor() * self.size.x,
            pos.y + self.pivot.y().to_factor() * self.size.y,
        );
    }

    pub fn rect(&self) -> Rect {
        Rect::from_min_size(self.left_top_pos(), self.size)
    }
}

/// An area on the screen that can be moved by dragging.
///
/// This forms the base of the [`Window`] container.
///
/// ```
/// # egui::__run_test_ctx(|ctx| {
/// egui::Area::new("my_area")
///     .fixed_pos(egui::pos2(32.0, 32.0))
///     .show(ctx, |ui| {
///         ui.label("Floating text!");
///     });
/// # });
/// ```
#[must_use = "You should call .show()"]
#[derive(Clone, Copy, Debug)]
pub struct Area {
    pub(crate) id: Id,
    movable: bool,
    interactable: bool,
    enabled: bool,
    constrain: bool,
    order: Order,
    default_pos: Option<Pos2>,
    pivot: Align2,
    anchor: Option<(Align2, Vec2)>,
    new_pos: Option<Pos2>,
    drag_bounds: Option<Rect>,
}

impl Area {
    pub fn new(id: impl Into<Id>) -> Self {
        Self {
            id: id.into(),
            movable: true,
            interactable: true,
            constrain: false,
            enabled: true,
            order: Order::Middle,
            default_pos: None,
            new_pos: None,
            pivot: Align2::LEFT_TOP,
            anchor: None,
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
        self.new_pos = Some(fixed_pos.into());
        self.movable = false;
        self
    }

    /// Constrains this area to the screen bounds.
    pub fn constrain(mut self, constrain: bool) -> Self {
        self.constrain = constrain;
        self
    }

    /// Where the "root" of the area is.
    ///
    /// For instance, if you set this to [`Align2::RIGHT_TOP`]
    /// then [`Self::fixed_pos`] will set the position of the right-top
    /// corner of the area.
    ///
    /// Default: [`Align2::LEFT_TOP`].
    pub fn pivot(mut self, pivot: Align2) -> Self {
        self.pivot = pivot;
        self
    }

    /// Positions the window but you can still move it.
    pub fn current_pos(mut self, current_pos: impl Into<Pos2>) -> Self {
        self.new_pos = Some(current_pos.into());
        self
    }

    /// Set anchor and distance.
    ///
    /// An anchor of `Align2::RIGHT_TOP` means "put the right-top corner of the window
    /// in the right-top corner of the screen".
    ///
    /// The offset is added to the position, so e.g. an offset of `[-5.0, 5.0]`
    /// would move the window left and down from the given anchor.
    ///
    /// Anchoring also makes the window immovable.
    ///
    /// It is an error to set both an anchor and a position.
    pub fn anchor(mut self, align: Align2, offset: impl Into<Vec2>) -> Self {
        self.anchor = Some((align, offset.into()));
        self.movable(false)
    }

    /// Constrain the area up to which the window can be dragged.
    pub fn drag_bounds(mut self, bounds: Rect) -> Self {
        self.drag_bounds = Some(bounds);
        self
    }

    pub(crate) fn get_pivot(&self) -> Align2 {
        if let Some((pivot, _)) = self.anchor {
            pivot
        } else {
            Align2::LEFT_TOP
        }
    }
}

pub(crate) struct Prepared {
    layer_id: LayerId,
    state: State,
    move_response: Response,
    enabled: bool,
    drag_bounds: Option<Rect>,

    /// We always make windows invisible the first frame to hide "first-frame-jitters".
    ///
    /// This is so that we use the first frame to calculate the window size,
    /// and then can correctly position the window and its contents the next frame,
    /// without having one frame where the window is wrongly positioned or sized.
    temporarily_invisible: bool,
}

impl Area {
    pub fn show<R>(
        self,
        ctx: &Context,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> InnerResponse<R> {
        let prepared = self.begin(ctx);
        let mut content_ui = prepared.content_ui(ctx);
        let inner = add_contents(&mut content_ui);
        let response = prepared.end(ctx, content_ui);
        InnerResponse { inner, response }
    }

    pub(crate) fn begin(self, ctx: &Context) -> Prepared {
        let Area {
            id,
            movable,
            order,
            interactable,
            enabled,
            default_pos,
            new_pos,
            pivot,
            anchor,
            drag_bounds,
            constrain,
        } = self;

        let layer_id = LayerId::new(order, id);

        let state = ctx.memory(|mem| mem.areas.get(id).copied());
        let is_new = state.is_none();
        if is_new {
            ctx.request_repaint(); // if we don't know the previous size we are likely drawing the area in the wrong place
        }
        let mut state = state.unwrap_or_else(|| State {
            pivot_pos: default_pos.unwrap_or_else(|| automatic_area_position(ctx)),
            pivot,
            size: Vec2::ZERO,
            interactable,
        });
        state.pivot_pos = new_pos.unwrap_or(state.pivot_pos);
        state.interactable = interactable;

        if let Some((anchor, offset)) = anchor {
            let screen = ctx.available_rect();
            state.set_left_top_pos(
                anchor.align_size_within_rect(state.size, screen).left_top() + offset,
            );
        }

        // interact right away to prevent frame-delay
        let move_response = {
            let interact_id = layer_id.id.with("move");
            let sense = if movable {
                Sense::click_and_drag()
            } else if interactable {
                Sense::click() // allow clicks to bring to front
            } else {
                Sense::hover()
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

            // Important check - don't try to move e.g. a combobox popup!
            if movable {
                if move_response.dragged() {
                    state.pivot_pos += ctx.input(|i| i.pointer.delta());
                }

                state.set_left_top_pos(
                    ctx.constrain_window_rect_to_area(state.rect(), drag_bounds)
                        .min,
                );
            }

            if (move_response.dragged() || move_response.clicked())
                || pointer_pressed_on_area(ctx, layer_id)
                || !ctx.memory(|m| m.areas.visible_last_frame(&layer_id))
            {
                ctx.memory_mut(|m| m.areas.move_to_top(layer_id));
                ctx.request_repaint();
            }

            move_response
        };

        state.set_left_top_pos(ctx.round_pos_to_pixels(state.left_top_pos()));

        if constrain {
            state.set_left_top_pos(
                ctx.constrain_window_rect_to_area(state.rect(), drag_bounds)
                    .left_top(),
            );
        }

        Prepared {
            layer_id,
            state,
            move_response,
            enabled,
            drag_bounds,
            temporarily_invisible: is_new,
        }
    }

    pub fn show_open_close_animation(&self, ctx: &Context, frame: &Frame, is_open: bool) {
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
        let area_rect = ctx.memory(|mem| mem.areas.get(self.id).map(|area| area.rect()));
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

    pub(crate) fn content_ui(&self, ctx: &Context) -> Ui {
        let screen_rect = ctx.screen_rect();

        let bounds = if let Some(bounds) = self.drag_bounds {
            bounds.intersect(screen_rect) // protect against infinite bounds
        } else {
            let central_area = ctx.available_rect();

            let is_within_central_area = central_area.contains_rect(self.state.rect().shrink(1.0));
            if is_within_central_area {
                central_area // let's try to not cover side panels
            } else {
                screen_rect
            }
        };

        let max_rect = Rect::from_min_max(
            self.state.left_top_pos(),
            bounds
                .max
                .at_least(self.state.left_top_pos() + Vec2::splat(32.0)),
        );

        let shadow_radius = ctx.style().visuals.window_shadow.extrusion; // hacky
        let clip_rect_margin = ctx.style().visuals.clip_rect_margin.max(shadow_radius);

        let clip_rect = Rect::from_min_max(self.state.left_top_pos(), bounds.max)
            .expand(clip_rect_margin)
            .intersect(bounds);

        let mut ui = Ui::new(
            ctx.clone(),
            self.layer_id,
            self.layer_id.id,
            max_rect,
            clip_rect,
        );
        ui.set_enabled(self.enabled);
        ui.set_visible(!self.temporarily_invisible);
        ui
    }

    #[allow(clippy::needless_pass_by_value)] // intentional to swallow up `content_ui`.
    pub(crate) fn end(self, ctx: &Context, content_ui: Ui) -> Response {
        let Prepared {
            layer_id,
            mut state,
            move_response,
            enabled: _,
            drag_bounds: _,
            temporarily_invisible: _,
        } = self;

        state.size = content_ui.min_rect().size();

        ctx.memory_mut(|m| m.areas.set_state(layer_id, state));

        move_response
    }
}

fn pointer_pressed_on_area(ctx: &Context, layer_id: LayerId) -> bool {
    if let Some(pointer_pos) = ctx.pointer_interact_pos() {
        let any_pressed = ctx.input(|i| i.pointer.any_pressed());
        any_pressed && ctx.layer_id_at(pointer_pos) == Some(layer_id)
    } else {
        false
    }
}

fn automatic_area_position(ctx: &Context) -> Pos2 {
    let mut existing: Vec<Rect> = ctx.memory(|mem| {
        mem.areas
            .visible_windows()
            .into_iter()
            .map(State::rect)
            .collect()
    });
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
