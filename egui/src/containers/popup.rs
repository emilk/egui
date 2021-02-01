//! Show popup windows, tooltips, context menus etc.

use crate::*;

/// Show a tooltip at the current pointer position (if any).
///
/// Most of the time it is easier to use [`Response::on_hover_ui`].
///
/// See also [`show_tooltip_text`].
///
/// ```
/// # let mut ui = egui::Ui::__test();
/// if ui.ui_contains_pointer() {
///     egui::show_tooltip(ui.ctx(), |ui| {
///         ui.label("Helpful text");
///     });
/// }
/// ```
pub fn show_tooltip(ctx: &CtxRef, add_contents: impl FnOnce(&mut Ui)) {
    let tooltip_rect = ctx.frame_state().tooltip_rect;

    let window_pos = if let Some(tooltip_rect) = tooltip_rect {
        tooltip_rect.left_bottom()
    } else if let Some(pointer_pos) = ctx.input().pointer.tooltip_pos() {
        let expected_size = vec2(ctx.style().spacing.tooltip_width, 32.0);
        let position = pointer_pos + vec2(16.0, 16.0);
        let position = position.min(ctx.input().screen_rect().right_bottom() - expected_size);
        let position = position.max(ctx.input().screen_rect().left_top());
        position
    } else if ctx.memory().everything_is_visible() {
        Pos2::default()
    } else {
        return; // No good place for a tooltip :(
    };

    //  TODO: default size
    let id = Id::tooltip();
    let response = show_tooltip_area(ctx, id, window_pos, add_contents);

    let tooltip_rect = tooltip_rect.unwrap_or_else(Rect::nothing);
    ctx.frame_state().tooltip_rect = Some(tooltip_rect.union(response.rect));
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
///     egui::show_tooltip_text(ui.ctx(), "Helpful text");
/// }
/// ```
pub fn show_tooltip_text(ctx: &CtxRef, text: impl Into<String>) {
    show_tooltip(ctx, |ui| {
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
                    ui.with_layout(Layout::top_down_justified(Align::left()), |ui| {
                        ui.set_width(widget_response.rect.width() - 2.0 * frame_margin.x);
                        add_contents(ui)
                    });
                });
            });

        if ui.input().key_pressed(Key::Escape)
            || ui.input().pointer.any_click() && !widget_response.clicked()
        {
            ui.memory().close_popup();
        }
    }
}
