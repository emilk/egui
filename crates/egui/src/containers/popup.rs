//! Show popup windows, tooltips, context menus etc.

use crate::*;

// ----------------------------------------------------------------------------

/// Same state for all tooltips.
#[derive(Clone, Debug, Default)]
pub(crate) struct TooltipState {
    last_common_id: Option<Id>,
    individual_ids_and_sizes: ahash::HashMap<usize, (Id, Vec2)>,
}

impl TooltipState {
    pub fn load(ctx: &Context) -> Option<Self> {
        ctx.data_mut(|d| d.get_temp(Id::null()))
    }

    fn store(self, ctx: &Context) {
        ctx.data_mut(|d| d.insert_temp(Id::null(), self));
    }

    fn individual_tooltip_size(&self, common_id: Id, index: usize) -> Option<Vec2> {
        if self.last_common_id == Some(common_id) {
            Some(self.individual_ids_and_sizes.get(&index)?.1)
        } else {
            None
        }
    }

    fn set_individual_tooltip(
        &mut self,
        common_id: Id,
        index: usize,
        individual_id: Id,
        size: Vec2,
    ) {
        if self.last_common_id != Some(common_id) {
            self.last_common_id = Some(common_id);
            self.individual_ids_and_sizes.clear();
        }

        self.individual_ids_and_sizes
            .insert(index, (individual_id, size));
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
pub fn show_tooltip<R>(
    ctx: &Context,
    id: Id,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> Option<R> {
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
    ctx: &Context,
    id: Id,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> Option<R> {
    let suggested_pos = ctx
        .input(|i| i.pointer.hover_pos())
        .map(|pointer_pos| pointer_pos + vec2(16.0, 16.0));
    show_tooltip_at(ctx, id, suggested_pos, add_contents)
}

/// Show a tooltip under the given area.
///
/// If the tooltip does not fit under the area, it tries to place it above it instead.
pub fn show_tooltip_for<R>(
    ctx: &Context,
    id: Id,
    rect: &Rect,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> Option<R> {
    let expanded_rect = rect.expand2(vec2(2.0, 4.0));
    let (above, position) = if ctx.input(|i| i.any_touches()) {
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
    ctx: &Context,
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
    ctx: &Context,
    individual_id: Id,
    suggested_position: Option<Pos2>,
    above: bool,
    mut avoid_rect: Rect,
    add_contents: Box<dyn FnOnce(&mut Ui) -> R + 'c>,
) -> Option<R> {
    let spacing = 4.0;

    // if there are multiple tooltips open they should use the same common_id for the `tooltip_size` caching to work.
    let mut frame_state =
        ctx.frame_state(|fs| fs.tooltip_state)
            .unwrap_or(crate::frame_state::TooltipFrameState {
                common_id: individual_id,
                rect: Rect::NOTHING,
                count: 0,
            });

    let mut position = if frame_state.rect.is_positive() {
        avoid_rect = avoid_rect.union(frame_state.rect);
        if above {
            frame_state.rect.left_top() - spacing * Vec2::Y
        } else {
            frame_state.rect.left_bottom() + spacing * Vec2::Y
        }
    } else if let Some(position) = suggested_position {
        position
    } else if ctx.memory(|mem| mem.everything_is_visible()) {
        Pos2::ZERO
    } else {
        return None; // No good place for a tooltip :(
    };

    let mut long_state = TooltipState::load(ctx).unwrap_or_default();
    let expected_size =
        long_state.individual_tooltip_size(frame_state.common_id, frame_state.count);
    let expected_size = expected_size.unwrap_or_else(|| vec2(64.0, 32.0));

    if above {
        position.y -= expected_size.y;
    }

    position = position.at_most(ctx.screen_rect().max - expected_size);

    // check if we intersect the avoid_rect
    {
        let new_rect = Rect::from_min_size(position, expected_size);

        // Note: We use shrink so that we don't get false positives when the rects just touch
        if new_rect.shrink(1.0).intersects(avoid_rect) {
            if above {
                // place below instead:
                position = avoid_rect.left_bottom() + spacing * Vec2::Y;
            } else {
                // place above instead:
                position = Pos2::new(position.x, avoid_rect.min.y - expected_size.y - spacing);
            }
        }
    }

    let position = position.at_least(ctx.screen_rect().min);

    let area_id = frame_state.common_id.with(frame_state.count);

    let InnerResponse { inner, response } =
        show_tooltip_area_dyn(ctx, area_id, position, add_contents);

    long_state.set_individual_tooltip(
        frame_state.common_id,
        frame_state.count,
        individual_id,
        response.rect.size(),
    );
    long_state.store(ctx);

    frame_state.count += 1;
    frame_state.rect = frame_state.rect.union(response.rect);
    ctx.frame_state_mut(|fs| fs.tooltip_state = Some(frame_state));

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
pub fn show_tooltip_text(ctx: &Context, id: Id, text: impl Into<WidgetText>) -> Option<()> {
    show_tooltip(ctx, id, |ui| {
        crate::widgets::Label::new(text).ui(ui);
    })
}

/// Show a pop-over window.
fn show_tooltip_area_dyn<'c, R>(
    ctx: &Context,
    area_id: Id,
    window_pos: Pos2,
    add_contents: Box<dyn FnOnce(&mut Ui) -> R + 'c>,
) -> InnerResponse<R> {
    use containers::*;
    Area::new(area_id)
        .order(Order::Tooltip)
        .fixed_pos(window_pos)
        .constrain(true)
        .interactable(false)
        .drag_bounds(ctx.screen_rect())
        .show(ctx, |ui| {
            Frame::popup(&ctx.style())
                .show(ui, |ui| {
                    ui.set_max_width(ui.spacing().tooltip_width);
                    add_contents(ui)
                })
                .inner
        })
}

/// Was this popup visible last frame?
pub fn was_tooltip_open_last_frame(ctx: &Context, tooltip_id: Id) -> bool {
    if let Some(state) = TooltipState::load(ctx) {
        if let Some(common_id) = state.last_common_id {
            for (count, (individual_id, _size)) in &state.individual_ids_and_sizes {
                if *individual_id == tooltip_id {
                    let area_id = common_id.with(count);
                    let layer_id = LayerId::new(Order::Tooltip, area_id);
                    if ctx.memory(|mem| mem.areas.visible_last_frame(&layer_id)) {
                        return true;
                    }
                }
            }
        }
    }

    false
}

/// Helper for [`popup_above_or_below_widget`].
pub fn popup_below_widget<R>(
    ui: &Ui,
    popup_id: Id,
    widget_response: &Response,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> Option<R> {
    popup_above_or_below_widget(
        ui,
        popup_id,
        widget_response,
        AboveOrBelow::Below,
        add_contents,
    )
}

/// Shows a popup above or below another widget.
///
/// Useful for drop-down menus (combo boxes) or suggestion menus under text fields.
///
/// The opened popup will have the same width as the parent.
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
///     ui.memory_mut(|mem| mem.toggle_popup(popup_id));
/// }
/// let below = egui::AboveOrBelow::Below;
/// egui::popup::popup_above_or_below_widget(ui, popup_id, &response, below, |ui| {
///     ui.set_min_width(200.0); // if you want to control the size
///     ui.label("Some more info, or things you can select:");
///     ui.label("…");
/// });
/// # });
/// ```
pub fn popup_above_or_below_widget<R>(
    ui: &Ui,
    popup_id: Id,
    widget_response: &Response,
    above_or_below: AboveOrBelow,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> Option<R> {
    if ui.memory(|mem| mem.is_popup_open(popup_id)) {
        let (pos, pivot) = match above_or_below {
            AboveOrBelow::Above => (widget_response.rect.left_top(), Align2::LEFT_BOTTOM),
            AboveOrBelow::Below => (widget_response.rect.left_bottom(), Align2::LEFT_TOP),
        };

        let inner = Area::new(popup_id)
            .order(Order::Foreground)
            .constrain(true)
            .fixed_pos(pos)
            .pivot(pivot)
            .show(ui.ctx(), |ui| {
                // Note: we use a separate clip-rect for this area, so the popup can be outside the parent.
                // See https://github.com/emilk/egui/issues/825
                let frame = Frame::popup(ui.style());
                let frame_margin = frame.total_margin();
                frame
                    .show(ui, |ui| {
                        ui.with_layout(Layout::top_down_justified(Align::LEFT), |ui| {
                            ui.set_width(widget_response.rect.width() - frame_margin.sum().x);
                            add_contents(ui)
                        })
                        .inner
                    })
                    .inner
            })
            .inner;

        if ui.input(|i| i.key_pressed(Key::Escape)) || widget_response.clicked_elsewhere() {
            ui.memory_mut(|mem| mem.close_popup());
        }
        Some(inner)
    } else {
        None
    }
}
