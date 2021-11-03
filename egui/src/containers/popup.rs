//! Show popup windows, tooltips, context menus etc.

use crate::*;

// ----------------------------------------------------------------------------

/// Same state for all tooltips.
#[derive(Clone, Debug, Default)]
pub(crate) struct MonoState {
    last_id: Option<Id>,
    last_size: Vec<Vec2>,
}

impl MonoState {
    fn load(ctx: &Context) -> Option<Self> {
        ctx.memory().data.get_temp(Id::null())
    }

    fn store(self, ctx: &Context) {
        ctx.memory().data.insert_temp(Id::null(), self);
    }

    fn tooltip_size(&self, id: Id, index: usize) -> Option<Vec2> {
        if self.last_id == Some(id) {
            self.last_size.get(index).cloned()
        } else {
            None
        }
    }

    fn set_tooltip_size(&mut self, id: Id, index: usize, size: Vec2) {
        if self.last_id == Some(id) {
            if let Some(stored_size) = self.last_size.get_mut(index) {
                *stored_size = size;
            } else {
                self.last_size
                    .extend((0..index - self.last_size.len()).map(|_| Vec2::ZERO));
                self.last_size.push(size);
            }
            return;
        }

        self.last_id = Some(id);
        self.last_size.clear();
        self.last_size.extend((0..index).map(|_| Vec2::ZERO));
        self.last_size.push(size);
    }
}

// ----------------------------------------------------------------------------

/// Show a tooltip at the current pointer position (if any).
///
/// Most of the time it is easier to use [`Response::on_hover_ui`].
///
/// See also [`show_tooltip_text`].
///
/// Returns `None` if the tooltip could not be placed.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// if ui.ui_contains_pointer() {
///     egui::show_tooltip(ui.ctx(), egui::Id::new("my_tooltip"), |ui| {
///         ui.label("Helpful text");
///     });
/// }
/// # });
/// ```
pub fn show_tooltip<R>(ctx: &CtxRef, id: Id, add_contents: impl FnOnce(&mut Ui) -> R) -> Option<R> {
    show_tooltip_at_pointer(ctx, id, add_contents)
}

