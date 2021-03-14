//! Show popup windows, tooltips, context menus etc.

use crate::*;

// ----------------------------------------------------------------------------

/// Same state for all tooltips.
#[derive(Clone, Debug, Default)]
pub(crate) struct MonoState {
    last_id: Option<Id>,
    last_size: Option<Vec2>,
}

impl MonoState {
    fn tooltip_size(&self, id: Id) -> Option<Vec2> {
        if self.last_id == Some(id) {
            self.last_size
        } else {
            None
        }
    }

    fn set_tooltip_size(&mut self, id: Id, size: Vec2) {
        if self.last_id == Some(id) {
            if let Some(stored_size) = &mut self.last_size {
                *stored_size = stored_size.max(size);
                return;
            }
        }

        self.last_id = Some(id);
        self.last_size = Some(size);
    }
}

// ----------------------------------------------------------------------------

/// Show a tooltip at the current pointer position (if any).
///
/// Most of the time it is easier to use [`Response::on_hover_ui`].
///
/// See also [`show_tooltip_text`].
///
/// ```
/// # let mut ui = egui::Ui::__test();
/// if ui.ui_contains_pointer() {
///     egui::show_tooltip(ui.ctx(), egui::Id::new("my_tooltip"), |ui| {
///         ui.label("Helpful text");
///     });
/// }
/// ```
pub fn show_tooltip(ctx: &CtxRef, id: Id, add_contents: impl FnOnce(&mut Ui)) {
    show_tooltip_at_pointer(ctx, id, add_contents)
}

pub fn show_tooltip_at_pointer(ctx: &CtxRef, id: Id, add_contents: impl FnOnce(&mut Ui)) {
    let suggested_pos = ctx
        .input()
        .pointer
        .hover_pos()
        .map(|pointer_pos| pointer_pos + vec2(16.0, 16.0));
    show_tooltip_at(ctx, id, suggested_pos, add_contents)
}

pub fn show_tooltip_under(ctx: &CtxRef, id: Id, rect: &Rect, add_contents: impl FnOnce(&mut Ui)) {
    show_tooltip_at(
        ctx,
        id,
        Some(rect.left_bottom() + vec2(-2.0, 4.0)),
        add_contents,
    )
}

pub fn show_tooltip_at(
    ctx: &CtxRef,
    mut id: Id,
    suggested_position: Option<Pos2>,
    add_contents: impl FnOnce(&mut Ui),
) {
    let mut tooltip_rect = Rect::NOTHING;

    let position = if let Some((stored_id, stored_tooltip_rect)) = ctx.frame_state().tooltip_rect {
        // if there are multiple tooltips open they should use the same id for the `tooltip_size` caching to work.
        id = stored_id;
        tooltip_rect = stored_tooltip_rect;
        tooltip_rect.left_bottom()
    } else if let Some(position) = suggested_position {
        position
    } else if ctx.memory().everything_is_visible() {
        Pos2::default()
    } else {
        return; // No good place for a tooltip :(
    };

    let expected_size = ctx.memory().tooltip.tooltip_size(id);
    let expected_size = expected_size.unwrap_or_else(|| vec2(64.0, 32.0));
    let position = position.min(ctx.input().screen_rect().right_bottom() - expected_size);
    let position = position.max(ctx.input().screen_rect().left_top());

    let response = show_tooltip_area(ctx, id, position, add_contents);
    ctx.memory()
        .tooltip
        .set_tooltip_size(id, response.rect.size());

    ctx.frame_state().tooltip_rect = Some((id, tooltip_rect.union(response.rect)));
}

/// Show some text at the current pointer position (if any).
///
/// Most of the time it is easier to use [`Response::on_hover_text`].
///
/// See also [`show_tooltip`].
///
/// ```
/// # let mut ui = egui::Ui::__test();
/// if ui.ui_contains_pointer() {
///     egui::show_tooltip_text(ui.ctx(), egui::Id::new("my_tooltip"), "Helpful text");
/// }
/// ```
pub fn show_tooltip_text(ctx: &CtxRef, id: Id, text: impl Into<String>) {
    show_tooltip(ctx, id, |ui| {
        ui.add(crate::widgets::Label::new(text));
    })
}

/// Show a pop-over window.
fn show_tooltip_area(
    ctx: &CtxRef,
    id: Id,
    window_pos: Pos2,
    add_contents: impl FnOnce(&mut Ui),
) -> Response {
    use containers::*;
    Area::new(id)
        .order(Order::Tooltip)
        .fixed_pos(window_pos)
        .interactable(false)
        .show(ctx, |ui| {
            Frame::popup(&ctx.style()).show(ui, |ui| {
                ui.set_max_width(ui.spacing().tooltip_width);
                add_contents(ui);
            })
        })
}

/// Shows a popup below another widget.
///
/// Useful for drop-down menus (combo boxes) or suggestion menus under text fields.
///
/// You must open the popup with [`Memory::open_popup`] or  [`Memory::toggle_popup`].
///
/// ```
/// # let ui = &mut egui::Ui::__test();
/// let response = ui.button("Open popup");
/// let popup_id = ui.make_persistent_id("my_unique_id");
/// if response.clicked() {
///     ui.memory().toggle_popup(popup_id);
/// }
/// egui::popup::popup_below_widget(ui, popup_id, &response, |ui| {
///     ui.label("Some more info, or things you can select:");
///     ui.label("…");
/// });
/// ```
pub fn popup_below_widget(
    ui: &Ui,
    popup_id: Id,
    widget_response: &Response,
    add_contents: impl FnOnce(&mut Ui),
) {
    if ui.memory().is_popup_open(popup_id) {
        let parent_clip_rect = ui.clip_rect();

        Area::new(popup_id)
            .order(Order::Foreground)
            .fixed_pos(widget_response.rect.left_bottom())
            .show(ui.ctx(), |ui| {
                ui.set_clip_rect(parent_clip_rect); // for when the combo-box is in a scroll area.
                let frame = Frame::popup(ui.style());
                let frame_margin = frame.margin;
                frame.show(ui, |ui| {
                    ui.with_layout(Layout::top_down_justified(Align::LEFT), |ui| {
                        ui.set_width(widget_response.rect.width() - 2.0 * frame_margin.x);
                        add_contents(ui)
                    });
                });
            });

        if ui.input().key_pressed(Key::Escape) || widget_response.clicked_elsewhere() {
            ui.memory().close_popup();
        }
    }
}
