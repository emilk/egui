use crate::*;

/// Show a tooltip at the current mouse position (if any).
///
/// Most of the time it is easier to use [`Response::on_hover_ui`].
///
/// See also [`show_tooltip_text`].
///
/// ```
/// # let mut ui = egui::Ui::__test();
/// if ui.ui_contains_mouse() {
///     egui::show_tooltip(ui.ctx(), |ui| {
///         ui.label("Helpful text");
///     });
/// }
/// ```
pub fn show_tooltip(ctx: &CtxRef, add_contents: impl FnOnce(&mut Ui)) {
    let tooltip_rect = ctx.frame_state().tooltip_rect;

    let window_pos = if let Some(tooltip_rect) = tooltip_rect {
        tooltip_rect.left_bottom()
    } else if let Some(mouse_pos) = ctx.input().mouse.pos {
        let expected_size = vec2(ctx.style().spacing.tooltip_width, 32.0);
        let position = mouse_pos + vec2(16.0, 16.0);
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
    let response = show_popup(ctx, id, window_pos, add_contents);

    let tooltip_rect = tooltip_rect.unwrap_or_else(Rect::nothing);
    ctx.frame_state().tooltip_rect = Some(tooltip_rect.union(response.rect));
}

/// Show some text at the current mouse position (if any).
///
/// Most of the time it is easier to use [`Response::on_hover_text`].
///
/// See also [`show_tooltip`].
///
/// ```
/// # let mut ui = egui::Ui::__test();
/// if ui.ui_contains_mouse() {
///     egui::show_tooltip_text(ui.ctx(), "Helpful text");
/// }
/// ```
pub fn show_tooltip_text(ctx: &CtxRef, text: impl Into<String>) {
    show_tooltip(ctx, |ui| {
        ui.add(crate::widgets::Label::new(text));
    })
}

/// Show a pop-over window.
fn show_popup(
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
                ui.set_max_width(ui.style().spacing.tooltip_width);
                add_contents(ui);
            })
        })
}
