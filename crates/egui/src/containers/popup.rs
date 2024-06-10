//! Show popup windows, tooltips, context menus etc.

use frame_state::PerWidgetTooltipState;

use crate::*;

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
    widget_id: Id,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> Option<R> {
    show_tooltip_at_pointer(ctx, widget_id, add_contents)
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
    widget_id: Id,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> Option<R> {
    ctx.input(|i| i.pointer.hover_pos()).map(|pointer_pos| {
        show_tooltip_at(ctx, widget_id, pointer_pos + vec2(16.0, 16.0), add_contents)
    })
}

/// Show a tooltip under the given area.
///
/// If the tooltip does not fit under the area, it tries to place it above it instead.
pub fn show_tooltip_for<R>(
    ctx: &Context,
    widget_id: Id,
    widget_rect: &Rect,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> R {
    let is_touch_screen = ctx.input(|i| i.any_touches());
    let allow_placing_below = !is_touch_screen; // There is a finger below.
    show_tooltip_at_avoid_dyn(
        ctx,
        widget_id,
        allow_placing_below,
        widget_rect,
        Box::new(add_contents),
    )
}

/// Show a tooltip at the given position.
///
/// Returns `None` if the tooltip could not be placed.
pub fn show_tooltip_at<R>(
    ctx: &Context,
    widget_id: Id,
    suggested_position: Pos2,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> R {
    let allow_placing_below = true;
    let rect = Rect::from_center_size(suggested_position, Vec2::ZERO);
    show_tooltip_at_avoid_dyn(
        ctx,
        widget_id,
        allow_placing_below,
        &rect,
        Box::new(add_contents),
    )
}

fn show_tooltip_at_avoid_dyn<'c, R>(
    ctx: &Context,
    widget_id: Id,
    allow_placing_below: bool,
    widget_rect: &Rect,
    add_contents: Box<dyn FnOnce(&mut Ui) -> R + 'c>,
) -> R {
    // if there are multiple tooltips open they should use the same common_id for the `tooltip_size` caching to work.
    let mut state = ctx.frame_state(|fs| {
        fs.tooltip_state
            .widget_tooltips
            .get(&widget_id)
            .copied()
            .unwrap_or(PerWidgetTooltipState {
                bounding_rect: *widget_rect,
                tooltip_count: 0,
            })
    });

    let tooltip_area_id = tooltip_id(widget_id, state.tooltip_count);
    let expected_tooltip_size =
        AreaState::load(ctx, tooltip_area_id).map_or(vec2(64.0, 32.0), |area| area.size);

    let screen_rect = ctx.screen_rect();

    let (pivot, anchor) = find_tooltip_position(
        screen_rect,
        state.bounding_rect,
        allow_placing_below,
        expected_tooltip_size,
    );

    let InnerResponse { inner, response } = Area::new(tooltip_area_id)
        .kind(UiKind::Popup)
        .order(Order::Tooltip)
        .pivot(pivot)
        .fixed_pos(anchor)
        .default_width(ctx.style().spacing.tooltip_width)
        .sense(Sense::hover()) // don't click to bring to front
        .show(ctx, |ui| {
            // By default the text in tooltips aren't selectable.
            // This means that most tooltips aren't interactable,
            // which also mean they won't stick around so you can click them.
            // Only tooltips that have actual interactive stuff (buttons, links, …)
            // will stick around when you try to click them.
            ui.style_mut().interaction.selectable_labels = false;

            Frame::popup(&ctx.style()).show_dyn(ui, add_contents).inner
        });

    state.tooltip_count += 1;
    state.bounding_rect = state.bounding_rect.union(response.rect);
    ctx.frame_state_mut(|fs| fs.tooltip_state.widget_tooltips.insert(widget_id, state));

    inner
}

/// What is the id of the next tooltip for this widget?
pub fn next_tooltip_id(ctx: &Context, widget_id: Id) -> Id {
    let tooltip_count = ctx.frame_state(|fs| {
        fs.tooltip_state
            .widget_tooltips
            .get(&widget_id)
            .map_or(0, |state| state.tooltip_count)
    });
    tooltip_id(widget_id, tooltip_count)
}

pub fn tooltip_id(widget_id: Id, tooltip_count: usize) -> Id {
    widget_id.with(tooltip_count)
}

/// Returns `(PIVOT, POS)` to mean: put the `PIVOT` corner of the tooltip at `POS`.
///
/// Note: the position might need to be constrained to the screen,
/// (e.g. moved sideways if shown under the widget)
/// but the `Area` will take care of that.
fn find_tooltip_position(
    screen_rect: Rect,
    widget_rect: Rect,
    allow_placing_below: bool,
    tooltip_size: Vec2,
) -> (Align2, Pos2) {
    let spacing = 4.0;

    // Does it fit below?
    if allow_placing_below
        && widget_rect.bottom() + spacing + tooltip_size.y <= screen_rect.bottom()
    {
        return (
            Align2::LEFT_TOP,
            widget_rect.left_bottom() + spacing * Vec2::DOWN,
        );
    }

    // Does it fit above?
    if screen_rect.top() + tooltip_size.y + spacing <= widget_rect.top() {
        return (
            Align2::LEFT_BOTTOM,
            widget_rect.left_top() + spacing * Vec2::UP,
        );
    }

    // Does it fit to the right?
    if widget_rect.right() + spacing + tooltip_size.x <= screen_rect.right() {
        return (
            Align2::LEFT_TOP,
            widget_rect.right_top() + spacing * Vec2::RIGHT,
        );
    }

    // Does it fit to the left?
    if screen_rect.left() + tooltip_size.x + spacing <= widget_rect.left() {
        return (
            Align2::RIGHT_TOP,
            widget_rect.left_top() + spacing * Vec2::LEFT,
        );
    }

    // It doesn't fit anywhere :(

    // Just show it anyway:
    (Align2::LEFT_TOP, screen_rect.left_top())
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
pub fn show_tooltip_text(ctx: &Context, widget_id: Id, text: impl Into<WidgetText>) -> Option<()> {
    show_tooltip(ctx, widget_id, |ui| {
        crate::widgets::Label::new(text).ui(ui);
    })
}

/// Was this popup visible last frame?
pub fn was_tooltip_open_last_frame(ctx: &Context, widget_id: Id) -> bool {
    let primary_tooltip_area_id = tooltip_id(widget_id, 0);
    ctx.memory(|mem| {
        mem.areas()
            .visible_last_frame(&LayerId::new(Order::Tooltip, primary_tooltip_area_id))
    })
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
/// The opened popup will have a minimum width matching its parent.
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
    parent_ui: &Ui,
    popup_id: Id,
    widget_response: &Response,
    above_or_below: AboveOrBelow,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> Option<R> {
    if parent_ui.memory(|mem| mem.is_popup_open(popup_id)) {
        let (pos, pivot) = match above_or_below {
            AboveOrBelow::Above => (widget_response.rect.left_top(), Align2::LEFT_BOTTOM),
            AboveOrBelow::Below => (widget_response.rect.left_bottom(), Align2::LEFT_TOP),
        };

        let frame = Frame::popup(parent_ui.style());
        let frame_margin = frame.total_margin();
        let inner_width = widget_response.rect.width() - frame_margin.sum().x;

        let inner = Area::new(popup_id)
            .kind(UiKind::Popup)
            .order(Order::Foreground)
            .fixed_pos(pos)
            .default_width(inner_width)
            .pivot(pivot)
            .show(parent_ui.ctx(), |ui| {
                frame
                    .show(ui, |ui| {
                        ui.with_layout(Layout::top_down_justified(Align::LEFT), |ui| {
                            ui.set_min_width(inner_width);
                            add_contents(ui)
                        })
                        .inner
                    })
                    .inner
            })
            .inner;

        if parent_ui.input(|i| i.key_pressed(Key::Escape)) || widget_response.clicked_elsewhere() {
            parent_ui.memory_mut(|mem| mem.close_popup());
        }
        Some(inner)
    } else {
        None
    }
}