/// Show a tooltip at the current pointer position (if any).
///
/// Most of the time it is easier to use [`Response::on_hover_ui`].
///
/// See also [`show_tooltip_text`].
///
/// Returns `None` if the tooltip could not be placed.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// if ui.ui_contains_pointer() {
///     egui::show_tooltip_at_pointer(ui.ctx(), egui::Id::new("my_tooltip"), |ui| {
///         ui.label("Helpful text");
///     });
/// }
/// # });
/// ```
pub fn show_tooltip_at_pointer<R>(
    ctx: &CtxRef,
    id: Id,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> Option<R> {
    let suggested_pos = ctx
        .input()
        .pointer
        .hover_pos()
        .map(|pointer_pos| pointer_pos + vec2(16.0, 16.0));
    show_tooltip_at(ctx, id, suggested_pos, add_contents)
}

/// Show a tooltip under the given area.
///
/// If the tooltip does not fit under the area, it tries to place it above it instead.
pub fn show_tooltip_for<R>(
    ctx: &CtxRef,
    id: Id,
    rect: &Rect,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> Option<R> {
    let expanded_rect = rect.expand2(vec2(2.0, 4.0));
    let (above, position) = if ctx.input().any_touches() {
        (true, expanded_rect.left_top())
    } else {
        (false, expanded_rect.left_bottom())
    };
    show_tooltip_at_avoid_dyn(
        ctx,
        id,
        Some(position),
        above,
        expanded_rect,
        Box::new(add_contents),
    )
}

/// Show a tooltip at the given position.
///
/// Returns `None` if the tooltip could not be placed.
pub fn show_tooltip_at<R>(
    ctx: &CtxRef,
    id: Id,
    suggested_position: Option<Pos2>,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> Option<R> {
    let above = false;
    show_tooltip_at_avoid_dyn(
        ctx,
        id,
        suggested_position,
        above,
        Rect::NOTHING,
        Box::new(add_contents),
    )
}

fn show_tooltip_at_avoid_dyn<'c, R>(
    ctx: &CtxRef,
    mut id: Id,
    suggested_position: Option<Pos2>,
    above: bool,
    mut avoid_rect: Rect,
    add_contents: Box<dyn FnOnce(&mut Ui) -> R + 'c>,
) -> Option<R> {
    let mut tooltip_rect = Rect::NOTHING;
    let mut count = 0;

    let mut position = if let Some((stored_id, stored_tooltip_rect, stored_count)) =
        ctx.frame_state().tooltip_rect
    {
        // if there are multiple tooltips open they should use the same id for the `tooltip_size` caching to work.
        id = stored_id;
        tooltip_rect = stored_tooltip_rect;
        count = stored_count;
        avoid_rect = avoid_rect.union(tooltip_rect);
        if above {
            tooltip_rect.left_top()
        } else {
            tooltip_rect.left_bottom()
        }
    } else if let Some(position) = suggested_position {
        position
    } else if ctx.memory().everything_is_visible() {
        Pos2::ZERO
    } else {
        return None; // No good place for a tooltip :(
    };

    let mut state = MonoState::load(ctx).unwrap_or_default();
    let expected_size = state.tooltip_size(id, count);
    let expected_size = expected_size.unwrap_or_else(|| vec2(64.0, 32.0));

    if above {
        position.y -= expected_size.y;
    }

    position = position.at_most(ctx.input().screen_rect().max - expected_size);

    // check if we intersect the avoid_rect
    {
        let new_rect = Rect::from_min_size(position, expected_size);

        // Note: We do not use Rect::intersects() since it returns true even if the rects only touch.
        if new_rect.shrink(1.0).intersects(avoid_rect) {
            if above {
                // place below instead:
                position = avoid_rect.left_bottom();
            } else {
                // place above instead:
                position = Pos2::new(position.x, avoid_rect.min.y - expected_size.y);
            }
        }
    }

    let position = position.at_least(ctx.input().screen_rect().min);

    let InnerResponse { inner, response } = show_tooltip_area_dyn(ctx, id, position, add_contents);

    state.set_tooltip_size(id, count, response.rect.size());
    state.store(ctx);

    ctx.frame_state().tooltip_rect = Some((id, tooltip_rect.union(response.rect), count + 1));
    Some(inner)
}

/// Show some text at the current pointer position (if any).
///
/// Most of the time it is easier to use [`Response::on_hover_text`].
///
/// See also [`show_tooltip`].
///
/// Returns `None` if the tooltip could not be placed.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// if ui.ui_contains_pointer() {
///     egui::show_tooltip_text(ui.ctx(), egui::Id::new("my_tooltip"), "Helpful text");
/// }
/// # });
/// ```
pub fn show_tooltip_text(ctx: &CtxRef, id: Id, text: impl Into<WidgetText>) -> Option<()> {
    show_tooltip(ctx, id, |ui| {
        crate::widgets::Label::new(text).ui(ui);
    })
}

/// Show a pop-over window.
fn show_tooltip_area_dyn<'c, R>(
    ctx: &CtxRef,
    id: Id,
    window_pos: Pos2,
    add_contents: Box<dyn FnOnce(&mut Ui) -> R + 'c>,
) -> InnerResponse<R> {
    use containers::*;
    Area::new(id)
        .order(Order::Tooltip)
        .fixed_pos(window_pos)
        .interactable(false)
        .drag_bounds(Rect::EVERYTHING) // disable clip rect
        .show(ctx, |ui| {
            Frame::popup(&ctx.style())
                .show(ui, |ui| {
                    ui.set_max_width(ui.spacing().tooltip_width);
                    add_contents(ui)
                })
                .inner
        })
}

/// Shows a popup below another widget.
///
/// Useful for drop-down menus (combo boxes) or suggestion menus under text fields.
///
/// You must open the popup with [`Memory::open_popup`] or  [`Memory::toggle_popup`].
///
/// Returns `None` if the popup is not open.
///
/// ```
/// # egui::__run_test_ui(|ui| {
/// let response = ui.button("Open popup");
/// let popup_id = ui.make_persistent_id("my_unique_id");
/// if response.clicked() {
///     ui.memory().toggle_popup(popup_id);
/// }
/// egui::popup::popup_below_widget(ui, popup_id, &response, |ui| {
///     ui.set_min_width(200.0); // if you want to control the size
///     ui.label("Some more info, or things you can select:");
///     ui.label("…");
/// });
/// # });
/// ```
pub fn popup_below_widget<R>(
    ui: &Ui,
    popup_id: Id,
    widget_response: &Response,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> Option<R> {
    if ui.memory().is_popup_open(popup_id) {
        let parent_clip_rect = ui.clip_rect();

        let inner = Area::new(popup_id)
            .order(Order::Foreground)
            .fixed_pos(widget_response.rect.left_bottom())
            .show(ui.ctx(), |ui| {
                ui.set_clip_rect(parent_clip_rect); // for when the combo-box is in a scroll area.
                let frame = Frame::popup(ui.style());
                let frame_margin = frame.margin;
                frame
                    .show(ui, |ui| {
                        ui.with_layout(Layout::top_down_justified(Align::LEFT), |ui| {
                            ui.set_width(widget_response.rect.width() - 2.0 * frame_margin.x);
                            add_contents(ui)
                        })
                        .inner
                    })
                    .inner
            })
            .inner;

        if ui.input().key_pressed(Key::Escape) || widget_response.clicked_elsewhere() {
            ui.memory().close_popup();
        }
        Some(inner)
    } else {
        None
    }
}
